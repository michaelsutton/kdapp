// src/api/http/handlers/auth.rs
use axum::{extract::State, response::Json, http::StatusCode};
use kaspa_addresses::{Address, Prefix, Version};
use kaspa_consensus_core::tx::{TransactionOutpoint, UtxoEntry};
use kaspa_wrpc_client::prelude::RpcApi;
use kdapp::{
    engine::EpisodeMessage,
    generator,
    pki::PubKey,
};
use rand::Rng;

use crate::api::http::{
    types::{AuthRequest, AuthResponse},
    state::ServerState,
};
use crate::core::episode::SimpleAuth;

pub async fn start_auth(
    State(state): State<ServerState>,
    Json(req): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    println!("🚀 Submitting REAL NewEpisode transaction to Kaspa blockchain...");
    
    // Parse the client's public key
    println!("📋 Received public key: {}", &req.public_key);
    let client_pubkey = match hex::decode(&req.public_key) {
        Ok(bytes) => {
            println!("✅ Hex decode successful, {} bytes", bytes.len());
            match secp256k1::PublicKey::from_slice(&bytes) {
                Ok(pk) => {
                    println!("✅ Public key parsing successful");
                    PubKey(pk)
                },
                Err(e) => {
                    println!("❌ Public key parsing failed: {}", e);
                    return Err(StatusCode::BAD_REQUEST);
                },
            }
        },
        Err(e) => {
            println!("❌ Hex decode failed: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        },
    };
    
    // Generate episode ID
    let episode_id = rand::thread_rng().gen();
    
    // Create client Kaspa address for transaction funding (like CLI does)
    let client_addr = Address::new(
        Prefix::Testnet, 
        Version::PubKey, 
        &client_pubkey.0.x_only_public_key().0.serialize()
    );
    
    // Create server Kaspa address for transaction funding (server funds, client participates)
    let server_addr = Address::new(
        Prefix::Testnet, 
        Version::PubKey, 
        &state.server_keypair.x_only_public_key().0.serialize()
    );
    
    // Create NewEpisode message for blockchain
    let new_episode = EpisodeMessage::<SimpleAuth>::NewEpisode { 
        episode_id, 
        participants: vec![client_pubkey] 
    };
    
    // Get REAL UTXOs from blockchain (like CLI does)
    let utxo = if let Some(ref kaspad) = state.kaspad_client {
        println!("🔍 Fetching UTXOs for server address...");
        let entries = match kaspad.get_utxos_by_addresses(vec![server_addr.clone()]).await {
            Ok(entries) => entries,
            Err(e) => {
                println!("❌ Failed to fetch UTXOs: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };
        
        if entries.is_empty() {
            println!("❌ No UTXOs found! Server wallet needs funding.");
            println!("💰 Fund this address: {}", server_addr);
            println!("🚰 Get testnet funds: https://faucet.kaspanet.io/");
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        
        let utxo = entries.first().map(|entry| {
            (TransactionOutpoint::from(entry.outpoint.clone()), UtxoEntry::from(entry.utxo_entry.clone()))
        }).unwrap();
        
        println!("✅ UTXO found: {}", utxo.0);
        utxo
    } else {
        println!("❌ No kaspad client available");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };
    
    // Build the blockchain transaction
    println!("🔨 Building transaction...");
    let tx = match std::panic::catch_unwind(|| {
        state.transaction_generator.build_command_transaction(
            utxo, 
            &server_addr, 
            &new_episode, 
            5000
        )
    }) {
        Ok(tx) => tx,
        Err(_) => {
            println!("❌ Transaction building failed (panicked)");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    
    let transaction_id = tx.id().to_string();
    println!("📋 Created transaction: {}", transaction_id);
    println!("🎯 Episode ID: {}", episode_id);
    println!("👤 Client PubKey: {}", client_pubkey);
    
    // ✅ Submit transaction to blockchain (exactly like CLI)
    println!("📤 Submitting transaction to Kaspa blockchain...");
    let submission_result = match state.kaspad_client.as_ref().unwrap().submit_transaction(tx.as_ref().into(), false).await {
        Ok(_response) => {
            println!("✅ Transaction submitted successfully to blockchain!");
            println!("🎬 Episode {} initialized on blockchain", episode_id);
            "submitted_to_blockchain"
        }
        Err(e) => {
            println!("❌ Transaction submission failed: {}", e);
            println!("💡 Make sure server wallet is funded: {}", server_addr);
            "transaction_submission_failed"
        }
    };
    
    Ok(Json(AuthResponse {
        episode_id: episode_id.into(),
        server_public_key: hex::encode(state.server_keypair.public_key().serialize()),
        client_kaspa_address: client_addr.to_string(),
        transaction_id: Some(transaction_id),
        status: submission_result.to_string(),
    }))
}