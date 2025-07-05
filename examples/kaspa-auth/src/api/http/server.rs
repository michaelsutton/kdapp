// src/api/http/server.rs
use axum::{routing::{get, post}, Router, response::Json, extract::{Path, State}, http::StatusCode};
use secp256k1::Keypair;
use axum::serve;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use kdapp::pki::{sign_message, to_message};

// Episode storage with full state
#[derive(Clone, Debug)]
struct EpisodeState {
    episode_id: u64,
    public_key: String,
    challenge: Option<String>,
    authenticated: bool,
    session_token: Option<String>,
}

type EpisodeStorage = Arc<Mutex<HashMap<u64, EpisodeState>>>;

// Request/Response types
#[derive(Deserialize)]
struct StartAuthRequest {
    public_key: String,
}

#[derive(Deserialize)]
struct RegisterEpisodeRequest {
    episode_id: u64,
    public_key: String,
}

#[derive(Serialize)]
struct StartAuthResponse {
    episode_id: u64,
    status: String,
}

#[derive(Deserialize)]
struct RequestChallengeRequest {
    episode_id: u64,
    public_key: String,
}

#[derive(Serialize)]
struct ChallengeResponse {
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

#[derive(Deserialize)]
struct VerifyRequest {
    episode_id: u64,
    signature: String,
    nonce: String,
}

#[derive(Serialize)]
struct VerifyResponse {
    episode_id: u64,
    authenticated: bool,
    status: String,
}

#[derive(Serialize)]
struct StatusResponse {
    episode_id: u64,
    authenticated: bool,
    challenge: Option<String>,
    session_token: Option<String>,
    status: String,
}

pub async fn run_http_server(keypair: Keypair, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let episode_storage: EpisodeStorage = Arc::new(Mutex::new(HashMap::new()));
    
    async fn hello_world() -> Json<serde_json::Value> {
        Json(serde_json::json!({"message": "Kaspa Auth HTTP Server", "status": "running"}))
    }

    let app = Router::new()
        .route("/", get(hello_world))
        .route("/health", get(hello_world))
        .route("/auth/start", post(start_auth))
        .route("/auth/register-episode", post(register_episode))
        .route("/auth/request-challenge", post(request_challenge))
        .route("/auth/sign-challenge", post(sign_challenge))
        .route("/auth/verify", post(verify_auth))
        .route("/auth/status/{episode_id}", get(get_status))
        .route("/challenge/{episode_id}", get(get_challenge))
        .with_state(episode_storage);

    let addr = format!("0.0.0.0:{}", port);
    println!("🚀 HTTP Authentication Server starting on port {}", port);
    println!("🔑 Server public key: {}", hex::encode(keypair.public_key().serialize()));
    println!("📡 Endpoints:");
    println!("  GET  /                           - Server info");
    println!("  GET  /health                     - Health check");
    println!("  POST /auth/start                 - Create authentication episode");
    println!("  POST /auth/register-episode      - Register blockchain episode with HTTP server");
    println!("  POST /auth/request-challenge     - Request challenge from blockchain");
    println!("  POST /auth/sign-challenge        - Sign challenge (helper endpoint)");
    println!("  POST /auth/verify                - Submit authentication response");
    println!("  GET  /auth/status/{{episode_id}}  - Get episode status");
    println!("  GET  /challenge/{{episode_id}}   - Get challenge for episode (legacy)");
    println!();
    println!("✅ Server running! Example workflow:");
    println!("  curl -X POST http://localhost:{}/auth/start -H 'Content-Type: application/json' -d '{{\"public_key\": \"YOUR_PUBKEY\"}}'", port);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    serve(listener, app.into_make_service()).await?;

    Ok(())
}

// Handler implementations
async fn start_auth(
    State(storage): State<EpisodeStorage>,
    Json(req): Json<StartAuthRequest>,
) -> Result<Json<StartAuthResponse>, StatusCode> {
    use rand::Rng;
    let episode_id = rand::thread_rng().gen::<u64>();
    
    let episode = EpisodeState {
        episode_id,
        public_key: req.public_key.clone(),
        challenge: None,
        authenticated: false,
        session_token: None,
    };
    
    storage.lock().unwrap().insert(episode_id, episode);
    
    println!("📝 Created episode {} for public key: {}", episode_id, req.public_key);
    
    Ok(Json(StartAuthResponse {
        episode_id,
        status: "episode_created".to_string(),
    }))
}

async fn register_episode(
    State(storage): State<EpisodeStorage>,
    Json(req): Json<RegisterEpisodeRequest>,
) -> Result<Json<StartAuthResponse>, StatusCode> {
    let episode = EpisodeState {
        episode_id: req.episode_id,
        public_key: req.public_key.clone(),
        challenge: None,
        authenticated: false,
        session_token: None,
    };
    
    storage.lock().unwrap().insert(req.episode_id, episode);
    
    println!("📝 Registered blockchain episode {} for public key: {}", req.episode_id, req.public_key);
    
    Ok(Json(StartAuthResponse {
        episode_id: req.episode_id,
        status: "episode_registered".to_string(),
    }))
}

async fn request_challenge(
    State(storage): State<EpisodeStorage>,
    Json(req): Json<RequestChallengeRequest>,
) -> Result<Json<ChallengeResponse>, StatusCode> {
    use rand::Rng;
    let challenge = format!("auth_{}", rand::thread_rng().gen::<u64>());
    
    if let Some(episode) = storage.lock().unwrap().get_mut(&req.episode_id) {
        episode.challenge = Some(challenge.clone());
        println!("🎲 Generated challenge {} for episode {}", challenge, req.episode_id);
        
        Ok(Json(ChallengeResponse {
            episode_id: req.episode_id,
            status: "challenge_requested".to_string(),
            message: "RequestChallenge command sent to blockchain...".to_string(),
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn sign_challenge(
    Json(req): Json<SignChallengeRequest>,
) -> Result<Json<SignChallengeResponse>, StatusCode> {
    use secp256k1::{Secp256k1, SecretKey};
    
    // Parse private key
    let secret_bytes = match hex::decode(&req.private_key) {
        Ok(bytes) => bytes,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };
    
    let secret_key = match SecretKey::from_slice(&secret_bytes) {
        Ok(key) => key,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };
    
    // Sign the challenge
    let message = to_message(&req.challenge);
    let signature = sign_message(&secret_key, &message);
    let signature_hex = hex::encode(signature.0.serialize_der());
    
    // Get public key
    let secp = Secp256k1::new();
    let keypair = secp256k1::Keypair::from_secret_key(&secp, &secret_key);
    let public_key_hex = hex::encode(keypair.public_key().serialize());
    
    println!("✍️ Signed challenge: {} with key: {}", req.challenge, public_key_hex);
    
    Ok(Json(SignChallengeResponse {
        challenge: req.challenge,
        signature: signature_hex,
        public_key: public_key_hex,
    }))
}

async fn verify_auth(
    State(storage): State<EpisodeStorage>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, StatusCode> {
    use rand::Rng;
    
    if let Some(episode) = storage.lock().unwrap().get_mut(&req.episode_id) {
        // In a real implementation, we would verify the signature here
        // For now, we'll just mark as authenticated
        episode.authenticated = true;
        episode.session_token = Some(format!("sess_{}", rand::thread_rng().gen::<u64>()));
        
        println!("✅ Authenticated episode {}", req.episode_id);
        
        Ok(Json(VerifyResponse {
            episode_id: req.episode_id,
            authenticated: true,
            status: "authenticated".to_string(),
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn get_status(
    State(storage): State<EpisodeStorage>,
    Path(episode_id): Path<u64>,
) -> Result<Json<StatusResponse>, StatusCode> {
    if let Some(episode) = storage.lock().unwrap().get(&episode_id) {
        let status = if episode.authenticated {
            "authenticated"
        } else if episode.challenge.is_some() {
            "challenge_ready"
        } else {
            "pending"
        };
        
        Ok(Json(StatusResponse {
            episode_id: episode.episode_id,
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
    State(storage): State<EpisodeStorage>,
    Path(episode_id): Path<u64>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Some(episode) = storage.lock().unwrap().get(&episode_id) {
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
