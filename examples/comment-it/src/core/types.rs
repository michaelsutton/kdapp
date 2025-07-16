use borsh::{BorshDeserialize, BorshSerialize};
use kdapp::pki::PubKey;
use std::collections::HashMap;

/// Rollback information for authentication operations
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub enum AuthRollback {
    Challenge { 
        previous_challenge: Option<String>,
        previous_timestamp: u64,
    },
    Authentication {
        previous_auth_status: bool,
        previous_session_token: Option<String>,
    },
    SessionRevoked {
        previous_token: String,
        was_authenticated: bool,
    },
}

/// Authentication state information
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct AuthState {
    pub is_authenticated: bool,
    pub challenge: Option<String>,
    pub session_token: Option<String>,
    pub challenge_timestamp: u64,
}

/// Role of a participant in the authentication process
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub enum AuthRole {
    /// Participant requesting authentication
    Requester,
    /// Participant verifying authentication
    Verifier,
    /// Participant observing the authentication process
    Observer,
    /// Participant acting as an arbiter in disputes
    Arbiter,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub enum Permission {
    CanRequestChallenge,
    CanSubmitResponse,
    CanViewEpisode,
    CanArbitrate,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct ParticipantRole {
    pub pubkey: PubKey,
    pub role: AuthRole,
    pub permissions: Vec<Permission>,
    pub stake: Option<u64>,
}

/// Session information for authenticated users
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct SessionInfo {
    pub token: String,
    pub expires_at: u64,
    pub pubkey: PubKey,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct EnhancedSession {
    pub token: String,
    pub expires_at: u64,
    pub permissions: Vec<Permission>,
    pub metadata: HashMap<String, String>,
    pub refresh_token: Option<String>,
}

/// Rate limiting information
#[derive(Clone, Debug, Default)]
pub struct RateLimitData {
    pub attempts: HashMap<String, u32>,
    pub last_reset: u64,
}

impl RateLimitData {
    pub fn is_rate_limited(&self, pubkey: &PubKey) -> bool {
        let pubkey_str = format!("{}", pubkey);
        self.attempts.get(&pubkey_str).map_or(false, |&attempts| attempts >= 5)
    }
    
    pub fn increment(&mut self, pubkey: &PubKey) {
        let pubkey_str = format!("{}", pubkey);
        *self.attempts.entry(pubkey_str).or_insert(0) += 1;
    }
}