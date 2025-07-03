use kdapp::{
    engine::{self},
    episode::{EpisodeEventHandler, EpisodeId, PayloadMetadata},
    generator::{PatternType, PrefixType, TransactionGenerator},
    pki::PubKey,
    proxy::{self, connect_client},
};
use kaspa_consensus_core::network::{NetworkId, NetworkType};
use std::sync::{mpsc::channel, Arc, atomic::AtomicBool, Mutex};
use std::collections::HashMap;
use secp256k1::Keypair;
use log::{info, warn, error};
use serde::{Serialize, Deserialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use rand::Rng;

use crate::{SimpleAuth, auth_commands::AuthCommand};

// Define unique pattern and prefix for auth transactions
// Pattern: specific byte positions that must match to reduce node overhead
pub const AUTH_PATTERN: PatternType = [
    (7, 0), (32, 1), (45, 0), (99, 1), (113, 0), 
    (126, 1), (189, 0), (200, 1), (211, 0), (250, 1)
];

// Unique prefix to identify auth transactions (chosen to avoid conflicts)
pub const AUTH_PREFIX: PrefixType = 0x41555448; // "AUTH" in hex

/// Event handler for authentication episodes
pub struct AuthEventHandler {
    pub name: String,
    pub episode_challenges: Arc<Mutex<HashMap<u64, String>>>,
}

impl AuthEventHandler {
    pub fn new(name: String, episode_challenges: Arc<Mutex<HashMap<u64, String>>>) -> Self {
        Self { name, episode_challenges }
    }
}

impl EpisodeEventHandler<SimpleAuth> for AuthEventHandler {
    fn on_initialize(&self, episode_id: EpisodeId, episode: &SimpleAuth) {
        info!("[{}] Episode {} initialized with owner: {:?}", 
              self.name, episode_id, episode.owner);
    }

    fn on_command(&self, episode_id: EpisodeId, episode: &SimpleAuth, 
                  cmd: &AuthCommand, authorization: Option<PubKey>, 
                  _metadata: &PayloadMetadata) {
        match cmd {
            AuthCommand::RequestChallenge => {
                info!("[{}] Episode {}: Challenge requested by {:?}", 
                      self.name, episode_id, authorization);
                if let Some(challenge) = &episode.challenge {
                    info!("[{}] Episode {}: Challenge generated: {}", 
                          self.name, episode_id, challenge);
                    // Store challenge for HTTP coordination
                    if let Ok(mut challenges) = self.episode_challenges.lock() {
                        challenges.insert(episode_id as u64, challenge.clone());
                    }
                }
            }
            AuthCommand::SubmitResponse { signature, nonce } => {
                info!("[{}] Episode {}: Response submitted with nonce: {}", 
                      self.name, episode_id, nonce);
                if episode.is_authenticated {
                    info!("[{}] Episode {}: ✅ Authentication successful!", 
                          self.name, episode_id);
                } else {
                    warn!("[{}] Episode {}: ❌ Authentication failed - invalid signature", 
                          self.name, episode_id);
                }
            }
        }
    }

    fn on_rollback(&self, episode_id: EpisodeId, _episode: &SimpleAuth) {
        warn!("[{}] Episode {} rolled back due to DAG reorg", self.name, episode_id);
    }
}

/// Configuration for the auth server
pub struct AuthServerConfig {
    pub signer: Keypair,
    pub network: NetworkId,
    pub rpc_url: Option<String>,
    pub name: String,
    pub http_port: u16,
}

/// Simple HTTP coordination structures
#[derive(Serialize, Deserialize)]
pub struct ChallengeRequest {
    pub client_pubkey: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChallengeResponse {
    pub challenge: String,
    pub success: bool,
}

#[derive(Serialize, Deserialize)]
pub struct AuthRequest {
    pub signature: String,
    pub nonce: String,
    pub client_pubkey: String,
}

#[derive(Serialize, Deserialize)]
pub struct AuthResponse {
    pub authenticated: bool,
    pub session_token: Option<String>,
}

/// Simple coordination state
pub struct CoordinationState {
    pub challenges: Arc<Mutex<HashMap<String, String>>>, // pubkey -> challenge  
    pub episode_challenges: Arc<Mutex<HashMap<u64, String>>>, // episode_id -> challenge
}

impl AuthServerConfig {
    pub fn new(signer: Keypair, name: String, rpc_url: Option<String>) -> Self {
        Self {
            signer,
            network: NetworkId::with_suffix(NetworkType::Testnet, 10),
            rpc_url,
            name,
            http_port: 8080,
        }
    }
}

/// Run the authentication server
pub async fn run_auth_server(config: AuthServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("🎯 Starting Kaspa Auth Server: {}", config.name);
    info!("📡 Connecting to network: {:?}", config.network);

    // 1. Connect to Kaspa network
    let kaspad = connect_client(config.network, config.rpc_url.clone()).await?;
    info!("✅ Connected to Kaspa node");

    // 2. Set up engine channel and episode challenges storage
    let (sender, receiver) = channel();
    let episode_challenges = Arc::new(Mutex::new(HashMap::new()));

    // 3. Create and start engine
    let mut engine = engine::Engine::<SimpleAuth, AuthEventHandler>::new(receiver);
    let event_handler = AuthEventHandler::new(config.name.clone(), episode_challenges.clone());
    
    let engine_task = tokio::task::spawn_blocking(move || {
        info!("🚀 Starting episode engine");
        engine.start(vec![event_handler]);
    });

    // 4. Set up exit signal for graceful shutdown
    let exit_signal = Arc::new(AtomicBool::new(false));
    let exit_signal_clone = exit_signal.clone();
    
    // Handle Ctrl+C for graceful shutdown
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to install CTRL+C signal handler");
        info!("🛑 Shutdown signal received");
        exit_signal_clone.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    // 5. Set up engines map for proxy
    let engines = std::iter::once((AUTH_PREFIX, (AUTH_PATTERN, sender))).collect();

    info!("👂 Listening for auth transactions with prefix: 0x{:08X}", AUTH_PREFIX);
    info!("🔍 Using pattern: {:?}", AUTH_PATTERN);

    // 6. Start simple HTTP coordination server
    let coordination_state = CoordinationState {
        challenges: Arc::new(Mutex::new(HashMap::new())),
        episode_challenges: episode_challenges.clone(),
    };
    
    let http_addr = format!("127.0.0.1:{}", config.http_port);
    info!("🌐 Starting HTTP coordination server on {}", http_addr);
    
    let episode_challenges_clone = coordination_state.episode_challenges.clone();
    let exit_signal_http = exit_signal.clone();
    
    tokio::spawn(async move {
        if let Err(e) = run_simple_http_server(&http_addr, episode_challenges_clone, exit_signal_http).await {
            error!("HTTP server error: {}", e);
        }
    });

    // 7. Start proxy listener
    proxy::run_listener(kaspad, engines, exit_signal).await;
    
    // Wait for engine to finish
    let _ = engine_task.await?;
    
    info!("✅ Auth server shutdown gracefully");

    Ok(())
}

/// Create a transaction generator for auth commands
pub fn create_auth_generator(signer: Keypair, _network: NetworkId) -> TransactionGenerator {
    TransactionGenerator::new(
        signer,
        AUTH_PATTERN,
        AUTH_PREFIX,
    )
}

/// Simple HTTP server for coordination
async fn run_simple_http_server(
    addr: &str, 
    episode_challenges: Arc<Mutex<HashMap<u64, String>>>,
    exit_signal: Arc<AtomicBool>
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(addr).await?;
    info!("HTTP coordination server listening on {}", addr);
    
    while !exit_signal.load(std::sync::atomic::Ordering::Relaxed) {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let episode_challenges_clone = episode_challenges.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_http_request(stream, episode_challenges_clone).await {
                                error!("Error handling HTTP request: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Error accepting connection: {}", e);
                    }
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                // Check exit signal periodically
            }
        }
    }
    
    Ok(())
}

