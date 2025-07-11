use borsh::{BorshDeserialize, BorshSerialize};
use kdapp::{
    episode::{Episode, EpisodeError, PayloadMetadata},
    pki::PubKey,
};
use log::info;
use std::collections::HashMap;

use crate::core::{AuthCommand, AuthError, AuthRollback};
use crate::crypto::challenges::ChallengeGenerator;
use crate::crypto::signatures::SignatureVerifier;

/// Simple authentication episode for Kaspa
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct SimpleAuth {
    /// Owner public key (the one being authenticated)
    pub owner: Option<PubKey>,
    /// Current challenge string for authentication
    pub challenge: Option<String>,
    /// Whether the owner is authenticated
    pub is_authenticated: bool,
    /// Session token for authenticated users
    pub session_token: Option<String>,
    /// Timestamp of last challenge generation
    pub challenge_timestamp: u64,
    /// In-memory rate limiting: attempts per pubkey (using string representation)
    pub rate_limits: HashMap<String, u32>,
    /// Authorized participants (who can request challenges)
    pub authorized_participants: Vec<PubKey>,
}




impl Episode for SimpleAuth {
    type Command = AuthCommand;
    type CommandRollback = AuthRollback;
    type CommandError = AuthError;

    fn initialize(participants: Vec<PubKey>, metadata: &PayloadMetadata) -> Self {
        info!("[SimpleAuth] initialize: {:?}", participants);
        Self {
            owner: participants.first().copied(),
            challenge: None,
            is_authenticated: false,
            session_token: None,
            challenge_timestamp: metadata.accepting_time,
            rate_limits: HashMap::new(),
            authorized_participants: participants,
        }
    }

    fn execute(
        &mut self,
        cmd: &Self::Command,
        authorization: Option<PubKey>,
        metadata: &PayloadMetadata,
    ) -> Result<Self::CommandRollback, EpisodeError<Self::CommandError>> {
        let Some(participant) = authorization else {
            return Err(EpisodeError::Unauthorized);
        };

        // Check if participant is authorized
        if !self.authorized_participants.contains(&participant) {
            return Err(EpisodeError::InvalidCommand(AuthError::NotAuthorized));
        }

        // Rate limiting check
        if self.is_rate_limited(&participant) {
            return Err(EpisodeError::InvalidCommand(AuthError::RateLimited));
        }

        match cmd {
            AuthCommand::RequestChallenge => {
                info!("[SimpleAuth] RequestChallenge from: {:?}", participant);
                
                // Store previous state for rollback
                let previous_challenge = self.challenge.clone();
                let previous_timestamp = self.challenge_timestamp;
                
                // Generate new challenge with timestamp from metadata
                let new_challenge = ChallengeGenerator::generate_with_provided_timestamp(metadata.accepting_time);
                self.challenge = Some(new_challenge);
                self.challenge_timestamp = metadata.accepting_time;
                self.owner = Some(participant);
                
                // Increment rate limit
                self.increment_rate_limit(&participant);
                
                Ok(AuthRollback::Challenge { 
                    previous_challenge, 
                    previous_timestamp 
                })
            }
            
            AuthCommand::SubmitResponse { signature, nonce } => {
                info!("[SimpleAuth] SubmitResponse from: {:?}", participant);
                
                // Check if already authenticated
                if self.is_authenticated {
                    return Err(EpisodeError::InvalidCommand(AuthError::AlreadyAuthenticated));
                }
                
                // Check if challenge exists and matches
                let Some(ref current_challenge) = self.challenge else {
                    return Err(EpisodeError::InvalidCommand(AuthError::ChallengeNotFound));
                };
                
                if *nonce != *current_challenge {
                    info!("[SimpleAuth] Challenge mismatch - received: '{}', expected: '{}'", nonce, current_challenge);
                    return Err(EpisodeError::InvalidCommand(AuthError::InvalidChallenge));
                }
                
                // Check if challenge has expired (1 hour timeout)
                if !ChallengeGenerator::is_valid(current_challenge, 3600) {
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                    info!("[SimpleAuth] Challenge expired: {} (current time: {})", current_challenge, now);
                    return Err(EpisodeError::InvalidCommand(AuthError::ChallengeExpired));
                }
                
                // Verify signature
                if !SignatureVerifier::verify(&participant, current_challenge, signature) {
                    return Err(EpisodeError::InvalidCommand(AuthError::SignatureVerificationFailed));
                }
                
                // Store previous state for rollback
                let previous_auth_status = self.is_authenticated;
                let previous_session_token = self.session_token.clone();
                
                // Authenticate user
                self.is_authenticated = true;
                self.session_token = Some(self.generate_session_token());
                
                info!("[SimpleAuth] Authentication successful for: {:?}", participant);
                
                Ok(AuthRollback::Authentication {
                    previous_auth_status,
                    previous_session_token,
                })
            }
            
            AuthCommand::RevokeSession { session_token, signature } => {
                info!("[SimpleAuth] RevokeSession from: {:?}", participant);
                
                // Check if session exists and matches
                let Some(ref current_token) = self.session_token else {
                    return Err(EpisodeError::InvalidCommand(AuthError::SessionNotFound));
                };
                
                if *session_token != *current_token {
                    return Err(EpisodeError::InvalidCommand(AuthError::InvalidSessionToken));
                }
                
                // Check if already not authenticated (session already revoked)
                if !self.is_authenticated {
                    return Err(EpisodeError::InvalidCommand(AuthError::SessionAlreadyRevoked));
                }
                
                // Verify signature - participant must sign their own session token to prove ownership
                if !SignatureVerifier::verify(&participant, session_token, signature) {
                    return Err(EpisodeError::InvalidCommand(AuthError::SignatureVerificationFailed));
                }
                
                // Store previous state for rollback
                let previous_token = self.session_token.clone().unwrap();
                let was_authenticated = self.is_authenticated;
                
                // Revoke session
                self.is_authenticated = false;
                self.session_token = None;
                
                info!("[SimpleAuth] Session revoked successfully for: {:?}", participant);
                
                Ok(AuthRollback::SessionRevoked {
                    previous_token,
                    was_authenticated,
                })
            }
            
        }
    }

