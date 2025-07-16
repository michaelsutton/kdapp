use serde::{Deserialize, Serialize};
use borsh::{BorshDeserialize, BorshSerialize};

/// Commands for the Kaspa authentication episode
#[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum AuthCommand {
    /// Request a challenge from the server
    RequestChallenge,
    /// Submit response with signature and nonce
    SubmitResponse {
        signature: String,
        nonce: String,
    },
    /// Revoke an existing session
    RevokeSession {
        session_token: String,
        signature: String,
    },
}

impl AuthCommand {
    /// Get the command type as a string for logging/debugging
    pub fn command_type(&self) -> &'static str {
        match self {
            AuthCommand::RequestChallenge => "RequestChallenge",
            AuthCommand::SubmitResponse { .. } => "SubmitResponse",
            AuthCommand::RevokeSession { .. } => "RevokeSession",
        }
    }
    
    /// Check if command requires authentication
    pub fn requires_auth(&self) -> bool {
        match self {
            AuthCommand::RequestChallenge => false,
            AuthCommand::SubmitResponse { .. } => true,
            AuthCommand::RevokeSession { .. } => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_challenge_command() {
        let cmd = AuthCommand::RequestChallenge;
        assert_eq!(cmd.command_type(), "RequestChallenge");
        assert!(!cmd.requires_auth());
    }

    #[test]
    fn test_submit_response_command() {
        let cmd = AuthCommand::SubmitResponse {
            signature: "test_signature".to_string(),
            nonce: "test_nonce".to_string(),
        };
        assert_eq!(cmd.command_type(), "SubmitResponse");
        assert!(cmd.requires_auth());
    }

    #[test]
    fn test_serialization() {
        let cmd = AuthCommand::SubmitResponse {
            signature: "sig123".to_string(),
            nonce: "nonce456".to_string(),
        };
        
        // Test that we can serialize and deserialize
        let serialized = serde_json::to_string(&cmd).unwrap();
        let deserialized: AuthCommand = serde_json::from_str(&serialized).unwrap();
        
        match deserialized {
            AuthCommand::SubmitResponse { signature, nonce } => {
                assert_eq!(signature, "sig123");
                assert_eq!(nonce, "nonce456");
            }
            _ => panic!("Expected SubmitResponse"),
        }
    }
}