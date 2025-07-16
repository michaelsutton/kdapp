// src/api/http/handlers/revoke.rs
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
    types::{RevokeSessionRequest, RevokeSessionResponse},
    state::PeerState,
};
use crate::core::{episode::SimpleAuth, commands::AuthCommand};

pub async fn revoke_session(
    State(state): State<PeerState>,
    Json(req): Json<RevokeSessionRequest>,
) -> Result<Json<RevokeSessionResponse>, StatusCode> {
    println!("🔄 DEBUG: RevokeSession request received - episode_id: {}, session_token: {}", req.episode_id, req.session_token);
    println!("🔍 DEBUG: Signature length: {}", req.signature.len());
    println!("📤 Sending RevokeSession command to blockchain...");
    
    // Parse episode_id from request (u64)
    let episode_id: u64 = req.episode_id;
    
    // Find the participant public key from the episode
    let episode = match state.blockchain_episodes.lock() {
        Ok(episodes) => {
            episodes.get(&episode_id).cloned()
        }
        Err(e) => {
            println!("❌ Failed to lock blockchain episodes: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    
    let (participant_pubkey, current_session_token) = match episode {
        Some(ref ep) => {
            let pubkey = ep.owner.unwrap_or_else(|| {
                println!("❌ Episode has no owner public key");
                // This shouldn't happen, but let's continue anyway
                PubKey(secp256k1::PublicKey::from_slice(&[2; 33]).unwrap())
            });
            (pubkey, ep.session_token.clone())
        },
        None => {
            println!("❌ Episode {} not found in blockchain state", episode_id);
            return Err(StatusCode::NOT_FOUND);
        }
    };
    
    // Verify that the session token matches the current episode session
    if let Some(ref current_token) = current_session_token {
        if req.session_token != *current_token {
            println!("❌ Session token mismatch");
            return Err(StatusCode::BAD_REQUEST);
        }
    } else {
        println!("❌ No active session found for episode {}", episode_id);
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // 🎯 TRUE P2P: Participant funds their own session revocation transaction
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
    let utxo = if let Some(ref kaspad) = state.kaspad_client {
        println!("🔍 Fetching UTXOs for RevokeSession transaction...");
        
        // Wait a bit for any previous transactions to confirm
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
        
        let utxo = entries.first().map(|entry| {
            (TransactionOutpoint::from(entry.outpoint.clone()), UtxoEntry::from(entry.utxo_entry.clone()))
        }).unwrap();
        
        println!("✅ Using UTXO: {}", utxo.0);
        utxo
    } else {
        println!("❌ No kaspad client available");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };
    
    // Create RevokeSession command
    let auth_command = AuthCommand::RevokeSession {
        session_token: req.session_token.clone(),
        signature: req.signature.clone(),
    };
    
    // Convert episode_id from u64 to u32 for EpisodeMessage (kdapp framework requirement)
    let episode_id_u32 = match episode_id.try_into() {
        Ok(id) => id,
        Err(_) => {
            println!("❌ Episode ID {} is too large to fit in u32", episode_id);
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    
    let step = EpisodeMessage::<SimpleAuth>::new_signed_command(
        episode_id_u32, 
        auth_command, 
        participant_secret_key, // 🚨 CRITICAL: Participant signs for episode authorization!
        participant_pubkey // Use participant's public key for episode authorization
    );
    
    // Submit transaction to blockchain via AuthHttpPeer
    println!("📤 Submitting RevokeSession transaction to Kaspa blockchain via AuthHttpPeer...");
    let submission_result = match state.auth_http_peer.as_ref().unwrap().submit_episode_message_transaction(
        step,
        participant_wallet.keypair,
        participant_addr.clone(),
        utxo,
    ).await {
        Ok(tx_id) => {
            println!("✅ RevokeSession transaction {} submitted successfully to blockchain via AuthHttpPeer!", tx_id);
            println!("📊 Transaction is now being processed by auth organizer peer's kdapp engine");
            (tx_id, "session_revocation_submitted".to_string())
        }
        Err(e) => {
            println!("❌ RevokeSession submission failed via AuthHttpPeer: {}", e);
            ("error".to_string(), "session_revocation_failed".to_string())
        }
    };
    
    let (transaction_id, status) = submission_result;
    
    Ok(Json(RevokeSessionResponse {
        episode_id,
        transaction_id,
        status,
    }))
}