/// Handle individual HTTP requests
async fn handle_http_request(
    mut stream: TcpStream,
    episode_challenges: Arc<Mutex<HashMap<u64, String>>>
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0; 1024];
    let n = stream.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..n]);
    
    // Parse HTTP request (very basic parsing) - NOW ONLY FOR COORDINATION
    if request.starts_with("GET /status") {
        // Simple status endpoint for coordination
        let response = r#"{"status": "kdapp auth server running", "blockchain": "active"}"#;
        let http_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            response.len(),
            response
        );
        
        stream.write_all(http_response.as_bytes()).await?;
        return Ok(());
    } else if request.starts_with("GET /challenge/") {
        // Extract episode ID from URL path
        if let Some(path_start) = request.find("GET /challenge/") {
            let path = &request[path_start + 15..];
            if let Some(space_pos) = path.find(' ') {
                let episode_id_str = &path[..space_pos];
                if let Ok(episode_id) = episode_id_str.parse::<u64>() {
                    // Get real challenge from episode state  
                    let challenge_response = {
                        if let Ok(challenges) = episode_challenges.lock() {
                            if let Some(challenge) = challenges.get(&episode_id) {
                                format!(r#"{{"episode_id": {}, "challenge": "{}", "available": true}}"#, episode_id, challenge)
                            } else {
                                format!(r#"{{"episode_id": {}, "error": "Challenge not yet available", "available": false}}"#, episode_id)
                            }
                        } else {
                            format!(r#"{{"episode_id": {}, "error": "Server error", "available": false}}"#, episode_id)
                        }
                    };
                    
                    let http_response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        challenge_response.len(),
                        challenge_response
                    );
                    
                    stream.write_all(http_response.as_bytes()).await?;
                    return Ok(());
                }
            }
        }
    }
    
    // Default 404 response
    let not_found = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
    stream.write_all(not_found.as_bytes()).await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1::{Secp256k1, SecretKey};

    #[test]
    fn test_auth_pattern_uniqueness() {
        // Ensure AUTH_PREFIX is unique (not conflicting with tictactoe)
        const TICTACTOE_PREFIX: PrefixType = 0x54544F45; // "TTOE"
        assert_ne!(AUTH_PREFIX, TICTACTOE_PREFIX);
    }

    #[test]
    fn test_event_handler_creation() {
        let test_challenges = Arc::new(Mutex::new(HashMap::new()));
        let handler = AuthEventHandler::new("test-server".to_string(), test_challenges);
        assert_eq!(handler.name, "test-server");
    }

    #[test]
    fn test_config_creation() {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::new(&mut rand::thread_rng());
        let keypair = Keypair::from_secret_key(&secp, &secret_key);
        
        let config = AuthServerConfig::new(keypair, "test".to_string(), None);
        assert_eq!(config.name, "test");
        assert_eq!(config.network, NetworkId::with_suffix(NetworkType::Testnet, 10));
        assert!(config.rpc_url.is_none());
    }
}