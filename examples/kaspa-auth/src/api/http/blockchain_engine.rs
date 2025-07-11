// src/api/http/blockchain_engine.rs
use std::sync::{Arc, atomic::AtomicBool, mpsc};
use std::collections::HashMap;
use tokio::sync::broadcast;
use secp256k1::Keypair;
use kdapp::{
    engine::Engine,
    episode::{EpisodeEventHandler, EpisodeId},
    proxy::connect_client,
    generator::TransactionGenerator,
};
use kaspa_consensus_core::network::{NetworkId, NetworkType};

use crate::core::episode::SimpleAuth;
use crate::core::commands::AuthCommand;
use crate::api::http::state::{PeerState, WebSocketMessage, SharedEpisodeState};
use crate::episode_runner::{AUTH_PREFIX, AUTH_PATTERN};
use kaspa_wrpc_client::prelude::{RpcApi, KaspaRpcClient};

/// The main HTTP coordination peer that runs a real kdapp engine
#[derive(Clone)]
pub struct AuthHttpPeer {
    pub peer_state: PeerState,
    pub network: NetworkId,
    pub exit_signal: Arc<AtomicBool>,
}

impl AuthHttpPeer {
    pub async fn new(
        peer_keypair: Keypair,
        websocket_tx: broadcast::Sender<WebSocketMessage>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let network = NetworkId::with_suffix(NetworkType::Testnet, 10);
        
        let transaction_generator = Arc::new(TransactionGenerator::new(
            peer_keypair,
            AUTH_PATTERN,
            AUTH_PREFIX,
        ));
        
        // Create shared episode state that both engine and HTTP coordination peer can access
        let blockchain_episodes = Arc::new(std::sync::Mutex::new(HashMap::new()));
        
        // Create kaspad client for transaction submission
        let kaspad_client = match connect_client(network, None).await {
            Ok(client) => {
                println!("✅ Connected to Kaspa node for transaction submission");
                Some(Arc::new(client))
            }
            Err(e) => {
                println!("⚠️ Failed to connect to Kaspa node: {}", e);
                println!("📋 Transactions will be created but not submitted");
                None
            }
        };
        
        let mut peer_state = PeerState {
            episodes: Arc::new(std::sync::Mutex::new(HashMap::new())),  // Legacy
            blockchain_episodes: blockchain_episodes.clone(),  // NEW - real blockchain state
            websocket_tx,
            peer_keypair,
            transaction_generator,
            kaspad_client,  // NEW - for actual transaction submission
            auth_http_peer: None, // Will be set after AuthHttpPeer is created
        };
        
        let exit_signal = Arc::new(AtomicBool::new(false));
        
        let auth_http_peer = AuthHttpPeer {
            peer_state: peer_state.clone(),
            network,
            exit_signal,
        };
        
        // Set the self reference after the struct is created
        peer_state.auth_http_peer = Some(Arc::new(auth_http_peer.clone()));
        
        Ok(auth_http_peer)
    }
    
    /// Start the blockchain listener - this makes HTTP coordination peer a real kdapp node!
    pub async fn start_blockchain_listener(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel();
        
        // Create the episode handler that will process blockchain updates
        let auth_handler = HttpAuthHandler {
            websocket_tx: self.peer_state.websocket_tx.clone(),
            blockchain_episodes: self.peer_state.blockchain_episodes.clone(),
        };
        
        // Start the kdapp engine in a background task
        let engine_task = {
            let rx = rx;
            tokio::task::spawn_blocking(move || {
                let mut engine = Engine::<SimpleAuth, HttpAuthHandler>::new(rx);
                engine.start(vec![auth_handler]);
            })
        };
        
        // Create engines map for proxy listener
        let engines = std::iter::once((AUTH_PREFIX, (AUTH_PATTERN, tx))).collect();
        
        // Start the blockchain listener using kdapp's proper pattern
        let kaspad = connect_client(self.network, None).await?;
        let exit_signal_clone = self.exit_signal.clone();
        let listener_task = tokio::spawn(async move {
            kdapp::proxy::run_listener(kaspad, engines, exit_signal_clone).await;
        });
        
        println!("🔗 kdapp engine started - HTTP coordination peer is now a real blockchain node!");
        
        // Wait for either task to complete
        tokio::select! {
            _ = engine_task => {
                println!("⚠️ kdapp engine task completed");
            }
            _ = listener_task => {
                println!("⚠️ Blockchain listener task completed");
            }
        }
        
        Ok(())
    }
    
    /// Set the auth peer reference in the peer state
    pub fn set_self_reference(self, _auth_peer: Arc<AuthHttpPeer>) -> Self {
        // This creates a circular reference which is fine for this use case
        // The auth_peer field allows handlers to access the kdapp engine
        // We'll use weak references if needed later
        self
    }
    
    /// Get episode state from the kdapp engine (not memory!)
    pub fn get_episode_state(&self, episode_id: EpisodeId) -> Option<SimpleAuth> {
        println!("🔍 Querying blockchain episode state for episode {}", episode_id);
        
        match self.peer_state.blockchain_episodes.lock() {
            Ok(episodes) => {
                if let Some(episode) = episodes.get(&(episode_id as u64)) {
                    println!("✅ Found episode {} in blockchain state", episode_id);
                    Some(episode.clone())
                } else {
                    println!("⚠️ Episode {} not found in blockchain state", episode_id);
                    None
                }
            }
            Err(e) => {
                println!("❌ Failed to lock blockchain episodes: {}", e);
                None
            }
        }
    }
    
