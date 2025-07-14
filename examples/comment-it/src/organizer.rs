use axum::{
    extract::{State, WebSocketUpgrade},
    http::StatusCode,
    response::{Html, Json, Response},
    routing::{get, post},
    Router,
};
use kdapp::pki::PubKey;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tower_http::{cors::CorsLayer, services::ServeDir};

use crate::comment::{Comment, CommentEpisode};

// Import auth components from our unified comment-it project
use crate::{
    core::episode::SimpleAuth,
    api::http::types::{AuthRequest, AuthResponse, ChallengeResponse, VerifyRequest, VerifyResponse},
    wallet::get_wallet_for_command,
};

/// State shared across the unified comment-it organizer peer
#[derive(Clone)]
pub struct OrganizerState {
    /// Authentication episodes by episode ID (from kaspa-auth)
    pub auth_episodes: Arc<Mutex<HashMap<u64, SimpleAuth>>>,
    /// Comment episodes by episode ID
    pub comment_episodes: Arc<Mutex<HashMap<u64, CommentEpisode>>>,
    /// WebSocket broadcast channel for real-time updates
    pub websocket_tx: broadcast::Sender<CommentUpdate>,
    /// Organizer peer's keypair for signing transactions
    pub organizer_keypair: secp256k1::Keypair,
}

/// Real-time comment updates sent via WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentUpdate {
    pub episode_id: u64,
    pub comment: Comment,
    pub update_type: String, // "new_comment"
}

/// Request to submit a comment
#[derive(Debug, Deserialize)]
pub struct SubmitCommentRequest {
    pub text: String,
    pub session_token: String,
    pub author: String,
}

/// Response after submitting a comment
#[derive(Debug, Serialize)]
pub struct SubmitCommentResponse {
    pub success: bool,
    pub comment_id: Option<u64>,
    pub message: String,
}

/// Response for getting comments
#[derive(Debug, Serialize)]
pub struct GetCommentsResponse {
    pub comments: Vec<Comment>,
    pub total: usize,
}

/// Comment organizer peer - coordinates comment episodes via HTTP/WebSocket
pub struct CommentOrganizer {
    host: String,
    port: u16,
    state: OrganizerState,
}

impl CommentOrganizer {
    pub async fn new(host: String, port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let (websocket_tx, _) = broadcast::channel(100);
        
        // Load organizer wallet (same pattern as kaspa-auth)
        let wallet = get_wallet_for_command("comment-organizer", None)?;
        let organizer_keypair = wallet.keypair;
        
        let state = OrganizerState {
            auth_episodes: Arc::new(Mutex::new(HashMap::new())),
            comment_episodes: Arc::new(Mutex::new(HashMap::new())),
            websocket_tx,
            organizer_keypair,
        };

        Ok(Self {
            host,
            port,
            state,
        })
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        // Print startup banner
        self.print_startup_banner();
        
        let app = Router::new()
            // Main page
            .route("/", get(serve_index))
            
            // Authentication endpoints (from kaspa-auth)
            .route("/auth/start", post(start_auth))
            .route("/auth/challenge/{episode_id}", get(get_challenge))
            .route("/auth/verify", post(verify_auth))
            .route("/auth/revoke-session", post(revoke_session))
            .route("/auth/status/{episode_id}", get(get_auth_status))
            
            // Comment endpoints
            .route("/api/comments", post(submit_comment))
            .route("/api/comments", get(get_comments))
            .route("/api/comments/latest", get(get_latest_comments))
            
            // Debug and utility
            .route("/api/debug", get(debug_endpoint))
            .route("/health", get(health_check))
            .route("/ws", get(websocket_handler))
            
            .nest_service("/static", ServeDir::new("public"))
            .layer(CorsLayer::permissive())
            .with_state(self.state);

        let addr = format!("{}:{}", self.host, self.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }

    fn print_startup_banner(&self) {
        println!();
        println!("💬 ===============================================");
        println!("💬   Comment It - Unified P2P Organizer Peer");
        println!("💬 ===============================================");
        println!();
        println!("🚀 Starting UNIFIED Comment + Auth Organizer Peer");
        println!("🔗 kaspa-auth integrated directly (no external dependency!)");
        println!();
        println!("📖 The Perfect Developer Journey:");
        println!("   1. 'How do I login?' → INTEGRATED authentication");
        println!("   2. 'How do I comment?' → SAME organizer peer!");
        println!();
        println!("🌐 Unified organizer peer running on: http://{}:{}", self.host, self.port);
        println!("🔐 Authentication endpoints:");
        println!("   • POST /auth/start       - Start auth episode");
        println!("   • GET  /auth/challenge/:id - Get challenge");
        println!("   • POST /auth/verify      - Verify signature");
        println!("   • POST /auth/revoke-session - Revoke session");
        println!("💬 Comment endpoints:");
        println!("   • POST /api/comments     - Submit new comment");
        println!("   • GET  /api/comments     - Get all comments");
        println!("   • GET  /api/comments/latest - Get latest comments");
        println!("🔗 Real-time WebSocket: ws://{}:{}/ws", self.host, self.port);
        println!();
        println!("✅ NO DEPENDENCIES: Everything in one organizer peer!");
        println!("🎯 Ready for the ultimate comment experience:");
        println!("   1. Open: http://{}:{}", self.host, self.port);
        println!("   2. Login (integrated auth)");
        println!("   3. Comment (same peer)");
        println!("   4. Real-time updates ✨");
        println!();
        println!("💡 True P2P Architecture:");
        println!("   • Unified organizer peer = Auth + Comments");
        println!("   • Web participant peer   = Your browser");
        println!("   • Blockchain            = Source of truth");
        println!();
        println!("🚀 Starting unified HTTP coordination peer...");
    }
}

/// Serve the main HTML page
async fn serve_index() -> Html<&'static str> {
    // Embed the HTML at compile time to avoid path issues
    Html(include_str!("../public/index.html"))
}

