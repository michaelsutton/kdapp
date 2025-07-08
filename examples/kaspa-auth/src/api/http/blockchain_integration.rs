// src/api/http/blockchain_integration.rs - REAL blockchain integration for HTTP server

use std::sync::{Arc, Mutex, mpsc::{channel, Sender}};
use std::collections::HashMap;
use tokio::task::JoinHandle;
use log::{info, error};
use rand::Rng;
use kdapp::{
    engine::{Engine, EngineMsg},
    episode::{EpisodeEventHandler, EpisodeId, PayloadMetadata},
    proxy,
    pki::PubKey,
};
use kaspa_consensus_core::network::NetworkId;
use crate::{
    core::{episode::SimpleAuth, commands::AuthCommand},
    episode_runner::{AUTH_PREFIX, AUTH_PATTERN},
};

/// HTTP-aware event handler that updates episode states
pub struct HttpAuthEventHandler {
    pub name: String,
    pub episode_storage: Arc<Mutex<HashMap<u64, super::server::EpisodeState>>>,
    pub websocket_tx: tokio::sync::broadcast::Sender<super::server::WebSocketMessage>,
    pub http_notify_url: String,
}

impl EpisodeEventHandler<SimpleAuth> for HttpAuthEventHandler {
    fn on_initialize(&self, episode_id: EpisodeId, episode: &SimpleAuth) {
        info!("[{}] Episode {} initialized on blockchain", self.name, episode_id);
        
        // Update HTTP server's episode storage
        if let Ok(mut episodes) = self.episode_storage.lock() {
            if let Some(http_episode) = episodes.get_mut(&(episode_id as u64)) {
                info!("[{}] Syncing episode {} with blockchain state", self.name, episode_id);
            }
        }
    }

    fn on_command(&self, episode_id: EpisodeId, episode: &SimpleAuth, 
                  cmd: &AuthCommand, authorization: Option<PubKey>, 
                  _metadata: &PayloadMetadata) {
        match cmd {
            AuthCommand::RequestChallenge => {
                info!("[{}] Episode {}: Challenge requested on blockchain", self.name, episode_id);
                if let Some(challenge) = &episode.challenge {
                    // Update HTTP storage with blockchain-generated challenge
                    if let Ok(mut episodes) = self.episode_storage.lock() {
                        if let Some(http_episode) = episodes.get_mut(&(episode_id as u64)) {
                            http_episode.challenge = Some(challenge.clone());
                            info!("[{}] Challenge synced to HTTP: {}", self.name, challenge);
                            
                            // Broadcast via WebSocket
                            let _ = self.websocket_tx.send(super::server::WebSocketMessage::ChallengeIssued {
                                episode_id: episode_id as u64,
                                challenge: challenge.clone(),
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                            });
                        }
                    }
                }
            }
            AuthCommand::SubmitResponse { .. } => {
                info!("[{}] Episode {}: Response submitted on blockchain", self.name, episode_id);
                if episode.is_authenticated {
                    info!("[{}] ✅ BLOCKCHAIN CONFIRMED: Episode {} authenticated!", self.name, episode_id);
                    
                    // Update HTTP storage with REAL authentication
                    if let Ok(mut episodes) = self.episode_storage.lock() {
                        if let Some(http_episode) = episodes.get_mut(&(episode_id as u64)) {
                            http_episode.authenticated = true;
                            let session_token = format!("sess_{}", rand::thread_rng().gen::<u64>());
                            http_episode.session_token = Some(session_token.clone());
                            
                            // Broadcast REAL success via WebSocket
                            let _ = self.websocket_tx.send(super::server::WebSocketMessage::AuthenticationSuccessful {
                                episode_id: episode_id as u64,
                                session_token: session_token.clone(),
                            });
                            
                            info!("[{}] 🎉 Real blockchain authentication complete for episode {}", 
                                  self.name, episode_id);
                        }
                    }
                    
                    // Optional: Notify HTTP server via internal endpoint
                    let client = reqwest::Client::new();
                    let episode_id_u64 = episode_id as u64;
                    let challenge = episode.challenge.clone().unwrap_or_default();
                    let notify_url = self.http_notify_url.clone();
                    
                    tokio::spawn(async move {
                        let _ = client.post(&notify_url)
                            .json(&serde_json::json!({
                                "episode_id": episode_id_u64,
                                "challenge": challenge,
                            }))
                            .send()
                            .await;
                    });
                } else {
                    error!("[{}] Episode {}: Authentication failed on blockchain", self.name, episode_id);
                    
                    // Broadcast failure
                    let _ = self.websocket_tx.send(super::server::WebSocketMessage::AuthenticationFailed {
                        episode_id: episode_id as u64,
                        reason: "Blockchain verification failed".to_string(),
                    });
                }
            }
        }
    }

