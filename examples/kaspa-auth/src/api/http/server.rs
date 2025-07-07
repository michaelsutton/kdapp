// src/api/http/server.rs
use axum::{routing::{get, post}, Router, response::Json, extract::{Path, State}, http::StatusCode, http::Method};
use secp256k1::Keypair;
use axum::serve;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use kdapp::pki::{sign_message, to_message};
use kaspa_addresses::{Address, Prefix, Version};
use kaspa_consensus_core::network::{NetworkId, NetworkType};
use tokio::sync::broadcast;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures_util::{sink::SinkExt, stream::StreamExt};
use log::{info, error};
use rand::Rng;
use crate::events::{AuthEvent, EventEmitter};
use crate::economics::{EconomicManager, EconomicParams};
use crate::api::http::endpoints::get_api_endpoints;
use crate::wallet::get_wallet_for_command;
use crate::episode_runner::{create_auth_generator, AUTH_PREFIX};
use kdapp::proxy::connect_client;
use kdapp::generator::TransactionGenerator;
use kdapp::engine::EpisodeMessage;
use kdapp::pki::PubKey;
use crate::core::commands::AuthCommand;
use crate::core::episode::SimpleAuth;
use tower_http::cors::{CorsLayer, Any};
use serde_with::{serde_as, DisplayFromStr};

// Episode storage with full state
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct EpisodeState {
    #[serde_as(as = "DisplayFromStr")]
    episode_id: u64,
    public_key: String,
    challenge: Option<String>,
    authenticated: bool,
    session_token: Option<String>,
}

type EpisodeStorage = Arc<Mutex<HashMap<u64, EpisodeState>>>;

// WebSocket message types
#[serde_as]
#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    #[serde(rename = "challenge_issued")]
    ChallengeIssued {
        #[serde_as(as = "DisplayFromStr")]
        episode_id: u64,
        challenge: String,
        timestamp: u64,
    },
    #[serde(rename = "authentication_successful")]
    AuthenticationSuccessful {
        #[serde_as(as = "DisplayFromStr")]
        episode_id: u64,
        session_token: String,
    },
    #[serde(rename = "authentication_failed")]
    AuthenticationFailed {
        #[serde_as(as = "DisplayFromStr")]
        episode_id: u64,
        reason: String,
    },
    #[serde(rename = "episode_updated")]
    EpisodeUpdated {
        #[serde_as(as = "DisplayFromStr")]
        episode_id: u64,
        challenge: Option<String>,
        authenticated: bool,
    },
}

// Server state with WebSocket broadcasting and event emission
#[derive(Clone)]
pub struct ServerState {
    episodes: EpisodeStorage,
    websocket_tx: broadcast::Sender<WebSocketMessage>,
    event_emitter: EventEmitter,
    economic_manager: Arc<Mutex<EconomicManager>>,
    server_keypair: Keypair,
    transaction_generator: Arc<TransactionGenerator>,
    client_wallet: Arc<Mutex<Option<crate::wallet::KaspaAuthWallet>>>, // Cached client wallet
}

impl ServerState {
    // Get cached client wallet or load it on first use
    fn get_client_wallet(&self) -> Result<crate::wallet::KaspaAuthWallet, Box<dyn std::error::Error>> {
        let mut wallet_cache = self.client_wallet.lock().unwrap();
        
        if wallet_cache.is_none() {
            println!("📁 Loading client wallet from: .kaspa-auth/client-wallet.key");
            let wallet = crate::wallet::get_wallet_for_command("client", None)?;
            *wallet_cache = Some(wallet.clone());
            Ok(wallet)
        } else {
            // Return cached wallet (no duplicate loading!)
            Ok(wallet_cache.as_ref().unwrap().clone())
        }
    }
    
