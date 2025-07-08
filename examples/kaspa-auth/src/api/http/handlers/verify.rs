// src/api/http/handlers/verify.rs
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
    types::{VerifyRequest, VerifyResponse},
    state::ServerState,
};
use crate::core::{episode::SimpleAuth, commands::AuthCommand};

pub async fn verify_auth(
    State(state): State<ServerState>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, StatusCode> {
    println!("📤 Sending SubmitResponse command to blockchain...");
    
    // Find the client public key from the episode
    let episode = match state.blockchain_episodes.lock() {
        Ok(episodes) => {
            episodes.get(&req.episode_id).cloned()
        }
        Err(e) => {
            println!("❌ Failed to lock blockchain episodes: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    
    let client_pubkey = match episode {
        Some(ep) => ep.owner.unwrap_or_else(|| {
            println!("❌ Episode has no owner public key");
            // This shouldn't happen, but let's continue anyway
            PubKey(secp256k1::PublicKey::from_slice(&[2; 33]).unwrap())
        }),
        None => {
            println!("❌ Episode {} not found in blockchain state", req.episode_id);
            return Err(StatusCode::NOT_FOUND);
        }
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
        println!("🔍 Fetching UTXOs for SubmitResponse transaction...");
        
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
            println!("❌ No UTXOs found! Client wallet needs funding.");
            println!("💰 Fund this address: {}", client_addr);
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
    println!("🚀 Submitting SubmitResponse transaction: {}", tx.id());
    
    let submission_result = match state.kaspad_client.as_ref().unwrap().submit_transaction(tx.as_ref().into(), false).await {
        Ok(_response) => {
            println!("✅ SubmitResponse transaction submitted to blockchain!");
            println!("📊 Transactions are now being processed by auth server's kdapp engine");
            "submit_response_submitted"
        }
        Err(e) => {
            println!("❌ SubmitResponse submission failed: {}", e);
            "submit_response_failed"
        }
    };
    
    Ok(Json(VerifyResponse {
        episode_id: req.episode_id,
        authenticated: false, // Will be updated by blockchain when processed
        status: submission_result.to_string(),
    }))
}