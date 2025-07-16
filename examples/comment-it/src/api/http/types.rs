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
    pub transaction_id: Option<String>,
    pub status: String,
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
    pub transaction_id: Option<String>,
}

#[derive(Serialize)]
pub struct EpisodeStatus {
    pub episode_id: u64,
    pub authenticated: bool,
    pub status: String,
}

#[derive(Deserialize)]
pub struct RevokeSessionRequest {
    pub episode_id: u64,
    pub session_token: String,
    pub signature: String,
}

#[derive(Serialize)]
pub struct RevokeSessionResponse {
    pub episode_id: u64,
    pub transaction_id: String,
    pub status: String,
}

// Comment-related types
#[derive(Deserialize)]
pub struct SubmitCommentRequest {
    pub episode_id: u64,
    pub text: String,
    pub session_token: String,
}

#[derive(Serialize)]
pub struct SubmitCommentResponse {
    pub episode_id: u64,
    pub comment_id: u64,
    pub transaction_id: Option<String>,
    pub status: String,
}

#[derive(Deserialize)]
pub struct GetCommentsRequest {
    pub episode_id: u64,
    pub session_token: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct CommentData {
    pub id: u64,
    pub text: String,
    pub author: String,
    pub timestamp: u64,
    pub author_type: String, // "authenticated" only for now (anonymous requires profile episode)
}

#[derive(Serialize)]
pub struct GetCommentsResponse {
    pub episode_id: u64,
    pub comments: Vec<CommentData>,
    pub status: String,
}