    // Method to update episode authentication status (called by kdapp engine)
    pub fn mark_episode_authenticated(&self, episode_id: u64, _challenge: String) {
        use rand::Rng;
        if let Ok(mut episodes) = self.episodes.lock() {
            if let Some(episode) = episodes.get_mut(&episode_id) {
                episode.authenticated = true;
                let session_token = format!("sess_{}", rand::thread_rng().gen::<u64>());
                episode.session_token = Some(session_token.clone());
                
                // Emit events
                self.event_emitter.emit(AuthEvent::AuthenticationAttempted {
                    episode_id,
                    success: true,
                    participant: episode.public_key.clone(),
                });
                
                self.event_emitter.emit(AuthEvent::SessionCreated {
                    episode_id,
                    session_token: session_token.clone(),
                    expires_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() + 3600, // 1 hour expiry
                });
                
                // Distribute success reward
                self.economic_manager.lock().unwrap().distribute_success_reward(
                    episode_id, 
                    &episode.public_key
                );
                
                // Broadcast authentication success via WebSocket
                let _ = self.websocket_tx.send(WebSocketMessage::AuthenticationSuccessful {
                    episode_id,
                    session_token: session_token.clone(),
                });
                
                println!("✅ Episode {} authenticated via blockchain - session token: {}", episode_id, session_token);
            }
        }
    }
}

// Request/Response types
#[derive(Deserialize)]
struct StartAuthRequest {
    public_key: String,
}

#[serde_as]
#[derive(Deserialize)]
struct RegisterEpisodeRequest {
    #[serde_as(as = "DisplayFromStr")]
    episode_id: u64,
    public_key: String,
    client_kaspa_address: String,
}

#[serde_as]
#[derive(Serialize)]
struct StartAuthResponse {
    #[serde_as(as = "DisplayFromStr")]
    episode_id: u64,
    status: String,
    client_kaspa_address: String,
}

#[serde_as]
#[derive(Deserialize)]
struct RequestChallengeRequest {
    #[serde_as(as = "DisplayFromStr")]
    episode_id: u64,
    public_key: String,
}

#[serde_as]
#[derive(Serialize)]
struct ChallengeResponse {
    #[serde_as(as = "DisplayFromStr")]
    episode_id: u64,
    status: String,
    message: String,
}

#[derive(Deserialize)]
struct SignChallengeRequest {
    challenge: String,
    private_key: String,
}

#[derive(Serialize)]
struct SignChallengeResponse {
    challenge: String,
    signature: String,
    public_key: String,
}

#[serde_as]
#[derive(Deserialize)]
struct VerifyRequest {
    #[serde_as(as = "DisplayFromStr")]
    episode_id: u64,
    signature: String,
    nonce: String,
}

#[serde_as]
#[derive(Serialize)]
struct VerifyResponse {
    #[serde_as(as = "DisplayFromStr")]
    episode_id: u64,
    authenticated: bool,
    status: String,
}

#[serde_as]
#[derive(Serialize)]
struct StatusResponse {
    #[serde_as(as = "DisplayFromStr")]
    episode_id: u64,
    authenticated: bool,
    challenge: Option<String>,
    session_token: Option<String>,
    status: String,
}

