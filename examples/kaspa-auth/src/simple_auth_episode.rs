use borsh::{BorshDeserialize, BorshSerialize};
use kdapp::{
    episode::{Episode, EpisodeError, PayloadMetadata},
    pki::PubKey,
};
use log::info;
use rand::{thread_rng, Rng};
use std::collections::HashMap;

use crate::auth_commands::AuthCommand;

#[derive(Debug, BorshDeserialize, BorshSerialize)]
pub enum AuthError {
    ChallengeNotFound,
    InvalidChallenge,
    SignatureVerificationFailed,
    AlreadyAuthenticated,
    NotAuthorized,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::ChallengeNotFound => write!(f, "Challenge not found for this participant."),
            AuthError::InvalidChallenge => write!(f, "Invalid or expired challenge."),
            AuthError::SignatureVerificationFailed => write!(f, "Signature verification failed."),
            AuthError::AlreadyAuthenticated => write!(f, "Participant is already authenticated."),
            AuthError::NotAuthorized => write!(f, "Participant is not authorized."),
        }
    }
}

impl std::error::Error for AuthError {}

// AuthCommand moved to auth_commands.rs to avoid duplication

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
}

#[derive(Clone, Debug)]
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

impl SimpleAuth {
    fn generate_challenge() -> String {
        let mut rng = thread_rng();
        format!("auth_{}", rng.gen::<u64>())
    }

    fn generate_session_token() -> String {
        let mut rng = thread_rng();
        format!("sess_{}", rng.gen::<u64>())
    }

    fn verify_signature(&self, pubkey: &PubKey, message: &str, signature: &str) -> bool {
        // Use kdapp's built-in verification
        use kdapp::pki::{verify_signature, to_message, Sig};
        use secp256k1::ecdsa::Signature;
        
        // Decode hex signature string to bytes
        let signature_bytes = match hex::decode(signature) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        
        // Convert signature bytes to Signature
        let sig = match Signature::from_der(&signature_bytes) {
            Ok(s) => Sig(s),
            Err(_) => return false,
        };
        
        // Create message for verification (kdapp expects a serializable object)
        let msg = to_message(&message.to_string());
        
        // Verify using kdapp's verification
        verify_signature(pubkey, &msg, &sig)
    }

    fn is_rate_limited(&self, pubkey: &PubKey) -> bool {
        let pubkey_str = format!("{}", pubkey);
        self.rate_limits.get(&pubkey_str).map_or(false, |&attempts| attempts >= 5)
    }

    fn increment_rate_limit(&mut self, pubkey: &PubKey) {
        let pubkey_str = format!("{}", pubkey);
        *self.rate_limits.entry(pubkey_str).or_insert(0) += 1;
    }
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
            return Err(EpisodeError::InvalidCommand(AuthError::NotAuthorized));
        }

        match cmd {
            AuthCommand::RequestChallenge => {
                info!("[SimpleAuth] RequestChallenge from: {:?}", participant);
                
                // Store previous state for rollback
                let previous_challenge = self.challenge.clone();
                let previous_timestamp = self.challenge_timestamp;
                
                // Generate new challenge
                let new_challenge = Self::generate_challenge();
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
                    return Err(EpisodeError::InvalidCommand(AuthError::InvalidChallenge));
                }
                
                // Verify signature
                if !self.verify_signature(&participant, current_challenge, signature) {
                    return Err(EpisodeError::InvalidCommand(AuthError::SignatureVerificationFailed));
                }
                
                // Store previous state for rollback
                let previous_auth_status = self.is_authenticated;
                let previous_session_token = self.session_token.clone();
                
                // Authenticate user
                self.is_authenticated = true;
                self.session_token = Some(Self::generate_session_token());
                
                info!("[SimpleAuth] Authentication successful for: {:?}", participant);
                
                Ok(AuthRollback::Authentication {
                    previous_auth_status,
                    previous_session_token,
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
        }
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
    fn test_auth_full_flow() {
        let ((s1, p1), (_s2, _p2)) = (generate_keypair(), generate_keypair());
        let metadata = PayloadMetadata { 
            accepting_hash: 0u64.into(), 
            accepting_daa: 0, 
            accepting_time: 0, 
            tx_id: 1u64.into() 
        };
        
        let mut auth = SimpleAuth::initialize(vec![p1], &metadata);
        
        // Request challenge
        let _rollback = auth.execute(
            &AuthCommand::RequestChallenge, 
            Some(p1), 
            &metadata
        ).unwrap();
        
        let challenge = auth.challenge.clone().unwrap();
        
        // Sign challenge
        let msg = to_message(&challenge.to_string());
        let sig = sign_message(&s1, &msg);
        
        // Submit response
        let _rollback = auth.execute(
            &AuthCommand::SubmitResponse { 
                signature: hex::encode(sig.0.serialize_der()), 
                nonce: challenge 
            }, 
            Some(p1), 
            &metadata
        ).unwrap();
        
        assert!(auth.is_authenticated);
        assert!(auth.session_token.is_some());
    }
}