    fn on_rollback(&self, episode_id: EpisodeId, _episode: &SimpleAuth) {
        info!("[{}] Episode {} rolled back due to DAG reorg", self.name, episode_id);
        
        // Update HTTP storage to reflect rollback
        if let Ok(mut episodes) = self.episode_storage.lock() {
            if let Some(http_episode) = episodes.get_mut(&(episode_id as u64)) {
                http_episode.authenticated = false;
                http_episode.session_token = None;
                info!("[{}] Episode {} authentication revoked due to rollback", self.name, episode_id);
            }
        }
    }
}

/// Integrated blockchain listener for HTTP server
pub struct BlockchainIntegration {
    pub engine_handle: Option<JoinHandle<()>>,
    pub listener_handle: Option<JoinHandle<()>>,
    pub exit_signal: Arc<std::sync::atomic::AtomicBool>,
}

impl BlockchainIntegration {
    pub async fn start(
        network_id: NetworkId,
        episode_storage: Arc<Mutex<HashMap<u64, super::server::EpisodeState>>>,
        websocket_tx: tokio::sync::broadcast::Sender<super::server::WebSocketMessage>,
        http_port: u16,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        info!("🔗 Starting blockchain integration for HTTP server...");
        
        // Connect to Kaspa
        let kaspad = proxy::connect_client(network_id, None).await?;
        info!("✅ Connected to Kaspa network for HTTP server integration");
        
        // Create engine channel
        let (sender, receiver) = channel();
        
        // Create HTTP-aware event handler
        let event_handler = HttpAuthEventHandler {
            name: "http-blockchain-bridge".to_string(),
            episode_storage: episode_storage.clone(),
            websocket_tx: websocket_tx.clone(),
            http_notify_url: format!("http://127.0.0.1:{}/internal/episode-authenticated", http_port),
        };
        
        // Start engine in background
        let engine_handle = tokio::task::spawn_blocking(move || {
            let mut engine = Engine::<SimpleAuth, HttpAuthEventHandler>::new(receiver);
            info!("🚀 Starting HTTP-integrated kdapp engine");
            engine.start(vec![event_handler]);
        });
        
        // Set up exit signal
        let exit_signal = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let exit_signal_clone = exit_signal.clone();
        
        // Start proxy listener
        let engines = std::iter::once((AUTH_PREFIX, (AUTH_PATTERN, sender))).collect();
        let listener_handle = tokio::spawn(async move {
            info!("👂 HTTP server listening for AUTH transactions with prefix: 0x{:08X}", AUTH_PREFIX);
            proxy::run_listener(kaspad, engines, exit_signal_clone).await;
        });
        
        Ok(BlockchainIntegration {
            engine_handle: Some(engine_handle),
            listener_handle: Some(listener_handle),
            exit_signal,
        })
    }
    
    pub fn shutdown(&self) {
        info!("🛑 Shutting down blockchain integration...");
        self.exit_signal.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

// Helper function to integrate with existing HTTP server
pub async fn integrate_http_server_with_blockchain(
    server_state: &super::server::ServerState,
    port: u16,
) -> Result<BlockchainIntegration, Box<dyn std::error::Error>> {
    let network_id = NetworkId::with_suffix(
        kaspa_consensus_core::network::NetworkType::Testnet, 
        10
    );
    
    BlockchainIntegration::start(
        network_id,
        server_state.episodes.clone(),
        server_state.websocket_tx.clone(),
        port,
    ).await
}