pub async fn run_http_server(provided_private_key: Option<&str>, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    // Use unified wallet system
    let wallet = get_wallet_for_command("http-server", provided_private_key)?;
    let keypair = wallet.keypair;
    
    // Create transaction generator with AUTH_PREFIX/AUTH_PATTERN
    let network_id = NetworkId::with_suffix(NetworkType::Testnet, 10);
    let transaction_generator = create_auth_generator(keypair, network_id);
    
    let episode_storage: EpisodeStorage = Arc::new(Mutex::new(HashMap::new()));
    let (websocket_tx, _) = broadcast::channel(100);
    let event_emitter = EventEmitter::new();
    let economic_manager = Arc::new(Mutex::new(EconomicManager::new(EconomicParams::default())));
    
    let server_state = ServerState {
        episodes: episode_storage,
        websocket_tx,
        event_emitter,
        economic_manager,
        server_keypair: keypair,
        transaction_generator: Arc::new(transaction_generator),
        client_wallet: Arc::new(Mutex::new(None)), // Initialize empty, load on first use
    };
    
    async fn hello_world() -> Json<serde_json::Value> {
        Json(serde_json::json!({"message": "Kaspa Auth HTTP Server", "status": "running"}))
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(serve_web_ui))  // Serve Web UI as the root page
        .route("/health", get(hello_world))
        .route("/ws", get(websocket_handler))
        .route("/web", get(serve_web_ui))  // Also available at /web for compatibility
        .route("/funding-info", get(get_funding_info))
        .route("/auth/start", post(start_auth))
        .route("/auth/register-episode", post(register_episode))
        .route("/auth/request-challenge", post(request_challenge))
        .route("/auth/sign-challenge", post(sign_challenge))
        .route("/auth/verify", post(verify_auth))
        .route("/auth/status/{episode_id}", get(get_status))
        .route("/auth/reset", post(reset_episodes))
        .route("/challenge/{episode_id}", get(get_challenge))
        .route("/wallet/client", get(get_client_wallet))
        .route("/wallet/status", get(get_wallet_status))
        .route("/internal/episode-authenticated", post(internal_episode_authenticated))
        
        .with_state(server_state)
        .layer(cors);

    let addr = format!("0.0.0.0:{}", port);
    
    let _kaspa_address = wallet.get_kaspa_address();
    
    println!("🚀 HTTP Authentication Server starting on port {}", port);
    println!("🔗 Blockchain: {} (testnet-10)", network_id);
    println!("🏷️  Transaction Prefix: 0x{:08X} (AUTH)", AUTH_PREFIX);
    println!("📡 Endpoints:");
    for endpoint in get_api_endpoints() {
        println!("  {:>4} {:<30} - {}", endpoint.method, endpoint.path, endpoint.description);
    }
    println!();
    println!("🌐 Web UI:");
    println!("  Open http://localhost:{} in your browser for the main dashboard", port);
    println!("  Also available at: http://localhost:{}/web", port);
    println!("🔌 WebSocket Support:");
    println!("  Connect to ws://localhost:{}/ws for real-time auth updates", port);
    println!("  Messages: challenge_issued, authentication_successful, authentication_failed, episode_updated");
    println!();
    println!("✅ Server running! Example workflow:");
    println!("  curl -X POST http://localhost:{}/auth/start -H 'Content-Type: application/json' -d '{{\"public_key\": \"YOUR_PUBKEY\"}}'", port);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    serve(listener, app.into_make_service()).await?;

    Ok(())
}

// WebSocket handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, state.websocket_tx))
}

async fn handle_websocket(
    socket: WebSocket,
    websocket_tx: broadcast::Sender<WebSocketMessage>,
) {
    info!("New WebSocket connection established");
    let mut rx = websocket_tx.subscribe();
    let (mut sender, mut receiver) = socket.split();

    // Task for sending messages to client
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let json_msg = match serde_json::to_string(&msg) {
                Ok(json) => json,
                Err(e) => {
                    error!("Failed to serialize WebSocket message: {}", e);
                    continue;
                }
            };
            if sender.send(Message::Text(json_msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Task for receiving messages from client  
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            info!("Received WebSocket message: {}", text);
            // Handle client messages if needed
        }
    });

    // Clean shutdown when either task completes
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    info!("WebSocket connection closed");
}

