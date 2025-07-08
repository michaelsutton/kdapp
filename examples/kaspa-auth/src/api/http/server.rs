// src/api/http/server.rs
use axum::{routing::{get, post}, Router, extract::State};
use axum::serve;
use std::sync::Arc;
use tokio::sync::broadcast;
use kdapp::generator::TransactionGenerator;
use crate::wallet::get_wallet_for_command;
use crate::episode_runner::create_auth_generator;
use kaspa_consensus_core::network::{NetworkId, NetworkType};
use tower_http::cors::{CorsLayer, Any};
use tower_http::services::ServeDir;
use std::collections::HashMap;
use std::sync::Mutex;
use secp256k1::Keypair;

use crate::api::http::{
    state::{ServerState, EpisodeState, WebSocketMessage},
    types::*,
    handlers::{
        auth::start_auth,
        challenge::request_challenge,
        verify::verify_auth,
        status::get_status,
    },
    blockchain_engine::AuthHttpServer,
};
use crate::api::websocket::server::websocket_handler;
use axum::Json;
use serde_json::json;
use kaspa_addresses::{Address, Prefix, Version};

// Simple endpoint handlers
async fn funding_info(State(state): State<ServerState>) -> Json<serde_json::Value> {
    let kaspa_addr = Address::new(
        Prefix::Testnet, 
        Version::PubKey, 
        &state.server_keypair.x_only_public_key().0.serialize()
    );
    
    Json(json!({
        "funding_address": kaspa_addr.to_string(),
        "network": "testnet-10",
        "transaction_prefix": "0x41555448",
        "transaction_prefix_meaning": "AUTH"
    }))
}

async fn wallet_status() -> Json<serde_json::Value> {
    // Check if web-client wallet exists
    match get_wallet_for_command("web-client", None) {
        Ok(wallet) => {
            let kaspa_addr = Address::new(
                Prefix::Testnet,
                Version::PubKey,
                &wallet.keypair.x_only_public_key().0.serialize()
            );
            
            Json(json!({
                "exists": true,
                "needs_funding": true,  // Always true for now - could check balance later
                "kaspa_address": kaspa_addr.to_string(),
                "was_created": wallet.was_created
            }))
        }
        Err(_) => {
            Json(json!({
                "exists": false,
                "needs_funding": true,
                "kaspa_address": "Will be created on first authentication"
            }))
        }
    }
}

async fn wallet_client() -> Json<serde_json::Value> {
    // Create a real client wallet (like CLI does)
    match get_wallet_for_command("web-client", None) {
        Ok(wallet) => {
            let public_key_hex = hex::encode(wallet.keypair.public_key().serialize());
            let kaspa_addr = Address::new(
                Prefix::Testnet,
                Version::PubKey,
                &wallet.keypair.x_only_public_key().0.serialize()
            );
            
            Json(json!({
                "public_key": public_key_hex,
                "kaspa_address": kaspa_addr.to_string(),
                "was_created": wallet.was_created,
                "needs_funding": true  // Always true for web clients for now
            }))
        }
        Err(e) => {
            Json(json!({
                "error": format!("Failed to create client wallet: {}", e),
                "public_key": "error",
                "kaspa_address": "error",
                "was_created": false,
                "needs_funding": true
            }))
        }
    }
}

async fn sign_challenge(Json(req): Json<serde_json::Value>) -> Json<serde_json::Value> {
    // Extract challenge and handle client wallet signing
    let challenge = req["challenge"].as_str().unwrap_or("");
    let private_key_hint = req["private_key"].as_str().unwrap_or("");
    
    if private_key_hint == "use_client_wallet" {
        // Use the web-client wallet to sign
        match get_wallet_for_command("web-client", None) {
            Ok(wallet) => {
                // Sign the challenge with the client wallet
                let message = kdapp::pki::to_message(&challenge.to_string());
                let signature = kdapp::pki::sign_message(&wallet.keypair.secret_key(), &message);
                let signature_hex = hex::encode(signature.0.serialize_der());
                let public_key_hex = hex::encode(wallet.keypair.public_key().serialize());
                
                Json(json!({
                    "challenge": challenge,
                    "signature": signature_hex,
                    "public_key": public_key_hex
                }))
            }
            Err(e) => {
                Json(json!({
                    "error": format!("Failed to sign challenge: {}", e)
                }))
            }
        }
    } else {
        Json(json!({
            "error": "Invalid signing request"
        }))
    }
}

pub async fn run_http_server(provided_private_key: Option<&str>, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let wallet = get_wallet_for_command("http-server", provided_private_key)?;
    let keypair = wallet.keypair;
    
    println!("🚀 Starting HTTP server with REAL kdapp blockchain integration");
    
    let (websocket_tx, _) = broadcast::channel::<WebSocketMessage>(100);
    
    // Create the AuthHttpServer with kdapp engine
    let auth_server = Arc::new(AuthHttpServer::new(keypair, websocket_tx.clone()).await?);
    let server_state = auth_server.server_state.clone();
    
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(tower_http::cors::AllowMethods::any())
        .allow_headers(Any);

    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .route("/funding-info", get(funding_info))
        .route("/wallet/status", get(wallet_status))
        .route("/wallet/client", get(wallet_client))
        .route("/auth/start", post(start_auth))
        .route("/auth/request-challenge", post(request_challenge))
        .route("/auth/sign-challenge", post(sign_challenge))
        .route("/auth/verify", post(verify_auth))
        .route("/auth/status/{episode_id}", get(get_status))
        .fallback_service(ServeDir::new("public"))
        .with_state(server_state)
        .layer(cors);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    println!("🚀 HTTP Authentication Server starting on port {}", port);
    println!("🔗 Starting kdapp blockchain engine...");
    
    // Start the blockchain listener in the background
    let auth_server_clone = auth_server.clone();
    tokio::spawn(async move {
        if let Err(e) = auth_server_clone.start_blockchain_listener().await {
            eprintln!("❌ Blockchain listener error: {}", e);
        }
    });
    
    // Start the HTTP server
    println!("✅ HTTP server is now a REAL kdapp blockchain node!");
    serve(listener, app.into_make_service()).await?;
    
    Ok(())
}