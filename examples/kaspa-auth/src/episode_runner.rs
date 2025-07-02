use kdapp::{
    engine::{self},
    episode::{EpisodeEventHandler, EpisodeId, PayloadMetadata},
    generator::{PatternType, PrefixType, TransactionGenerator},
    pki::PubKey,
    proxy::{self, connect_client},
};
use kaspa_consensus_core::network::{NetworkId, NetworkType};
use std::sync::{mpsc::channel, Arc, atomic::AtomicBool};
use secp256k1::Keypair;
use log::{info, warn, error};

use crate::{SimpleAuth, auth_commands::AuthCommand};

// Define unique pattern and prefix for auth transactions
// Pattern: specific byte positions that must match to reduce node overhead
const AUTH_PATTERN: PatternType = [
    (7, 0), (32, 1), (45, 0), (99, 1), (113, 0), 
    (126, 1), (189, 0), (200, 1), (211, 0), (250, 1)
];

// Unique prefix to identify auth transactions (chosen to avoid conflicts)
const AUTH_PREFIX: PrefixType = 0x41555448; // "AUTH" in hex

/// Event handler for authentication episodes
pub struct AuthEventHandler {
    pub name: String,
}

impl AuthEventHandler {
    pub fn new(name: String) -> Self {
        Self { name }
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
}

impl AuthServerConfig {
    pub fn new_testnet10(signer: Keypair, name: String) -> Self {
        Self {
            signer,
            network: NetworkId::with_suffix(NetworkType::Testnet, 10),
            rpc_url: None, // Use public nodes
            name,
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

    // 2. Set up engine channel
    let (sender, receiver) = channel();

    // 3. Create and start engine
    let mut engine = engine::Engine::<SimpleAuth, AuthEventHandler>::new(receiver);
    let event_handler = AuthEventHandler::new(config.name.clone());
    
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

    // 6. Start proxy listener
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
        let handler = AuthEventHandler::new("test-server".to_string());
        assert_eq!(handler.name, "test-server");
    }

    #[test]
    fn test_config_creation() {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::new(&mut rand::thread_rng());
        let keypair = Keypair::from_secret_key(&secp, &secret_key);
        
        let config = AuthServerConfig::new_testnet10(keypair, "test".to_string());
        assert_eq!(config.name, "test");
        assert_eq!(config.network, NetworkId::with_suffix(NetworkType::Testnet, 10));
        assert!(config.rpc_url.is_none());
    }
}