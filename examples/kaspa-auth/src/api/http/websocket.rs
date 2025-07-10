// src/api/http/websocket.rs
use axum::{
    extract::{ws::{WebSocket, Message}, WebSocketUpgrade, State},
    response::Response,
};
use crate::api::http::state::PeerState;
use tokio::select;
use log::info;

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<PeerState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: PeerState) {
    info!("New WebSocket connection established");
    
    // Subscribe to broadcast messages
    let mut rx = state.websocket_tx.subscribe();
    
    loop {
        select! {
            // Listen for broadcast messages from the server
            msg = rx.recv() => {
                match msg {
                    Ok(ws_message) => {
                        let json_str = match serde_json::to_string(&ws_message) {
                            Ok(json) => json,
                            Err(e) => {
                                eprintln!("Failed to serialize WebSocket message: {}", e);
                                continue;
                            }
                        };
                        
                        if socket.send(Message::Text(json_str.into())).await.is_err() {
                            info!("WebSocket connection closed");
                            break;
                        }
                    }
                    Err(_) => {
                        // Channel closed
                        break;
                    }
                }
            }
            
            // Listen for incoming messages from client (optional)
            socket_msg = socket.recv() => {
                match socket_msg {
                    Some(Ok(_)) => {
                        // Handle client messages if needed
                        // For now, just continue
                    }
                    _ => {
                        info!("WebSocket connection closed by client");
                        break;
                    }
                }
            }
        }
    }
}