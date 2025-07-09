// src/api/http/types.rs
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AuthRequest {
    // Intentionally empty for now
    pub public_key: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub episode_id: u64,
    pub organizer_public_key: String,
    pub participant_kaspa_address: String,
    pub transaction_id: Option<String>,
    pub status: String,
}

#[derive(Deserialize)]
pub struct ChallengeRequest {
    pub episode_id: u64,
    pub public_key: String,
}

#[derive(Serialize)]
pub struct ChallengeResponse {
    pub episode_id: u64,
    pub nonce: String,
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub episode_id: u64,
    pub signature: String,
    pub nonce: String,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub episode_id: u64,
    pub authenticated: bool,
    pub status: String,
}

#[derive(Serialize)]
pub struct EpisodeStatus {
    pub episode_id: u64,
    pub authenticated: bool,
    pub status: String,
}