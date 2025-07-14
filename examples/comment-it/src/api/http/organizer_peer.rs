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
    println!("🎭 MATRIX UI ACTION: User checking wallet status");
    
    // Check if web-participant wallet exists WITHOUT creating it
    use crate::wallet::wallet_exists_for_command;
    
    if wallet_exists_for_command("web-participant") {
        // Load existing wallet to get details
        match get_wallet_for_command("web-participant", None) {
            Ok(wallet) => {
                let kaspa_addr = Address::new(
                    Prefix::Testnet,
                    Version::PubKey,
                    &wallet.keypair.public_key().serialize()[1..]
                );
                
                println!("✅ MATRIX UI SUCCESS: Existing wallet found - {}", kaspa_addr);
                
                Json(json!({
                    "exists": true,
                    "needs_funding": false, // Existing wallets assumed funded
                    "kaspa_address": kaspa_addr.to_string(),
                    "was_created": false
                }))
            }
            Err(e) => {
                println!("❌ MATRIX UI ERROR: Failed to load existing wallet - {}", e);
                Json(json!({
                    "exists": false,
                    "needs_funding": true,
                    "kaspa_address": "Wallet file corrupted - needs recreation",
                    "error": format!("Failed to load wallet: {}", e)
                }))
            }
        }
    } else {
        println!("⚠️ MATRIX UI INFO: No wallet found - user needs to create or import one");
        Json(json!({
            "exists": false,
            "needs_funding": true,
            "kaspa_address": "No wallet - user must create or import one"
        }))
    }
}

async fn wallet_participant() -> Json<serde_json::Value> {
    println!("🎭 MATRIX UI ACTION: User requesting participant wallet info");
    
    // Check if wallet exists WITHOUT creating it
    use crate::wallet::wallet_exists_for_command;
    
    if wallet_exists_for_command("web-participant") {
        // Load existing wallet to get details
        match get_wallet_for_command("web-participant", None) {
            Ok(wallet) => {
                let public_key_hex = hex::encode(wallet.keypair.public_key().serialize());
                let kaspa_addr = Address::new(
                    Prefix::Testnet,
                    Version::PubKey,
                    &wallet.keypair.public_key().serialize()[1..]
                );
                
                println!("✅ MATRIX UI SUCCESS: Existing participant wallet - {}", kaspa_addr);
                
                Json(json!({
                    "public_key": public_key_hex,
                    "kaspa_address": kaspa_addr.to_string(),
                    "was_created": false,
                    "needs_funding": false  // Existing wallets assumed funded
                }))
            }
            Err(e) => {
                println!("❌ MATRIX UI ERROR: Failed to load participant wallet - {}", e);
                Json(json!({
                    "error": format!("Failed to load participant wallet: {}", e),
                    "public_key": "error",
                    "kaspa_address": "error",
                    "was_created": false,
                    "needs_funding": true
                }))
            }
        }
    } else {
        println!("⚠️ MATRIX UI INFO: No participant wallet found - user needs to create one");
        Json(json!({
            "error": "No participant wallet found - user must create or import one",
            "public_key": "none",
            "kaspa_address": "none",
            "was_created": false,
            "needs_funding": true
        }))
    }
}

async fn wallet_participant_post(Json(req): Json<serde_json::Value>) -> Json<serde_json::Value> {
    // Handle participant peer wallet creation/import from web interface
    if let Some(private_key_hex) = req["private_key"].as_str() {
        let save_to_file = req["save_to_file"].as_bool().unwrap_or(false);
        
        println!("🎭 MATRIX UI ACTION: User {} wallet with private key", 
                 if save_to_file { "creating/importing and saving" } else { "importing temporarily" });
        
        // Validate private key format
        if private_key_hex.len() != 64 {
            println!("❌ MATRIX UI ERROR: Invalid private key length ({})", private_key_hex.len());
            return Json(json!({
                "error": "Invalid private key length. Must be 64 hexadecimal characters.",
                "success": false
            }));
        }
        
        // Decode private key
        let private_key_bytes = match hex::decode(private_key_hex) {
            Ok(bytes) => bytes,
            Err(_) => {
                println!("❌ MATRIX UI ERROR: Invalid private key format (not hex)");
                return Json(json!({
                    "error": "Invalid private key format. Must be hexadecimal.",
                    "success": false
                }));
            }
        };
        
        if private_key_bytes.len() != 32 {
            println!("❌ MATRIX UI ERROR: Invalid private key length ({} bytes)", private_key_bytes.len());
            return Json(json!({
                "error": "Invalid private key length. Must be 32 bytes.",
                "success": false
            }));
        }
        
        // Create wallet from private key
        let wallet_result = if save_to_file {
            // Save to participant peer wallet file
            use crate::wallet::KaspaAuthWallet;
            KaspaAuthWallet::from_private_key_and_save(private_key_hex, "participant-peer-wallet.key")
        } else {
            // Use temporarily without saving
            get_wallet_for_command("web-participant", Some(private_key_hex))
        };
        
        match wallet_result {
            Ok(wallet) => {
                let public_key_hex = hex::encode(wallet.keypair.public_key().serialize());
                let kaspa_addr = Address::new(
                    Prefix::Testnet,
                    Version::PubKey,
                    &wallet.keypair.public_key().serialize()[1..]
                );
                
                println!("✅ MATRIX UI SUCCESS: Wallet {} for address: {}", 
                         if save_to_file { "created/imported and saved" } else { "created/imported temporarily" }, 
                         kaspa_addr);
                println!("🔑 Public Key: {}", public_key_hex);
                if save_to_file {
                    println!("💾 Saved to: .kaspa-auth/participant-peer-wallet.key");
                }
                
                Json(json!({
                    "public_key": public_key_hex,
                    "kaspa_address": kaspa_addr.to_string(),
                    "was_created": wallet.was_created,
                    "save_to_file": save_to_file,
                    "needs_funding": true,
                    "success": true
                }))
            }
            Err(e) => {
                println!("❌ MATRIX UI ERROR: Failed to create wallet: {}", e);
                Json(json!({
                    "error": format!("Failed to create wallet from private key: {}", e),
                    "success": false
                }))
            }
        }
    } else {
        Json(json!({
            "error": "Missing private_key field",
            "success": false
        }))
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
        .route("/wallet-participant", get(wallet_participant))
        .route("/wallet-participant", post(wallet_participant_post))
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
    println!();
    println!("🎭 MATRIX UI READY - Waiting for user actions...");
    println!("💻 Web dashboard available at: http://localhost:{}/", port);
    println!("🚀 Backend will respond to frontend wallet creation/import actions");
    println!();
    
    // Start the blockchain listener in the background
    let auth_peer_clone = auth_peer.clone();
    tokio::spawn(async move {
        if let Err(e) = auth_peer_clone.start_blockchain_listener().await {
            eprintln!("❌ Blockchain listener error: {}", e);
        }
    });
    
    // Start the HTTP coordination peer
    println!("🔗 kdapp engine started - HTTP coordination peer is now a real blockchain node!");
    serve(listener, app.into_make_service()).await?;
    
    Ok(())
}