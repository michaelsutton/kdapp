// src/api/http/server.rs
use axum::{routing::{get, post}, Router, extract::State};
use axum::serve;
use std::sync::Arc;
use tokio::sync::broadcast;
use crate::wallet::get_wallet_for_command;
use tower_http::cors::{CorsLayer, Any};
use tower_http::services::ServeDir;

use crate::api::http::{
    state::{PeerState, WebSocketMessage},
    handlers::{
        auth::start_auth,
        challenge::request_challenge,
        verify::verify_auth,
        status::get_status,
        revoke::revoke_session,
    },
    blockchain_engine::AuthHttpPeer,
};
use crate::api::http::websocket::websocket_handler;
use axum::Json;
use serde_json::json;
use kaspa_addresses::{Address, Prefix, Version};

// Simple endpoint handlers
async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "service": "kaspa-auth-http-peer",
        "version": "0.1.0"
    }))
}

async fn funding_info(State(state): State<PeerState>) -> Json<serde_json::Value> {
    let kaspa_addr = Address::new(
        Prefix::Testnet, 
        Version::PubKey, 
        &state.peer_keypair.x_only_public_key().0.serialize()
    );
    
    Json(json!({
        "funding_address": kaspa_addr.to_string(),
        "network": "testnet-10",
        "transaction_prefix": "0x41555448",
        "transaction_prefix_meaning": "AUTH"
    }))
}

