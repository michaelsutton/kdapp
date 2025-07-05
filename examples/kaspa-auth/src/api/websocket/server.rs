use axum::{extract::ws::{Message, WebSocket, WebSocketUpgrade}, response::IntoResponse, extract::State};
use futures_util::{sink::SinkExt, stream::StreamExt};
use log::info;
use std::sync::Arc;
use tokio::sync::broadcast;

// This struct will hold the shared state for the WebSocket server
// For now, we'll use a simple broadcast channel for episode updates.
#[derive(Clone)]
pub struct HttpServerState {
    pub episode_updates: broadcast::Sender<String>,
    pub keypair: secp256k1::Keypair,
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<HttpServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: HttpServerState) {
    info!("New WebSocket connection established.");
    let mut rx = state.episode_updates.subscribe();

    // Split the socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Task for sending messages to the client
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                // Client disconnected
                break;
            }
        }
    });

    // Task for receiving messages from the client (if any)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            info!("Received message from WebSocket: {}", text);
            // Here you could process incoming messages, e.g., commands from a client
        }
    });

    // If one of the tasks completes, abort the other
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    info!("WebSocket connection closed.");
}
