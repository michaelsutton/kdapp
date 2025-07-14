// src/api/http/handlers/auth.rs
use axum::{extract::State, response::Json, http::StatusCode};
use kaspa_addresses::{Address, Prefix, Version};
use kaspa_consensus_core::tx::{TransactionOutpoint, UtxoEntry};
use kaspa_wrpc_client::prelude::RpcApi;
use kdapp::{
    engine::EpisodeMessage,
    pki::PubKey,
    generator::TransactionGenerator,
};
use rand::Rng;

use crate::api::http::{
    types::{AuthRequest, AuthResponse},
    state::PeerState,
};
use crate::core::episode::SimpleAuth;

pub async fn start_auth(
    State(state): State<PeerState>,
    Json(req): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    println!("🎭 MATRIX UI ACTION: User started authentication episode");
    println!("🚀 Submitting REAL NewEpisode transaction to Kaspa blockchain...");
    
    // Parse the participant's public key
    println!("📋 Received public key: {}", &req.public_key);
    let participant_pubkey = match hex::decode(&req.public_key) {
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
            println!("❌ MATRIX UI ERROR: Invalid public key format - {}", e);
            return Err(StatusCode::BAD_REQUEST);
        },
    };
    
    // Generate episode ID
    let episode_id = rand::thread_rng().gen();
    
    // Create participant Kaspa address for transaction funding (like CLI does)
    let participant_addr = Address::new(
        Prefix::Testnet, 
        Version::PubKey, 
        &participant_pubkey.0.x_only_public_key().0.serialize()
    );
    
    // 🎯 TRUE P2P: Get participant's wallet to fund their own episode creation
    let participant_wallet = crate::wallet::get_wallet_for_command("web-participant", None)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Create participant's Kaspa address for transaction funding (True P2P!)
    let participant_funding_addr = Address::new(
        Prefix::Testnet, 
        Version::PubKey, 
        &participant_wallet.keypair.x_only_public_key().0.serialize()
    );
    
    // Create NewEpisode message for blockchain
    let new_episode = EpisodeMessage::<SimpleAuth>::NewEpisode { 
        episode_id, 
        participants: vec![participant_pubkey] 
    };
    
    // Get REAL UTXOs from blockchain (like CLI does)
    let utxo = if let Some(ref kaspad) = state.kaspad_client {
        println!("🔍 Fetching UTXOs for participant address...");
        let entries = match kaspad.get_utxos_by_addresses(vec![participant_funding_addr.clone()]).await {
            Ok(entries) => entries,
            Err(e) => {
                println!("❌ Failed to fetch UTXOs: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };
        
        if entries.is_empty() {
            println!("❌ MATRIX UI ERROR: Participant wallet needs funding");
            println!("💰 Fund this address: {}", participant_funding_addr);
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
    
    println!("🎯 Episode ID: {}", episode_id);
    println!("👤 Participant PubKey: {}", participant_pubkey);
    
    // ✅ Submit transaction to blockchain via AuthHttpPeer (centralized submission)
    println!("📤 Submitting transaction to Kaspa blockchain via AuthHttpPeer...");
    let submission_result = match state.auth_http_peer.as_ref().unwrap().submit_episode_message_transaction(
        new_episode,
        participant_wallet.keypair,
        participant_funding_addr.clone(),
        utxo,
    ).await {
        Ok(tx_id) => {
            println!("✅ MATRIX UI SUCCESS: Auth episode created - Transaction {}", tx_id);
            println!("🎬 Episode {} initialized on blockchain", episode_id);
            (tx_id, "submitted_to_blockchain".to_string())
        }
        Err(e) => {
            println!("❌ MATRIX UI ERROR: Auth episode creation failed - {}", e);
            println!("💡 Make sure participant wallet is funded: {}", participant_funding_addr);
            ("error".to_string(), "transaction_submission_failed".to_string())
        }
    };
    
    let (transaction_id, status) = submission_result;
    
    Ok(Json(AuthResponse {
        episode_id: episode_id.into(),
        organizer_public_key: hex::encode(state.peer_keypair.public_key().serialize()),
        participant_kaspa_address: participant_addr.to_string(),
        transaction_id: Some(transaction_id),
        status: status,
    }))
}