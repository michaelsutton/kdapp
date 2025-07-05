use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct StartAuthRequest;

#[derive(Serialize, Deserialize)]
pub struct StartAuthResponse;

#[derive(Serialize, Deserialize)]
pub struct RequestChallengeRequest;

#[derive(Serialize, Deserialize)]
pub struct ChallengeResponse;

#[derive(Serialize, Deserialize)]
pub struct AuthRequest {
    pub signature: String,
    pub nonce: String,
    pub client_pubkey: String,
}

#[derive(Serialize, Deserialize)]
pub struct AuthResponse {
    pub authenticated: bool,
    pub session_token: Option<String>,
}