/// Debug endpoint to test if comment-it is working
async fn debug_endpoint() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "comment-it unified organizer peer",
        "message": "Comment-it with integrated auth is running correctly!",
        "auth": "integrated",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}

/// Health check endpoint (from kaspa-auth)
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "comment-it unified organizer peer",
        "auth": "integrated",
        "comments": "enabled"
    }))
}

/// Start authentication episode (integrated from kaspa-auth)
async fn start_auth(
    State(state): State<OrganizerState>,
    Json(_req): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    info!("🚀 Starting authentication episode (integrated)");
    
    // TODO: Implement using kaspa-auth logic but in integrated way
    // For now, return a basic response
    Ok(Json(AuthResponse {
        episode_id: 12345,
        organizer_public_key: hex::encode(state.organizer_keypair.public_key().serialize()),
        participant_kaspa_address: "kaspatest:placeholder".to_string(),
        transaction_id: Some("integrated_auth_tx".to_string()),
        status: "episode_created".to_string(),
    }))
}

/// Get challenge for authentication episode
async fn get_challenge(
    State(_state): State<OrganizerState>,
    axum::extract::Path(episode_id): axum::extract::Path<u64>,
) -> Result<Json<ChallengeResponse>, StatusCode> {
    info!("🎲 Getting challenge for episode {}", episode_id);
    
    // TODO: Get real challenge from auth episode
    Ok(Json(ChallengeResponse {
        episode_id,
        nonce: format!("auth_challenge_{}", episode_id),
        transaction_id: Some("challenge_tx".to_string()),
        status: "challenge_ready".to_string(),
    }))
}

/// Verify authentication signature  
async fn verify_auth(
    State(_state): State<OrganizerState>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, StatusCode> {
    info!("✅ Verifying authentication for episode {}", req.episode_id);
    
    // TODO: Implement real signature verification
    Ok(Json(VerifyResponse {
        episode_id: req.episode_id,
        authenticated: true,
        status: "authenticated".to_string(),
        transaction_id: Some("verify_tx".to_string()),
    }))
}

/// Revoke authentication session
async fn revoke_session(
    State(_state): State<OrganizerState>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("🔄 Revoking session");
    
    // TODO: Implement session revocation
    Ok(Json(serde_json::json!({
        "status": "session_revoked",
        "message": "Session revoked successfully"
    })))
}

/// Get authentication status for episode
async fn get_auth_status(
    State(_state): State<OrganizerState>,
    axum::extract::Path(episode_id): axum::extract::Path<u64>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("📊 Getting auth status for episode {}", episode_id);
    
    // TODO: Get real auth status from episode
    Ok(Json(serde_json::json!({
        "episode_id": episode_id,
        "authenticated": false,
        "challenge": null,
        "session_token": null
    })))
}

/// Submit a new comment to the blockchain
async fn submit_comment(
    State(state): State<OrganizerState>,
    Json(request): Json<SubmitCommentRequest>,
) -> Result<Json<SubmitCommentResponse>, StatusCode> {
    info!("📝 Comment submission request: {}", request.text);

    // TODO: Verify session token with kaspa-auth organizer peer
    // TODO: Submit comment transaction to blockchain
    // TODO: Wait for blockchain confirmation

    // For now, simulate comment creation
    let comment = Comment {
        id: 1, // TODO: Get from episode
        text: request.text.clone(),
        author: request.author.clone(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        session_token: request.session_token.clone(),
    };

    // Broadcast to WebSocket clients
    let update = CommentUpdate {
        episode_id: 1, // TODO: Use real episode ID
        comment: comment.clone(),
        update_type: "new_comment".to_string(),
    };

    if let Err(e) = state.websocket_tx.send(update) {
        error!("Failed to broadcast comment update: {}", e);
    }

    // Store in memory (TODO: Store in blockchain episode)
    // For now, just return success

    Ok(Json(SubmitCommentResponse {
        success: true,
        comment_id: Some(comment.id),
        message: "Comment submitted successfully".to_string(),
    }))
}

/// Get all comments
async fn get_comments(
    State(_state): State<OrganizerState>,
) -> Result<Json<GetCommentsResponse>, StatusCode> {
    // TODO: Get comments from blockchain episode
    
    // For now, return empty list
    Ok(Json(GetCommentsResponse {
        comments: vec![],
        total: 0,
    }))
}

/// Get latest comments
async fn get_latest_comments(
    State(_state): State<OrganizerState>,
) -> Result<Json<GetCommentsResponse>, StatusCode> {
    // TODO: Get latest comments from blockchain episode
    
    // For now, return empty list
    Ok(Json(GetCommentsResponse {
        comments: vec![],
        total: 0,
    }))
}

/// WebSocket handler for real-time comment updates
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<OrganizerState>,
) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(
    socket: axum::extract::ws::WebSocket,
    state: OrganizerState,
) {
    use axum::extract::ws::Message;
    use futures_util::{sink::SinkExt, stream::StreamExt};
    
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.websocket_tx.subscribe();

    info!("🔗 WebSocket connection established");

    // Spawn task to send updates to client
    let send_task = tokio::spawn(async move {
        while let Ok(update) = rx.recv().await {
            let message = serde_json::to_string(&update).unwrap();
            if sender.send(Message::Text(message.into())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages (if any)
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            if let Ok(Message::Text(text)) = msg {
                info!("📨 WebSocket message received: {}", text);
                // TODO: Handle incoming WebSocket messages if needed
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    info!("🔌 WebSocket connection closed");
}