    /// Submit an EpisodeMessage transaction to the blockchain
    pub async fn submit_episode_message_transaction(
        &self,
        episode_message: kdapp::engine::EpisodeMessage<crate::core::episode::SimpleAuth>,
        signer_keypair: secp256k1::Keypair,
        funding_address: kaspa_addresses::Address,
        utxo: (kaspa_consensus_core::tx::TransactionOutpoint, kaspa_consensus_core::tx::UtxoEntry),
    ) -> Result<String, Box<dyn std::error::Error>> {
        let generator = kdapp::generator::TransactionGenerator::new(
            signer_keypair,
            crate::episode_runner::AUTH_PATTERN,
            crate::episode_runner::AUTH_PREFIX,
        );

        let tx = generator.build_command_transaction(
            utxo,
            &funding_address,
            &episode_message,
            5000,
        );

        let transaction_id = tx.id().to_string();

        if let Some(kaspad) = self.peer_state.kaspad_client.as_ref() {
            match kaspad.submit_transaction(tx.as_ref().into(), false).await {
                Ok(_) => {
                    println!("✅ Transaction {} submitted to blockchain via AuthHttpPeer", transaction_id);
                    Ok(transaction_id)
                }
                Err(e) => {
                    println!("❌ Transaction {} submission failed: {}", transaction_id, e);
                    Err(e.into())
                }
            }
        } else {
            Err("Kaspad client not available for transaction submission.".into())
        }
    }
}

/// Episode event handler that broadcasts updates to WebSocket clients
pub struct HttpAuthHandler {
    pub websocket_tx: broadcast::Sender<WebSocketMessage>,
    pub blockchain_episodes: SharedEpisodeState,
}

impl EpisodeEventHandler<SimpleAuth> for HttpAuthHandler {
    fn on_initialize(&self, episode_id: EpisodeId, episode: &SimpleAuth) {
        println!("🎬 Episode {} initialized on blockchain", episode_id);
        
        // Store episode in shared blockchain state
        if let Ok(mut episodes) = self.blockchain_episodes.lock() {
            episodes.insert(episode_id.into(), episode.clone());
            println!("✅ Stored episode {} in blockchain state", episode_id);
        } else {
            println!("❌ Failed to store episode {} in blockchain state", episode_id);
        }
        
        let message = WebSocketMessage {
            message_type: "episode_created".to_string(),
            episode_id: Some(episode_id.into()),
            authenticated: Some(false),
            challenge: episode.challenge.clone(),
            session_token: episode.session_token.clone(),
        };
        
        let _ = self.websocket_tx.send(message);
    }
    
    fn on_command(
        &self,
        episode_id: EpisodeId,
        episode: &SimpleAuth,
        _cmd: &AuthCommand,
        _authorization: Option<kdapp::pki::PubKey>,
        _metadata: &kdapp::episode::PayloadMetadata,
    ) {
        println!("⚡ Episode {} updated on blockchain", episode_id);
        
        // Read previous state BEFORE updating (for session revocation detection)
        let previous_episode = if let Ok(episodes) = self.blockchain_episodes.lock() {
            episodes.get(&(episode_id as u64)).cloned()
        } else {
            None
        };
        
        // Update episode in shared blockchain state
        if let Ok(mut episodes) = self.blockchain_episodes.lock() {
            episodes.insert(episode_id.into(), episode.clone());
            println!("✅ Updated episode {} in blockchain state", episode_id);
        } else {
            println!("❌ Failed to update episode {} in blockchain state", episode_id);
        }
        
        // Check what kind of update this is
        if episode.is_authenticated && episode.session_token.is_some() {
            // Authentication successful
            let message = WebSocketMessage {
                message_type: "authentication_successful".to_string(),
                episode_id: Some(episode_id.into()),
                authenticated: Some(true),
                challenge: episode.challenge.clone(),
                session_token: episode.session_token.clone(),
            };
            let _ = self.websocket_tx.send(message);
        } else if !episode.is_authenticated && episode.session_token.is_none() && episode.challenge.is_some() {
            // Check if this was a session revocation by comparing with previous state
            if let Some(prev_episode) = previous_episode {
                if prev_episode.is_authenticated && prev_episode.session_token.is_some() {
                    // Previous state was authenticated, now it's not -> session revoked
                    let message = WebSocketMessage {
                        message_type: "session_revoked".to_string(),
                        episode_id: Some(episode_id.into()),
                        authenticated: Some(false),
                        challenge: episode.challenge.clone(),
                        session_token: None,
                    };
                    let _ = self.websocket_tx.send(message);
                    println!("📡 Sent session_revoked WebSocket message for episode {}", episode_id);
                    return; // Don't send challenge_issued message
                }
            }
            
            // Challenge was issued (initial state)
            let message = WebSocketMessage {
                message_type: "challenge_issued".to_string(),
                episode_id: Some(episode_id.into()),
                authenticated: Some(false),
                challenge: episode.challenge.clone(),
                session_token: None,
            };
            let _ = self.websocket_tx.send(message);
        }
    }
    
    fn on_rollback(&self, episode_id: EpisodeId, _episode: &SimpleAuth) {
        println!("🔄 Episode {} rolled back on blockchain", episode_id);
    }
}