use axum::{extract::ws::{Message, WebSocket, WebSocketUpgrade}, response::IntoResponse, extract::State};
use futures_util::{sink::SinkExt, stream::StreamExt};
use log::info;
use std::sync::Arc;
use tokio::sync::broadcast;
use serde_json;
use crate::api::http::state::ServerState;

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: ServerState) {
    info!("New WebSocket connection established.");
    let mut rx = state.websocket_tx.subscribe();

    // Split the socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Task for sending messages to the client
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let json_msg = serde_json::to_string(&msg).unwrap_or_else(|_| "{}".to_string());
            if sender.send(Message::Text(json_msg.into())).await.is_err() {
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