async fn wallet_status() -> Json<serde_json::Value> {
    // Check if web-participant wallet exists
    match get_wallet_for_command("web-participant", None) {
        Ok(wallet) => {
            let kaspa_addr = Address::new(
                Prefix::Testnet,
                Version::PubKey,
                &wallet.keypair.public_key().serialize()[1..]
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
    // Create a real participant wallet (like CLI does)
    match get_wallet_for_command("web-participant", None) {
        Ok(wallet) => {
            let public_key_hex = hex::encode(wallet.keypair.public_key().serialize());
            let kaspa_addr = Address::new(
                Prefix::Testnet,
                Version::PubKey,
                &wallet.keypair.public_key().serialize()[1..]
            );
            
            Json(json!({
                "public_key": public_key_hex,
                "kaspa_address": kaspa_addr.to_string(),
                "was_created": wallet.was_created,
                "needs_funding": true  // Always true for web participants for now
            }))
        }
        Err(e) => {
            Json(json!({
                "error": format!("Failed to create participant wallet: {}", e),
                "public_key": "error",
                "kaspa_address": "error",
                "was_created": false,
                "needs_funding": true
            }))
        }
    }
}

async fn sign_challenge(Json(req): Json<serde_json::Value>) -> Json<serde_json::Value> {
    // Extract challenge and handle participant wallet signing
    let challenge = req["challenge"].as_str().unwrap_or("");
    let private_key_hint = req["private_key"].as_str().unwrap_or("");
    
    if private_key_hint == "use_client_wallet" || private_key_hint == "use_participant_wallet" {
        // Use the web-participant wallet to sign
        match get_wallet_for_command("web-participant", None) {
            Ok(wallet) => {
                // Sign the challenge with the participant wallet
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

async fn wallet_debug() -> Json<serde_json::Value> {
    let mut debug_info = json!({});
    
    // Check all wallet types
    let wallet_types = vec![
        ("web-participant", "participant-peer-wallet.key"),
        ("authenticate", "participant-peer-wallet.key"),
        ("participant-peer", "participant-peer-wallet.key"),
        ("organizer-peer", "organizer-peer-wallet.key"),
        ("http-peer", "organizer-peer-wallet.key"),
    ];
    
    for (command, expected_file) in wallet_types {
        match get_wallet_for_command(command, None) {
            Ok(wallet) => {
                let public_key_hex = hex::encode(wallet.keypair.public_key().serialize());
                let kaspa_addr = Address::new(
                    Prefix::Testnet,
                    Version::PubKey,
                    &wallet.keypair.public_key().serialize()[1..]
                );
                
                debug_info[command] = json!({
                    "public_key": public_key_hex,
                    "kaspa_address": kaspa_addr.to_string(),
                    "expected_file": expected_file,
                    "was_created": wallet.was_created
                });
            }
            Err(e) => {
                debug_info[command] = json!({
                    "error": format!("Failed to load wallet: {}", e),
                    "expected_file": expected_file
                });
            }
        }
    }
    
    Json(debug_info)
}

async fn episode_authenticated(
    State(state): State<PeerState>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let episode_id = payload["episode_id"].as_u64().unwrap_or(0);
    let challenge = payload["challenge"].as_str().unwrap_or("");
    
    // Get the real session token from blockchain episode
    let real_session_token = if let Ok(episodes) = state.blockchain_episodes.lock() {
        if let Some(episode) = episodes.get(&episode_id) {
            episode.session_token.clone()
        } else {
            None
        }
    } else {
        None
    };
    
    // Broadcast WebSocket message for authentication success
    let ws_message = WebSocketMessage {
        message_type: "authentication_successful".to_string(),
        episode_id: Some(episode_id),
        authenticated: Some(true),
        challenge: Some(challenge.to_string()),
        session_token: real_session_token,
    };
    
    // Send to all connected WebSocket clients
    let _ = state.websocket_tx.send(ws_message);
    
    Json(json!({
        "status": "success",
        "episode_id": episode_id,
        "message": "Authentication notification sent"
    }))
}

async fn session_revoked(
    State(state): State<PeerState>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let episode_id = payload["episode_id"].as_u64().unwrap_or(0);
    let session_token = payload["session_token"].as_str().unwrap_or("");
    
    println!("🔔 Received session revocation notification for episode {}, token: {}", episode_id, session_token);
    
    // Broadcast WebSocket message for session revocation success
    let ws_message = WebSocketMessage {
        message_type: "session_revoked".to_string(),
        episode_id: Some(episode_id),
        authenticated: Some(false),
        challenge: None,
        session_token: Some(session_token.to_string()),
    };
    
    // Send to all connected WebSocket clients
    match state.websocket_tx.send(ws_message) {
        Ok(_) => {
            println!("✅ Session revocation WebSocket message sent for episode {}", episode_id);
        }
        Err(e) => {
            println!("❌ Failed to send session revocation WebSocket message: {}", e);
        }
    }
    
    Json(json!({
        "status": "success",
        "episode_id": episode_id,
        "session_token": session_token,
        "message": "Session revocation notification sent"
    }))
}

pub async fn run_http_peer(provided_private_key: Option<&str>, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let wallet = get_wallet_for_command("http-peer", provided_private_key)?;
    let keypair = wallet.keypair;
    
    println!("🚀 Starting HTTP coordination peer with REAL kdapp blockchain integration");
    
    let (websocket_tx, _) = broadcast::channel::<WebSocketMessage>(100);
    
    // Create the AuthHttpPeer with kdapp engine
    let auth_peer = Arc::new(AuthHttpPeer::new(keypair, websocket_tx.clone()).await?);
    let peer_state = PeerState {
        episodes: auth_peer.peer_state.episodes.clone(),
        blockchain_episodes: auth_peer.peer_state.blockchain_episodes.clone(),
        websocket_tx: auth_peer.peer_state.websocket_tx.clone(),
        peer_keypair: auth_peer.peer_state.peer_keypair,
        transaction_generator: auth_peer.peer_state.transaction_generator.clone(),
        kaspad_client: auth_peer.peer_state.kaspad_client.clone(),
        auth_http_peer: Some(auth_peer.clone()), // Pass the Arc<AuthHttpPeer> here
    };
    
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(tower_http::cors::AllowMethods::any())
        .allow_headers(Any);

    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .route("/health", get(health))
        .route("/funding-info", get(funding_info))
        .route("/wallet/status", get(wallet_status))
        .route("/wallet/client", get(wallet_client))
        .route("/wallet/debug", get(wallet_debug))
        .route("/auth/start", post(start_auth))
        .route("/auth/request-challenge", post(request_challenge))
        .route("/auth/sign-challenge", post(sign_challenge))
        .route("/auth/verify", post(verify_auth))
        .route("/auth/revoke-session", post(revoke_session))
        .route("/auth/status/{episode_id}", get(get_status))
        .route("/internal/episode-authenticated", post(episode_authenticated))
        .route("/internal/session-revoked", post(session_revoked))
        .fallback_service(ServeDir::new("public"))
        .with_state(peer_state)
        .layer(cors);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    println!("🚀 HTTP Authentication Coordination Peer starting on port {}", port);
    println!("🔗 Starting kdapp blockchain engine...");
    
    // Show participant wallet funding information
    match get_wallet_for_command("web-participant", None) {
        Ok(wallet) => {
            let participant_addr = Address::new(
                Prefix::Testnet,
                Version::PubKey,
                &wallet.keypair.public_key().serialize()[1..]
            );
            println!();
            println!("💰 PARTICIPANT WALLET FUNDING REQUIRED:");
            println!("📍 Participant Address: {}", participant_addr);
            println!("🚰 Get testnet funds: https://faucet.kaspanet.io/");
            println!("💡 Participants must fund their own authentication transactions");
            println!("🌐 Network: testnet-10");
            println!();
        }
        Err(_e) => {
            println!("⚠️  Participant wallet creation pending (will be created on first use)");
        }
    }
    
    // Start the blockchain listener in the background
    let auth_peer_clone = auth_peer.clone();
    tokio::spawn(async move {
        if let Err(e) = auth_peer_clone.start_blockchain_listener().await {
            eprintln!("❌ Blockchain listener error: {}", e);
        }
    });
    
    // Start the HTTP coordination peer
    println!("🔗 kdapp engine started - HTTP coordination peer is now a real blockchain node!");
    println!("🌐 Web dashboard available at: http://localhost:{}/", port);
    serve(listener, app.into_make_service()).await?;
    
    Ok(())
}