// src/api/http/websocket.rs
use axum::{
    extract::{ws::WebSocket, WebSocketUpgrade, State},
    response::Response,
};
use crate::api::http::state::PeerState;

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(_state): State<PeerState>,
) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    // Simple placeholder WebSocket handler
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            // Echo back the message for now
            if socket.send(msg).await.is_err() {
                break;
            }
        } else {
            break;
        }
    }
}