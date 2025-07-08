// src/api/http/handlers/challenge.rs
use axum::{extract::State, response::Json, http::StatusCode};
use kaspa_addresses::{Address, Prefix, Version};
use kaspa_consensus_core::tx::{TransactionOutpoint, UtxoEntry};
use kaspa_wrpc_client::prelude::RpcApi;
use kdapp::{
    engine::EpisodeMessage,
    generator,
    pki::PubKey,
};
use crate::api::http::{
    types::{ChallengeRequest, ChallengeResponse},
    state::ServerState,
};
use crate::core::{episode::SimpleAuth, commands::AuthCommand};

pub async fn request_challenge(
    State(state): State<ServerState>,
    Json(req): Json<ChallengeRequest>,
) -> Result<Json<ChallengeResponse>, StatusCode> {
    println!("📨 Sending RequestChallenge command to blockchain...");
    
    // Parse the client's public key (like CLI does)
    let client_pubkey = match hex::decode(&req.public_key) {
        Ok(bytes) => {
            match secp256k1::PublicKey::from_slice(&bytes) {
                Ok(pk) => PubKey(pk),
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
    
    // 🚨 CRITICAL FIX: Get client's secret key from their wallet
    // In the CLI, client signs their own commands!
    let client_wallet = crate::wallet::get_wallet_for_command("web-client", None)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let client_secret_key = client_wallet.keypair.secret_key();
    
    // Create client Kaspa address for transaction funding (like CLI does)
    let client_addr = Address::new(
        Prefix::Testnet, 
        Version::PubKey, 
        &client_pubkey.0.x_only_public_key().0.serialize()
    );
    
    // Create server Kaspa address for transaction funding (server funds, client signs)
    let server_addr = Address::new(
        Prefix::Testnet, 
        Version::PubKey, 
        &state.server_keypair.x_only_public_key().0.serialize()
    );
    
    // Get REAL UTXOs from blockchain (exactly like CLI)
    // Wait for previous transaction to confirm before fetching new UTXOs
    let utxo = if let Some(ref kaspad) = state.kaspad_client {
        println!("🔍 Fetching UTXOs for RequestChallenge transaction...");
        
        // Wait a bit for the previous transaction to confirm
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
        
        let entries = match kaspad.get_utxos_by_addresses(vec![server_addr.clone()]).await {
            Ok(entries) => entries,
            Err(e) => {
                println!("❌ Failed to fetch UTXOs: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };
        
        if entries.is_empty() {
            println!("❌ No UTXOs found! Server wallet needs funding.");
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        
        // Try to find the newest UTXO (which should be from the previous transaction)
        let utxo = entries.first().map(|entry| {
            (TransactionOutpoint::from(entry.outpoint.clone()), UtxoEntry::from(entry.utxo_entry.clone()))
        }).unwrap();
        
        println!("✅ Using UTXO: {}", utxo.0);
        utxo
    } else {
        println!("❌ No kaspad client available");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };
    
    // Create RequestChallenge command signed by CLIENT (exactly like CLI)
    let auth_command = AuthCommand::RequestChallenge;
    let step = EpisodeMessage::<SimpleAuth>::new_signed_command(
        req.episode_id.try_into().unwrap(), 
        auth_command, 
        client_secret_key, // 🚨 CRITICAL: Client signs their own commands!
        client_pubkey
    );
    
    // Create CLIENT transaction generator (not server's!)
    let network = kaspa_consensus_core::network::NetworkId::with_suffix(kaspa_consensus_core::network::NetworkType::Testnet, 10);
    let client_generator = crate::episode_runner::create_auth_generator(client_wallet.keypair, network);
    
    // Build and submit transaction to blockchain with CLIENT'S keys
    let tx = client_generator.build_command_transaction(utxo, &client_addr, &step, 5000);
    println!("🚀 Submitting RequestChallenge transaction: {}", tx.id());
    
    let submission_result = match state.kaspad_client.as_ref().unwrap().submit_transaction(tx.as_ref().into(), false).await {
        Ok(_response) => {
            println!("✅ RequestChallenge transaction submitted to blockchain!");
            println!("⏳ Server will generate challenge and update episode on blockchain");
            "request_challenge_submitted"
        }
        Err(e) => {
            println!("❌ RequestChallenge submission failed: {}", e);
            "request_challenge_failed"
        }
    };
    
    Ok(Json(ChallengeResponse {
        episode_id: req.episode_id,
        nonce: String::new(), // Will come from blockchain when processed
    }))
}