// src/api/http/state.rs
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::sync::broadcast;
use secp256k1::Keypair;
use kdapp::generator::TransactionGenerator;
use crate::core::episode::SimpleAuth;
use kaspa_wrpc_client::KaspaRpcClient;

// Real blockchain-based episode state (not the old fake HashMap approach)
pub type SharedEpisodeState = Arc<Mutex<HashMap<u64, SimpleAuth>>>;

#[derive(Clone)]
pub struct EpisodeState {
    pub public_key: String,
    pub authenticated: bool,
    pub status: String,
}

#[derive(Clone)]
pub struct PeerState {
    pub episodes: Arc<Mutex<HashMap<u64, EpisodeState>>>,  // Legacy - will remove
    pub blockchain_episodes: SharedEpisodeState,  // NEW - real blockchain state
    pub websocket_tx: broadcast::Sender<WebSocketMessage>,
    pub peer_keypair: Keypair,
    pub transaction_generator: Arc<TransactionGenerator>,
    pub kaspad_client: Option<Arc<KaspaRpcClient>>,  // NEW - for transaction submission
    pub auth_http_peer: Option<Arc<crate::api::http::blockchain_engine::AuthHttpPeer>>, // Reference to the main peer
}

// WebSocket message for real-time blockchain updates
#[derive(Clone, Debug, serde::Serialize)]
pub struct WebSocketMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub episode_id: Option<u64>,
    pub authenticated: Option<bool>,
    pub challenge: Option<String>,
    pub session_token: Option<String>,
}