    fn rollback(&mut self, rollback: Self::CommandRollback) -> bool {
        match rollback {
            AuthRollback::Challenge { previous_challenge, previous_timestamp } => {
                self.challenge = previous_challenge;
                self.challenge_timestamp = previous_timestamp;
                // Note: We don't rollback rate limits as they should persist
                true
            }
            AuthRollback::Authentication { previous_auth_status, previous_session_token } => {
                self.is_authenticated = previous_auth_status;
                self.session_token = previous_session_token;
                true
            }
            AuthRollback::SessionRevoked { previous_token, was_authenticated } => {
                self.is_authenticated = was_authenticated;
                self.session_token = Some(previous_token);
                true
            }
        }
    }
}

impl SimpleAuth {

    /// Check if a participant is rate limited
    fn is_rate_limited(&self, pubkey: &PubKey) -> bool {
        let pubkey_str = format!("{}", pubkey);
        self.rate_limits.get(&pubkey_str).map_or(false, |&attempts| attempts >= 5)
    }

    /// Increment rate limit counter for a participant
    fn increment_rate_limit(&mut self, pubkey: &PubKey) {
        let pubkey_str = format!("{}", pubkey);
        *self.rate_limits.entry(pubkey_str).or_insert(0) += 1;
    }

    /// Generate a new session token
    fn generate_session_token(&self) -> String {
        use rand_chacha::ChaCha8Rng;
        use rand::SeedableRng;
        use rand::Rng;
        let mut rng = ChaCha8Rng::seed_from_u64(self.challenge_timestamp);
        format!("sess_{}", rng.gen::<u64>())
    }

}





#[cfg(test)]
mod tests {
    use super::*;
    use kdapp::pki::{generate_keypair, sign_message, to_message};

    #[test]
    fn test_auth_challenge_flow() {
        let ((_s1, p1), (_s2, _p2)) = (generate_keypair(), generate_keypair());
        let metadata = PayloadMetadata { 
            accepting_hash: 0u64.into(), 
            accepting_daa: 0, 
            accepting_time: 0, 
            tx_id: 1u64.into() 
        };
        
        let mut auth = SimpleAuth::initialize(vec![p1], &metadata);
        
        // Request challenge
        let rollback = auth.execute(
            &AuthCommand::RequestChallenge, 
            Some(p1), 
            &metadata
        ).unwrap();
        
        assert!(auth.challenge.is_some());
        assert!(!auth.is_authenticated);
        
        // Test rollback
        auth.rollback(rollback);
        assert!(auth.challenge.is_none());
    }

    

    #[test]
    fn test_rate_limiting() {
        let ((_s1, p1), (_s2, _p2)) = (generate_keypair(), generate_keypair());
        let metadata = PayloadMetadata { 
            accepting_hash: 0u64.into(), 
            accepting_daa: 0, 
            accepting_time: 0, 
            tx_id: 1u64.into() 
        };
        
        let mut auth = SimpleAuth::initialize(vec![p1], &metadata);
        
        // Should not be rate limited initially
        assert!(!auth.is_rate_limited(&p1));
        
        // Make 4 requests - should still work
        for _ in 0..4 {
            auth.execute(&AuthCommand::RequestChallenge, Some(p1), &metadata).unwrap();
        }
        assert!(!auth.is_rate_limited(&p1));
        
        // 5th request should trigger rate limit
        auth.execute(&AuthCommand::RequestChallenge, Some(p1), &metadata).unwrap();
        assert!(auth.is_rate_limited(&p1));
        
        // 6th request should be rejected
        let result = auth.execute(&AuthCommand::RequestChallenge, Some(p1), &metadata);
        assert!(result.is_err());
    }
}