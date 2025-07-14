// src/api/http/handlers/verify.rs
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
    types::{VerifyRequest, VerifyResponse},
    state::PeerState,
};
use crate::core::{episode::SimpleAuth, commands::AuthCommand};

pub async fn verify_auth(
    State(state): State<PeerState>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, StatusCode> {
    println!("🎭 MATRIX UI ACTION: User submitted authentication signature");
    println!("🔍 DEBUG: Verify request received - episode_id: {}, nonce: {}", req.episode_id, req.nonce);
    println!("🔍 DEBUG: Signature length: {}", req.signature.len());
    println!("📤 Sending SubmitResponse command to blockchain...");
    
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
    
    let participant_pubkey = match episode {
        Some(ep) => ep.owner.unwrap_or_else(|| {
            println!("❌ Episode has no owner public key");
            // This shouldn't happen, but let's continue anyway
            PubKey(secp256k1::PublicKey::from_slice(&[2; 33]).unwrap())
        }),
        None => {
            println!("❌ Episode {} not found in blockchain state", episode_id);
            return Err(StatusCode::NOT_FOUND);
        }
    };
    
    // 🚨 CRITICAL: In HTTP demo mode, participant must sign the command
    // The organizer can fund the transaction, but the participant must be the signer
    // for the episode authorization to work correctly
    
    // Create participant Kaspa address for transaction funding (like CLI does)
    let _participant_addr = Address::new(
        Prefix::Testnet, 
        Version::PubKey, 
        &participant_pubkey.0.x_only_public_key().0.serialize()
    );
    
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
    let utxo = if let Some(ref kaspad) = state.kaspad_client {
        println!("🔍 Fetching UTXOs for SubmitResponse transaction...");
        
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
            println!("❌ MATRIX UI ERROR: Participant wallet needs funding for signature verification");
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
    
    // Create SubmitResponse command (exactly like CLI)
    let auth_command = AuthCommand::SubmitResponse {
        signature: req.signature.clone(),
        nonce: req.nonce.clone(),
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
    
    // Submit transaction to blockchain via AuthHttpPeer (centralized submission)
    println!("📤 Submitting SubmitResponse transaction to Kaspa blockchain via AuthHttpPeer...");
    let submission_result = match state.auth_http_peer.as_ref().unwrap().submit_episode_message_transaction(
        step,
        participant_wallet.keypair,
        participant_addr.clone(),
        utxo,
    ).await {
        Ok(tx_id) => {
            println!("✅ MATRIX UI SUCCESS: Authentication signature submitted - Transaction {}", tx_id);
            println!("📊 Transactions are now being processed by auth organizer peer's kdapp engine");
            (tx_id, "submit_response_submitted".to_string())
        }
        Err(e) => {
            println!("❌ MATRIX UI ERROR: Authentication signature submission failed - {}", e);
            ("error".to_string(), "submit_response_failed".to_string())
        }
    };
    
    let (transaction_id, status) = submission_result;
    
    Ok(Json(VerifyResponse {
        episode_id,
        authenticated: false, // Will be updated by blockchain when processed
        status,
        transaction_id: Some(transaction_id),
    }))
}