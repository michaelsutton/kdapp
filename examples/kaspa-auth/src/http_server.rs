use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use secp256k1::Keypair;
use kdapp::{
    engine::EpisodeMessage,
    generator::TransactionGenerator,
    proxy::connect_client,
    pki::PubKey,
};
use kaspa_addresses::{Address, Prefix, Version};
use kaspa_consensus_core::{network::NetworkId, tx::{TransactionOutpoint, UtxoEntry}};
use kaspa_wrpc_client::prelude::*;
use kaspa_rpc_core::api::rpc::RpcApi;
use crate::{
    simple_auth_episode::SimpleAuth,
    auth_commands::AuthCommand,
    episode_runner::{AUTH_PATTERN, AUTH_PREFIX},
};

// HTTP Request/Response types
#[derive(Serialize, Deserialize)]
pub struct StartAuthRequest {
    pub public_key: String, // hex-encoded public key
}

#[derive(Serialize, Deserialize)]
pub struct StartAuthResponse {
    pub episode_id: u64,
    pub status: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChallengeResponse {
    pub episode_id: u64,
    pub challenge: Option<String>,
    pub status: String,
}

#[derive(Serialize, Deserialize)]
pub struct VerifyRequest {
    pub episode_id: u64,
    pub signature: String,
    pub nonce: String,
}

#[derive(Serialize, Deserialize)]
pub struct VerifyResponse {
    pub episode_id: u64,
    pub authenticated: bool,
    pub session_token: Option<String>,
    pub status: String,
}

#[derive(Serialize, Deserialize)]
pub struct AuthStatusResponse {
    pub episode_id: u64,
    pub authenticated: bool,
    pub session_token: Option<String>,
    pub challenge: Option<String>,
    pub status: String,
}

#[derive(Serialize, Deserialize)]
pub struct RequestChallengeRequest {
    pub episode_id: u64,
    pub public_key: String, // hex-encoded public key of requester
}

#[derive(Serialize, Deserialize)]
pub struct RequestChallengeResponse {
    pub episode_id: u64,
    pub status: String,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct SignChallengeRequest {
    pub challenge: String,
    pub private_key: String, // hex-encoded private key
}

#[derive(Serialize, Deserialize)]
pub struct SignChallengeResponse {
    pub challenge: String,
    pub signature: String,
    pub public_key: String,
}

// Shared state for the HTTP server
#[derive(Clone)]
pub struct HttpServerState {
    pub kaspa_signer: Keypair,
    pub network: NetworkId,
    pub episodes: Arc<Mutex<HashMap<u64, SimpleAuth>>>,
    pub utxos: Arc<Mutex<HashMap<u64, (TransactionOutpoint, UtxoEntry)>>>,
    pub kaspa_addr: Address,
}

impl HttpServerState {
    pub fn new(kaspa_signer: Keypair, network: NetworkId) -> Self {
        let kaspa_addr = Address::new(
            Prefix::Testnet,
            Version::PubKey,
            &kaspa_signer.x_only_public_key().0.serialize(),
        );
        
        Self {
            kaspa_signer,
            network,
            episodes: Arc::new(Mutex::new(HashMap::new())),
            utxos: Arc::new(Mutex::new(HashMap::new())),
            kaspa_addr,
        }
    }
}

// HTTP Handlers

/// POST /auth/start - Creates a new authentication episode on the blockchain
pub async fn start_auth(
    State(state): State<HttpServerState>,
    Json(req): Json<StartAuthRequest>,
) -> Result<Json<StartAuthResponse>, StatusCode> {
    log::info!("🚀 Starting new authentication episode");
    
    // Parse the public key
    let pubkey_bytes = hex::decode(&req.public_key)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let pubkey = secp256k1::PublicKey::from_slice(&pubkey_bytes)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let auth_pubkey = PubKey(pubkey);
    
    log::info!("🔑 Auth public key: {}", auth_pubkey);
    
    // Connect to Kaspa network
    let kaspad = connect_client(state.network, None).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Get UTXOs for transaction funding
    log::info!("🔍 Fetching UTXOs for address: {}", state.kaspa_addr);
    let entries = kaspad.get_utxos_by_addresses(vec![state.kaspa_addr.clone()]).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if entries.is_empty() {
        log::error!("No UTXOs found for address: {}", state.kaspa_addr);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    
    let utxo = entries.first().map(|entry| {
        (
            TransactionOutpoint::from(entry.outpoint.clone()),
            UtxoEntry::from(entry.utxo_entry.clone()),
        )
    }).unwrap();
    
    log::info!("✅ UTXO found: {}", utxo.0);
    
    // Generate episode ID
    let episode_id = rand::random::<u32>() as u64;
    
    // Create NewEpisode transaction
    let new_episode = EpisodeMessage::<SimpleAuth>::NewEpisode {
        episode_id: episode_id as u32,
        participants: vec![auth_pubkey],
    };
    
    // Create transaction generator
    let generator = TransactionGenerator::new(state.kaspa_signer, AUTH_PATTERN, AUTH_PREFIX);
    let tx = generator.build_command_transaction(utxo, &state.kaspa_addr, &new_episode, 5000);
    log::info!("🚀 Submitting NewEpisode transaction: {}", tx.id());
    
    // Submit to blockchain
    let _res = kaspad.submit_transaction(tx.as_ref().into(), false).await
        .map_err(|e| {
            log::error!("Failed to submit transaction: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Store the next UTXO for future transactions
    let next_utxo = kdapp::generator::get_first_output_utxo(&tx);
    state.utxos.lock().unwrap().insert(episode_id, next_utxo);
    
    log::info!("✅ Episode {} initialized on blockchain!", episode_id);
    
    Ok(Json(StartAuthResponse {
        episode_id: episode_id,
        status: "episode_created".to_string(),
    }))
}

/// GET /auth/challenge/{episode_id} - Reads challenge from episode state
pub async fn get_challenge(
    State(state): State<HttpServerState>,
    Path(episode_id): Path<u64>,
) -> Result<Json<ChallengeResponse>, StatusCode> {
    log::info!("🔍 Getting challenge for episode: {}", episode_id);
    
    // Check if episode exists in our state
    let episodes = state.episodes.lock().unwrap();
    if let Some(episode) = episodes.get(&episode_id) {
        let challenge = episode.challenge.clone();
        let status = if episode.is_authenticated {
            "authenticated"
        } else if challenge.is_some() {
            "challenge_ready"
        } else {
            "pending"
        };
        
        Ok(Json(ChallengeResponse {
            episode_id,
            challenge,
            status: status.to_string(),
        }))
    } else {
        // Episode not found in local state
        log::warn!("Episode {} not found in local state", episode_id);
        Ok(Json(ChallengeResponse {
            episode_id,
            challenge: None,
            status: "episode_not_found".to_string(),
        }))
    }
}

/// POST /auth/verify - Submits authentication response to blockchain
pub async fn verify_auth(
    State(state): State<HttpServerState>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, StatusCode> {
    log::info!("📤 Verifying authentication for episode: {}", req.episode_id);
    
    // Get the UTXO for this episode
    let utxo = {
        let utxos = state.utxos.lock().unwrap();
        utxos.get(&req.episode_id).cloned()
    };
    
    let utxo = utxo.ok_or(StatusCode::NOT_FOUND)?;
    
    // Connect to Kaspa network
    let kaspad = connect_client(state.network, None).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Create SubmitResponse command
    let auth_command = AuthCommand::SubmitResponse {
        signature: req.signature,
        nonce: req.nonce,
    };
    
    // For now, we'll use a dummy public key - in a real implementation,
    // this would be extracted from the HTTP request authentication
    let dummy_pubkey = PubKey(state.kaspa_signer.public_key());
    
    let step = EpisodeMessage::<SimpleAuth>::new_signed_command(
        req.episode_id as u32,
        auth_command,
        state.kaspa_signer.secret_key(),
        dummy_pubkey,
    );
    
    // Create transaction generator
    let generator = TransactionGenerator::new(state.kaspa_signer, AUTH_PATTERN, AUTH_PREFIX);
    let tx = generator.build_command_transaction(utxo, &state.kaspa_addr, &step, 5000);
    log::info!("🚀 Submitting SubmitResponse transaction: {}", tx.id());
    
    // Submit to blockchain
    let _res = kaspad.submit_transaction(tx.as_ref().into(), false).await
        .map_err(|e| {
            log::error!("Failed to submit transaction: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Update UTXO for next transaction
    let next_utxo = kdapp::generator::get_first_output_utxo(&tx);
    state.utxos.lock().unwrap().insert(req.episode_id, next_utxo);
    
    log::info!("✅ SubmitResponse transaction submitted to blockchain!");
    
    // Check episode state (this would be updated by the kdapp engine)
    let episodes = state.episodes.lock().unwrap();
    let (authenticated, session_token) = if let Some(episode) = episodes.get(&req.episode_id) {
        (episode.is_authenticated, episode.session_token.clone())
    } else {
        (false, None)
    };
    
    Ok(Json(VerifyResponse {
        episode_id: req.episode_id,
        authenticated,
        session_token,
        status: if authenticated { "authenticated" } else { "pending" }.to_string(),
    }))
}

/// GET /auth/status/{episode_id} - Get complete authentication status
pub async fn get_auth_status(
    State(state): State<HttpServerState>,
    Path(episode_id): Path<u64>,
) -> Result<Json<AuthStatusResponse>, StatusCode> {
    log::info!("🔍 Getting complete auth status for episode: {}", episode_id);
    
    // Check if episode exists in our state
    let episodes = state.episodes.lock().unwrap();
    if let Some(episode) = episodes.get(&episode_id) {
        let status = if episode.is_authenticated {
            "authenticated"
        } else if episode.challenge.is_some() {
            "challenge_ready"
        } else {
            "pending"
        };
        
        Ok(Json(AuthStatusResponse {
            episode_id,
            authenticated: episode.is_authenticated,
            session_token: episode.session_token.clone(),
            challenge: episode.challenge.clone(),
            status: status.to_string(),
        }))
    } else {
        // Episode not found in local state
        log::warn!("Episode {} not found in local state", episode_id);
        Ok(Json(AuthStatusResponse {
            episode_id,
            authenticated: false,
            session_token: None,
            challenge: None,
            status: "episode_not_found".to_string(),
        }))
    }
}

/// POST /auth/request-challenge - Send RequestChallenge command to blockchain
pub async fn request_challenge(
    State(state): State<HttpServerState>,
    Json(req): Json<RequestChallengeRequest>,
) -> Result<Json<RequestChallengeResponse>, StatusCode> {
    log::info!("📨 Requesting challenge for episode: {}", req.episode_id);
    
    // Parse the public key
    let pubkey_bytes = hex::decode(&req.public_key)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let pubkey = secp256k1::PublicKey::from_slice(&pubkey_bytes)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let auth_pubkey = PubKey(pubkey);
    
    // Get the UTXO for this episode (if we stored it)
    let utxo = {
        let utxos = state.utxos.lock().unwrap();
        utxos.get(&req.episode_id).cloned()
    };
    
    let utxo = utxo.ok_or(StatusCode::NOT_FOUND)?;
    
    // Connect to Kaspa network
    let kaspad = connect_client(state.network, None).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Create RequestChallenge command
    let auth_command = AuthCommand::RequestChallenge;
    
    let step = EpisodeMessage::<SimpleAuth>::new_signed_command(
        req.episode_id as u32,
        auth_command,
        state.kaspa_signer.secret_key(),
        auth_pubkey,
    );
    
    // Create transaction generator
    let generator = TransactionGenerator::new(state.kaspa_signer, AUTH_PATTERN, AUTH_PREFIX);
    let tx = generator.build_command_transaction(utxo, &state.kaspa_addr, &step, 5000);
    log::info!("🚀 Submitting RequestChallenge transaction: {}", tx.id());
    
    // Submit to blockchain
    let _res = kaspad.submit_transaction(tx.as_ref().into(), false).await
        .map_err(|e| {
            log::error!("Failed to submit RequestChallenge transaction: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Update UTXO for next transaction
    let next_utxo = kdapp::generator::get_first_output_utxo(&tx);
    state.utxos.lock().unwrap().insert(req.episode_id, next_utxo);
    
    log::info!("✅ RequestChallenge transaction submitted to blockchain!");
    
    Ok(Json(RequestChallengeResponse {
        episode_id: req.episode_id,
        status: "challenge_requested".to_string(),
        message: "RequestChallenge command sent to blockchain. Check status endpoint for challenge.".to_string(),
    }))
}

/// POST /auth/sign-challenge - Sign a challenge with private key (REAL CRYPTOGRAPHY)
pub async fn sign_challenge(
    Json(req): Json<SignChallengeRequest>,
) -> Result<Json<SignChallengeResponse>, StatusCode> {
    log::info!("✍️ Signing challenge: {}", req.challenge);
    
    // Parse private key from hex
    let private_key_bytes = hex::decode(&req.private_key)
        .map_err(|e| {
            log::error!("Failed to decode private key: {}", e);
            StatusCode::BAD_REQUEST
        })?;
    
    let secp = secp256k1::Secp256k1::new();
    let secret_key = secp256k1::SecretKey::from_slice(&private_key_bytes)
        .map_err(|e| {
            log::error!("Invalid private key: {}", e);
            StatusCode::BAD_REQUEST
        })?;
    
    let keypair = secp256k1::Keypair::from_secret_key(&secp, &secret_key);
    let public_key = keypair.public_key();
    
    // Sign the challenge using kdapp's signing function
    let message = kdapp::pki::to_message(&req.challenge);
    let signature = kdapp::pki::sign_message(&secret_key, &message);
    let signature_hex = hex::encode(signature.0.serialize_der());
    
    log::info!("✅ Challenge signed successfully");
    log::info!("📝 Signature: {}", signature_hex);
    log::info!("🔑 Public key: {}", hex::encode(public_key.serialize()));
    
    Ok(Json(SignChallengeResponse {
        challenge: req.challenge,
        signature: signature_hex,
        public_key: hex::encode(public_key.serialize()),
    }))
}

/// Create the HTTP server router
pub fn create_router(state: HttpServerState) -> Router {
    Router::new()
        .route("/auth/start", post(start_auth))
        .route("/auth/request-challenge", post(request_challenge))
        .route("/auth/sign-challenge", post(sign_challenge))
        .route("/auth/challenge/{episode_id}", get(get_challenge))
        .route("/auth/verify", post(verify_auth))
        .route("/auth/status/{episode_id}", get(get_auth_status))
        .with_state(state)
}

/// Start the HTTP server with full blockchain integration
pub async fn start_http_server(kaspa_signer: Keypair, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let network = NetworkId::with_suffix(kaspa_consensus_core::network::NetworkType::Testnet, 10);
    
    // Show funding information first (like client command does)
    let kaspa_addr = Address::new(
        Prefix::Testnet,
        Version::PubKey,
        &kaspa_signer.x_only_public_key().0.serialize(),
    );
    
    println!("💰 Server Funding Address: {}", kaspa_addr);
    println!("🔑 Private Key: {}", hex::encode(kaspa_signer.secret_key().secret_bytes()));
    println!();
    println!("📋 Server Funding Instructions:");
    println!("1. Send testnet KAS to: {}", kaspa_addr);
    println!("2. Get testnet funds from: https://faucet.kaspanet.io");
    println!("3. For authentication services: users pay transaction fees to this address");
    println!();
    println!("🚀 After funding, HTTP server will process authentication transactions");
    println!();
    
    // Create shared state for HTTP server and blockchain listener
    let state = HttpServerState::new(kaspa_signer, network);
    
    // Start blockchain listener in background
    let episodes_for_listener = state.episodes.clone();
    let kaspa_signer_for_listener = kaspa_signer;
    let network_for_listener = network;
    
    tokio::spawn(async move {
        if let Err(e) = start_blockchain_listener(kaspa_signer_for_listener, network_for_listener, episodes_for_listener).await {
            log::error!("Blockchain listener error: {}", e);
        }
    });
    
    // Start HTTP server
    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    
    println!("🌐 HTTP Auth Server listening on http://127.0.0.1:{}", port);
    println!("📡 Endpoints:");
    println!("  POST /auth/start - Create new authentication episode");
    println!("  POST /auth/request-challenge - Send RequestChallenge to blockchain");
    println!("  POST /auth/sign-challenge - Sign challenge with private key (REAL CRYPTO)");
    println!("  GET /auth/challenge/{{episode_id}} - Get challenge for episode");
    println!("  POST /auth/verify - Submit authentication response");
    println!("  GET /auth/status/{{episode_id}} - Get complete authentication status");
    println!("🔗 Blockchain listener: Active and processing episode updates");
    
    axum::serve(listener, app).await?;
    Ok(())
}

/// Start blockchain listener to process episode updates
async fn start_blockchain_listener(
    kaspa_signer: Keypair,
    network: NetworkId,
    episodes: Arc<Mutex<HashMap<u64, SimpleAuth>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::{mpsc::channel, Arc as StdArc, atomic::AtomicBool};
    use kdapp::{engine::{self, Engine}, episode::EpisodeEventHandler};
    
    log::info!("🔗 Starting blockchain listener for episode updates...");
    
    // Create channels for engine communication
    let (sender, receiver) = channel();
    let exit_signal = StdArc::new(AtomicBool::new(false));
    
    // Create episode event handler that updates HTTP server state
    struct HttpEpisodeHandler {
        episodes: Arc<Mutex<HashMap<u64, SimpleAuth>>>,
    }
    
    impl EpisodeEventHandler<SimpleAuth> for HttpEpisodeHandler {
        fn on_initialize(&self, episode_id: kdapp::episode::EpisodeId, episode: &SimpleAuth) {
            log::info!("[HTTP Server] Episode {} initialized", episode_id);
            self.episodes.lock().unwrap().insert(episode_id as u64, episode.clone());
        }
        
        fn on_command(&self, episode_id: kdapp::episode::EpisodeId, episode: &SimpleAuth, 
                      cmd: &crate::auth_commands::AuthCommand, _authorization: Option<kdapp::pki::PubKey>, 
                      _metadata: &kdapp::episode::PayloadMetadata) {
            log::info!("[HTTP Server] Episode {} command processed: {:?}", episode_id, cmd);
            self.episodes.lock().unwrap().insert(episode_id as u64, episode.clone());
            
            match cmd {
                crate::auth_commands::AuthCommand::RequestChallenge => {
                    if let Some(ref challenge) = episode.challenge {
                        log::info!("[HTTP Server] Challenge generated for episode {}: {}", episode_id, challenge);
                    }
                }
                crate::auth_commands::AuthCommand::SubmitResponse { .. } => {
                    if episode.is_authenticated {
                        log::info!("[HTTP Server] Authentication successful for episode {}", episode_id);
                    }
                }
            }
        }
        
        fn on_rollback(&self, episode_id: kdapp::episode::EpisodeId, episode: &SimpleAuth) {
            log::info!("[HTTP Server] Episode {} rolled back", episode_id);
            self.episodes.lock().unwrap().insert(episode_id as u64, episode.clone());
        }
    }
    
    // Start kdapp engine
    let mut engine = Engine::<SimpleAuth, HttpEpisodeHandler>::new(receiver);
    let handler = HttpEpisodeHandler { episodes };
    
    let engine_task = tokio::task::spawn_blocking(move || {
        engine.start(vec![handler]);
    });
    
    // Connect to Kaspa network and start listening
    let kaspad = connect_client(network, None).await?;
    let engines = std::iter::once((AUTH_PREFIX, (AUTH_PATTERN, sender))).collect();
    
    // Start proxy listener
    tokio::select! {
        _ = kdapp::proxy::run_listener(kaspad, engines, exit_signal) => {
            log::info!("Blockchain listener stopped");
        }
        _ = engine_task => {
            log::info!("Engine task completed");
        }
    }
    
    Ok(())
}