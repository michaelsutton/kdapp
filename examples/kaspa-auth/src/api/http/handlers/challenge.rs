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
    
    // Submit transaction to blockchain via AuthHttpPeer (centralized submission)
    println!("📤 Submitting RequestChallenge transaction to Kaspa blockchain via AuthHttpPeer...");
    let submission_result = match state.auth_http_peer.as_ref().unwrap().submit_episode_message_transaction(
        step,
        participant_wallet.keypair,
        participant_addr.clone(),
        utxo,
    ).await {
        Ok(tx_id) => {
            println!("✅ RequestChallenge transaction {} submitted successfully to blockchain via AuthHttpPeer!", tx_id);
            println!("⏳ Organizer peer will generate challenge and update episode on blockchain");
            (tx_id, "request_challenge_submitted".to_string())
        }
        Err(e) => {
            println!("❌ RequestChallenge submission failed via AuthHttpPeer: {}", e);
            ("error".to_string(), "request_challenge_failed".to_string())
        }
    };
    
    let (transaction_id, status) = submission_result;

    // Wait for blockchain to process RequestChallenge and generate challenge
    let mut challenge_nonce = String::new();
    let mut attempts = 0;
    let max_attempts = 150; // 30 second timeout (150 attempts * 200ms)
    
    while challenge_nonce.is_empty() && attempts < max_attempts {
        if let Some(episode) = state.blockchain_episodes.lock().unwrap().get(&req.episode_id.try_into().unwrap()) {
            if let Some(challenge) = &episode.challenge {
                challenge_nonce = challenge.clone();
                println!("✅ Challenge generated by blockchain: {}", challenge_nonce);
                break;
            }
        }
        
        attempts += 1;
        if attempts % 10 == 0 {
            println!("⏳ Waiting for blockchain to generate challenge... attempt {}/{}", attempts, max_attempts);
        }
        
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
    
    if challenge_nonce.is_empty() {
        println!("❌ Timeout waiting for blockchain challenge generation");
        return Err(StatusCode::REQUEST_TIMEOUT);
    }

    Ok(Json(ChallengeResponse {
        episode_id: req.episode_id,
        nonce: challenge_nonce,
        transaction_id: Some(transaction_id),
        status: status,
    }))
}