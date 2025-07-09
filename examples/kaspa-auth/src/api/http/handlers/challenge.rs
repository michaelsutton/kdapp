// src/api/http/handlers/challenge.rs
use axum::{extract::State, response::Json, http::StatusCode};
use kaspa_addresses::{Address, Prefix, Version};
use kaspa_consensus_core::tx::{TransactionOutpoint, UtxoEntry};
use kaspa_wrpc_client::prelude::RpcApi;
use kdapp::{
    engine::EpisodeMessage,
    pki::PubKey,
    generator::TransactionGenerator,
};
use crate::api::http::{
    types::{ChallengeRequest, ChallengeResponse},
    state::PeerState,
};
use crate::core::{episode::SimpleAuth, commands::AuthCommand};

pub async fn request_challenge(
    State(state): State<PeerState>,
    Json(req): Json<ChallengeRequest>,
) -> Result<Json<ChallengeResponse>, StatusCode> {
    println!("📨 Sending RequestChallenge command to blockchain...");
    
    // Parse the participant's public key (like CLI does)
    let participant_pubkey = match hex::decode(&req.public_key) {
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
    
    // 🎯 TRUE P2P: Participant funds their own transactions (like CLI)
    let participant_wallet = crate::wallet::get_wallet_for_command("web-participant", None)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let participant_secret_key = participant_wallet.keypair.secret_key();
    
    // Create participant's Kaspa address for transaction funding (True P2P!)
    let participant_addr = Address::new(
        Prefix::Testnet, 
        Version::PubKey, 
        &participant_wallet.keypair.x_only_public_key().0.serialize()
    );
    
    // 🚨 CRITICAL: Create participant's transaction generator for proper signing
    let participant_generator = TransactionGenerator::new(
        participant_wallet.keypair,
        crate::episode_runner::AUTH_PATTERN,
        crate::episode_runner::AUTH_PREFIX,
    );
    
    // Get REAL UTXOs from blockchain (exactly like CLI)
    // Wait for previous transaction to confirm before fetching new UTXOs
    let utxo = if let Some(ref kaspad) = state.kaspad_client {
        println!("🔍 Fetching UTXOs for RequestChallenge transaction...");
        
        // Wait a bit for the previous transaction to confirm
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
        
        let entries = match kaspad.get_utxos_by_addresses(vec![participant_addr.clone()]).await {
            Ok(entries) => entries,
            Err(e) => {
                println!("❌ Failed to fetch UTXOs: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };
        
        if entries.is_empty() {
            println!("❌ No UTXOs found! Participant wallet needs funding.");
            println!("💰 Fund this address: {}", participant_addr);
            println!("🚰 Get testnet funds: https://faucet.kaspanet.io/");
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
    
    // Create RequestChallenge command signed by PARTICIPANT (exactly like CLI)
    let auth_command = AuthCommand::RequestChallenge;
    let step = EpisodeMessage::<SimpleAuth>::new_signed_command(
        req.episode_id.try_into().unwrap(), 
        auth_command, 
        participant_secret_key, // 🚨 CRITICAL: Participant signs their own commands!
        participant_pubkey
    );
    
    // Build and submit transaction to blockchain (exactly like CLI)
    let tx = participant_generator.build_command_transaction(utxo, &participant_addr, &step, 5000);
    println!("🚀 Submitting RequestChallenge transaction: {}", tx.id());
    
    let _submission_result = match state.kaspad_client.as_ref().unwrap().submit_transaction(tx.as_ref().into(), false).await {
        Ok(_response) => {
            println!("✅ RequestChallenge transaction submitted to blockchain!");
            println!("⏳ Organizer peer will generate challenge and update episode on blockchain");
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