// Handler implementations
async fn start_auth(
    State(state): State<ServerState>,
    Json(req): Json<StartAuthRequest>,
) -> Result<Json<StartAuthResponse>, StatusCode> {
    use rand::Rng;
    
    // Check if an episode already exists for this public key
    let mut episodes = state.episodes.lock().unwrap();
    let existing_episode = episodes.iter().find(|(_, episode)| episode.public_key == req.public_key);
    
    let (episode_id, episode, is_reused) = if let Some((id, existing)) = existing_episode {
        // Reuse existing episode
        println!("🔄 Reusing existing episode {} for public key: {}", id, req.public_key);
        (*id, existing.clone(), true)
    } else {
        // Create new episode only if none exists for this public key
        let episode_id = rand::thread_rng().gen::<u64>();
        let episode = EpisodeState {
            episode_id,
            public_key: req.public_key.clone(),
            challenge: None,
            authenticated: false,
            session_token: None,
        };
        
        episodes.insert(episode_id, episode.clone());
        println!("🆕 Created new episode {} for public key: {}", episode_id, req.public_key);
        (episode_id, episode, false)
    };
    
    drop(episodes); // Release the lock early
    
    // Emit event (only for new episodes)
    if !is_reused {
        state.event_emitter.emit(AuthEvent::EpisodeCreated {
            episode_id,
            participants: vec![req.public_key.clone()],
        });
    }
    
    // Broadcast episode status (creation or reuse)
    let _ = state.websocket_tx.send(WebSocketMessage::EpisodeUpdated {
        episode_id,
        challenge: episode.challenge,
        authenticated: episode.authenticated,
    });
    
    // Derive Kaspa address from client's public key
    let client_pubkey_bytes = hex::decode(&req.public_key).map_err(|_| StatusCode::BAD_REQUEST)?;
    if client_pubkey_bytes.len() != 33 { // Compressed public key length
        return Err(StatusCode::BAD_REQUEST);
    }
    let client_kaspa_address = Address::new(
        Prefix::Testnet, // Assuming testnet for now, could be dynamic
        Version::PubKey,
        &client_pubkey_bytes[1..] // Skip the prefix byte (0x02 or 0x03)
    ).to_string();

    println!("📝 Created episode {} for public key: {} (Kaspa Address: {})", episode_id, req.public_key, client_kaspa_address);
    
    Ok(Json(StartAuthResponse {
        episode_id,
        status: "episode_created".to_string(),
        client_kaspa_address,
    }))
}

async fn register_episode(
    State(state): State<ServerState>,
    Json(req): Json<RegisterEpisodeRequest>,
) -> Result<Json<StartAuthResponse>, StatusCode> {
    let episode = EpisodeState {
        episode_id: req.episode_id,
        public_key: req.public_key.clone(),
        challenge: None,
        authenticated: false,
        session_token: None,
    };
    
    state.episodes.lock().unwrap().insert(req.episode_id, episode.clone());
    
    // Broadcast episode registration
    let _ = state.websocket_tx.send(WebSocketMessage::EpisodeUpdated {
        episode_id: req.episode_id,
        challenge: None,
        authenticated: false,
    });
    
    println!("📝 Registered blockchain episode {} for public key: {} (Kaspa Address: {})", req.episode_id, req.public_key, req.client_kaspa_address);
    
    Ok(Json(StartAuthResponse {
        episode_id: req.episode_id,
        status: "episode_registered".to_string(),
        client_kaspa_address: req.client_kaspa_address,
    }))
}

async fn request_challenge(
    State(state): State<ServerState>,
    Json(req): Json<RequestChallengeRequest>,
) -> Result<Json<ChallengeResponse>, StatusCode> {
    println!("🔍 DEBUG: request_challenge called with episode_id: {}, public_key: {}", req.episode_id, req.public_key);
    
    use rand::Rng;
    let challenge = format!("auth_{}", rand::thread_rng().gen::<u64>());
    
    // Debug: Print all stored episodes
    {
        let episodes = state.episodes.lock().unwrap();
        println!("🔍 DEBUG: Current episodes in storage: {:?}", episodes.keys().collect::<Vec<_>>());
    }
    
    let mut episodes = state.episodes.lock().unwrap();
    let episode_state = match episodes.get_mut(&req.episode_id) {
        Some(episode) => episode,
        None => {
            println!("❌ Episode {} not found", req.episode_id);
            return Err(StatusCode::NOT_FOUND);
        }
    };
    
    episode_state.challenge = Some(challenge.clone());
    
    // Emit event
    state.event_emitter.emit(AuthEvent::ChallengeIssued {
        episode_id: req.episode_id,
        challenge: challenge.clone(),
        requester: req.public_key.clone(),
    });
    
    // Broadcast challenge issued
    let _ = state.websocket_tx.send(WebSocketMessage::ChallengeIssued {
        episode_id: req.episode_id,
        challenge: challenge.clone(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    });
    
    println!("🎲 Generated challenge {} for episode {}", challenge, req.episode_id);
    
    Ok(Json(ChallengeResponse {
        episode_id: req.episode_id,
        status: "challenge_requested".to_string(),
        message: "RequestChallenge command sent to blockchain...".to_string(),
    }))
}

async fn sign_challenge(
    State(state): State<ServerState>,
    Json(req): Json<SignChallengeRequest>,
) -> Result<Json<SignChallengeResponse>, StatusCode> {
    use secp256k1::{Secp256k1, SecretKey};
    
    let (secret_key, public_key_hex) = if req.private_key == "use_client_wallet" {
        // Use cached client wallet (no duplicate loading!)
        match state.get_client_wallet() {
            Ok(wallet) => {
                let secret_key = wallet.keypair.secret_key();
                let public_key_hex = wallet.get_public_key_hex();
                (secret_key, public_key_hex)
            },
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else {
        // Parse provided private key (legacy mode)
        let secret_bytes = match hex::decode(&req.private_key) {
            Ok(bytes) => bytes,
            Err(_) => return Err(StatusCode::BAD_REQUEST),
        };
        
        let secret_key = match SecretKey::from_slice(&secret_bytes) {
            Ok(key) => key,
            Err(_) => return Err(StatusCode::BAD_REQUEST),
        };
        
        let secp = Secp256k1::new();
        let keypair = secp256k1::Keypair::from_secret_key(&secp, &secret_key);
        let public_key_hex = hex::encode(keypair.public_key().serialize());
        
        (secret_key, public_key_hex)
    };
    
    // Sign the challenge
    let message = to_message(&req.challenge);
    let signature = sign_message(&secret_key, &message);
    let signature_hex = hex::encode(signature.0.serialize_der());
    
    println!("✍️ Signed challenge: {} with key: {}", req.challenge, public_key_hex);
    
    Ok(Json(SignChallengeResponse {
        challenge: req.challenge,
        signature: signature_hex,
        public_key: public_key_hex,
    }))
}

async fn verify_auth(
    State(state): State<ServerState>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, StatusCode> {
    use crate::core::commands::AuthCommand;
    use crate::core::episode::SimpleAuth;
    
    // Get episode state
    let episodes = state.episodes.lock().unwrap();
    let episode_state = match episodes.get(&req.episode_id) {
        Some(episode) => episode.clone(),
        None => {
            println!("❌ Episode {} not found", req.episode_id);
            return Err(StatusCode::NOT_FOUND);
        }
    };
    drop(episodes); // Explicitly release the lock
    
    println!(" DEBUG: verify_auth called for episode {} with signature: {}, nonce: {}", 
            req.episode_id, req.signature, req.nonce);
    
    // Parse client's public key for authorization
    let client_pubkey = match parse_client_pubkey(&episode_state.public_key) {
        Ok(pubkey) => pubkey,
        Err(e) => {
            println!("❌ Invalid client public key: {}", e);
            return Ok(Json(VerifyResponse {
                episode_id: req.episode_id,
                authenticated: false,
                status: "invalid_public_key".to_string(),
            }));
        }
    };
    
    // Verify the signature locally first
    match verify_signature(&client_pubkey, &req.nonce, &req.signature) {
        Ok(true) => println!("✅ Signature verified locally"),
        Ok(false) => {
            println!("❌ Invalid signature");
            return Ok(Json(VerifyResponse {
                episode_id: req.episode_id,
                authenticated: false,
                status: "invalid_signature".to_string(),
            }));
        }
        Err(e) => {
            println!("❌ Signature verification error: {}", e);
            return Ok(Json(VerifyResponse {
                episode_id: req.episode_id,
                authenticated: false,
                status: "signature_verification_error".to_string(),
            }));
        }
    }
    
    // Create SubmitResponse command
    let auth_command = AuthCommand::SubmitResponse {
        signature: req.signature.clone(),
        nonce: req.nonce.clone(),
    };
    
    // Try to create episode message with proper error handling
    let episode_message = match create_episode_message_safe(
        req.episode_id,
        auth_command,
        &state.server_keypair,
        client_pubkey,
    ) {
        Ok(msg) => msg,
        Err(e) => {
            println!("❌ Failed to create episode message: {}", e);
            return Ok(Json(VerifyResponse {
                episode_id: req.episode_id,
                authenticated: false,
                status: format!("episode_message_error: {}", e),
            }));
        }
    };
    
    println!(" Episode message created successfully");
    
    // TODO: Actually submit the transaction to the blockchain
    // For now, simulate success after local verification
    println!("🚀 Transaction would be submitted to blockchain (simulated success)");
    println!("🔗 Explorer: https://explorer-tn10.kaspa.org/addresses/{}", 
             episode_state.public_key.chars().collect::<String>());
    
    // Emit event for authentication attempt
    state.event_emitter.emit(AuthEvent::AuthenticationAttempted {
        episode_id: req.episode_id,
        success: true,
        participant: episode_state.public_key.clone(),
    });
    
    // For demo: Mark as authenticated immediately after local verification
    {
        let mut episodes = state.episodes.lock().unwrap();
        if let Some(episode) = episodes.get_mut(&req.episode_id) {
            episode.authenticated = true;
            let session_token = format!("sess_{}", rand::thread_rng().gen::<u64>());
            episode.session_token = Some(session_token.clone());
            println!("✅ Authentication successful for episode {} - session token: {}", req.episode_id, session_token);
        }
    }
    
    // Broadcast success via WebSocket
    let _ = state.websocket_tx.send(WebSocketMessage::AuthenticationSuccessful {
        episode_id: req.episode_id,
        session_token: "sess_demo".to_string(),
    });
    
    println!("✅ Authentication completed for episode {}", req.episode_id);
    
    Ok(Json(VerifyResponse {
        episode_id: req.episode_id,
        authenticated: true,
        status: "authenticated".to_string(),
    }))
}

// Helper function to parse client public key
fn parse_client_pubkey(pubkey_hex: &str) -> Result<PubKey, String> {
    let pubkey_bytes = hex::decode(pubkey_hex)
        .map_err(|_| "Invalid hex encoding")?;
    
    if pubkey_bytes.len() != 33 {
        return Err("Invalid public key length".to_string());
    }
    
    let secp_pubkey = secp256k1::PublicKey::from_slice(&pubkey_bytes)
        .map_err(|_| "Invalid secp256k1 public key")?;
    
    Ok(PubKey(secp_pubkey))
}

// Helper function to verify signature
fn verify_signature(pubkey: &PubKey, message: &str, signature_hex: &str) -> Result<bool, String> {
    use crate::crypto::signatures::SignatureVerifier;
    Ok(SignatureVerifier::verify(pubkey, message, signature_hex))
}

// Safe wrapper for EpisodeMessage creation
fn create_episode_message_safe(
    episode_id: u64,
    command: AuthCommand,
    server_keypair: &Keypair,
    client_pubkey: PubKey,
) -> Result<EpisodeMessage<SimpleAuth>, String> {
    use std::panic;
    
    // Catch any panics from kdapp
    panic::catch_unwind(|| {
        EpisodeMessage::<SimpleAuth>::new_signed_command(
            episode_id as u32,
            command,
            server_keypair.secret_key(),
            client_pubkey,
        )
    })
    .map_err(|_| "Panic in EpisodeMessage creation".to_string())
}

async fn get_status(
    State(state): State<ServerState>,
    Path(episode_id): Path<u64>,
) -> Result<Json<StatusResponse>, StatusCode> {
    if let Some(episode) = state.episodes.lock().unwrap().get(&episode_id) {
        let status = if episode.authenticated {
            "authenticated"
        } else if episode.challenge.is_some() {
            "challenge_ready"
        } else {
            "pending"
        };
        
        Ok(Json(StatusResponse {
            episode_id,
            authenticated: episode.authenticated,
            challenge: episode.challenge.clone(),
            session_token: episode.session_token.clone(),
            status: status.to_string(),
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// Legacy endpoint for backward compatibility
async fn get_challenge(
    State(state): State<ServerState>,
    Path(episode_id): Path<u64>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Some(episode) = state.episodes.lock().unwrap().get(&episode_id) {
        if let Some(ref challenge) = episode.challenge {
            println!("📡 Legacy challenge request for episode: {}", episode_id);
            
            Ok(Json(serde_json::json!({
                "episode_id": episode_id,
                "challenge": challenge,
                "status": "ready"
            })))
        } else {
            // Generate challenge if none exists
            use rand::Rng;
            let challenge = format!("auth_{}", rand::thread_rng().gen::<u64>());
            
            Ok(Json(serde_json::json!({
                "episode_id": episode_id,
                "challenge": challenge,
                "status": "generated"
            })))
        }
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// Web UI serving
async fn serve_web_ui() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("../../../public/index.html"))
}

// Funding info endpoint
async fn get_funding_info(
    State(state): State<ServerState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let network_id = NetworkId::with_suffix(NetworkType::Testnet, 10);
    let network_prefix = Prefix::from(network_id);
    let kaspa_address = Address::new(network_prefix, Version::PubKey, &state.server_keypair.public_key().serialize()[1..]);
    
    let economic_params = {
        let _manager = state.economic_manager.lock().unwrap();
        serde_json::json!({
            "entry_fee": 1000,
            "challenge_fee": 500,
            "success_reward": 2000,
            "oracle_fee": 100,
            "tournament_buy_in": 10000
        })
    };
    
    Ok(Json(serde_json::json!({
        "funding_address": kaspa_address.to_string(),
        "network": "testnet-10",
        "public_key": hex::encode(state.server_keypair.public_key().serialize()),
        "faucet_url": "https://faucet.kaspanet.io/",
        "transaction_prefix": format!("0x{:08X}", AUTH_PREFIX),
        "transaction_prefix_meaning": "AUTH",
        "economic_parameters": economic_params,
        "note": "Fund this address to test authentication with economic incentives. All transactions use AUTH prefix for blockchain filtering."
    })))
}

// Web wallet management endpoints
async fn get_client_wallet(
    State(state): State<ServerState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Use cached client wallet (no duplicate loading!)
    match state.get_client_wallet() {
        Ok(wallet) => {
            let response = serde_json::json!({
                "success": true,
                "public_key": wallet.get_public_key_hex(),
                "kaspa_address": wallet.get_kaspa_address(),
                "was_created": wallet.was_created,
                "needs_funding": wallet.check_funding_status()
            });
            Ok(Json(response))
        }
        Err(e) => {
            println!("❌ Failed to get client wallet: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_wallet_status(
    State(state): State<ServerState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.get_client_wallet() {
        Ok(wallet) => {
            let response = serde_json::json!({
                "exists": true,
                "kaspa_address": wallet.get_kaspa_address(),
                "needs_funding": wallet.check_funding_status(),
                "was_created": wallet.was_created
            });
            Ok(Json(response))
        }
        Err(_) => {
            let response = serde_json::json!({
                "exists": false,
                "needs_creation": true
            });
            Ok(Json(response))
        }
    }
}

// Reset episodes endpoint - for testing/debugging
async fn reset_episodes(
    State(state): State<ServerState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut episodes = state.episodes.lock().unwrap();
    let count = episodes.len();
    episodes.clear();
    drop(episodes);
    
    println!("🗑️  Reset {} episodes - next authentication will create new episode", count);
    
    let response = serde_json::json!({
        "success": true,
        "episodes_cleared": count,
        "message": "All episodes cleared - next authentication will create new episode"
    });
    
    Ok(Json(response))
}

#[serde_as]
#[derive(Deserialize)]
struct InternalEpisodeAuthenticatedRequest {
    #[serde_as(as = "DisplayFromStr")]
    episode_id: u64,
    challenge: String,
}

async fn internal_episode_authenticated(
    State(state): State<ServerState>,
    Json(req): Json<InternalEpisodeAuthenticatedRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    println!("Received internal authentication confirmation for episode {}", req.episode_id);
    state.mark_episode_authenticated(req.episode_id, req.challenge);
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Episode authentication status updated"
    })))
}

// TODO: Helper function to submit episode to blockchain (disabled for now)
// async fn submit_episode_to_blockchain(...) { ... }
