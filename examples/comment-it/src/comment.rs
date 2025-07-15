use borsh::{BorshDeserialize, BorshSerialize};
use kdapp::{
    episode::{Episode, EpisodeError, PayloadMetadata},
    pki::PubKey,
};
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Commands for the comment episode
#[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum CommentCommand {
    /// Submit a new comment to the blockchain
    SubmitComment {
        text: String,
        author: String, // Public key as string
        session_token: String,
        signature: String,
    },
    /// Get all comments (for authenticated users)
    GetComments {
        session_token: Option<String>,
    },
    /// Get comments by specific author
    GetCommentsByAuthor {
        author: String,
        session_token: Option<String>,
    },
    /// Register a valid authentication session
    RegisterSession {
        public_key: String,
        session_token: String,
        auth_episode_id: u64,
    },
    /// Revoke a session (when user logs out)
    RevokeSession {
        session_token: String,
    },
}

/// Rollback data for comment commands
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum CommentRollback {
    CommentSubmitted {
        comment_id: u64,
    },
    CommentsQueried {
        // No rollback needed for read operations
    },
    SessionRegistered {
        public_key: String,
    },
    SessionRevoked {
        public_key: String,
        session_token: String,
    },
}

/// Error types for comment operations
#[derive(Debug, Clone, Error)]
pub enum CommentError {
    #[error("Invalid session token")]
    InvalidSessionToken,
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    #[error("Comment is too long (max 2000 characters)")]
    CommentTooLong,
    #[error("Comment cannot be empty")]
    CommentEmpty,
    #[error("Not authorized to perform this action")]
    NotAuthorized,
}

/// A single comment stored on the blockchain
#[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct Comment {
    pub id: u64,
    pub text: String,
    pub author: String,
    pub timestamp: u64,
    pub session_token: String,
}

/// Comment episode for storing comments on Kaspa blockchain
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct CommentEpisode {
    /// All comments stored in this episode
    pub comments: Vec<Comment>,
    /// Next comment ID
    pub next_id: u64,
    /// Authorized participants (who can comment)
    pub authorized_participants: Vec<PubKey>,
    /// Timestamp of episode creation
    pub created_at: u64,
    /// Valid authentication sessions (pubkey -> session_token)
    pub valid_sessions: std::collections::HashMap<String, String>,
    /// Associated authentication episode ID (for session validation)
    pub auth_episode_id: Option<u64>,
}

impl Episode for CommentEpisode {
    type Command = CommentCommand;
    type CommandRollback = CommentRollback;
    type CommandError = CommentError;

    fn initialize(participants: Vec<PubKey>, metadata: &PayloadMetadata) -> Self {
        info!("[CommentEpisode] initialize: {:?}", participants);
        Self {
            comments: Vec::new(),
            next_id: 1,
            authorized_participants: participants,
            created_at: metadata.accepting_time,
            valid_sessions: HashMap::new(),
            auth_episode_id: None,
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
            return Err(EpisodeError::InvalidCommand(CommentError::NotAuthorized));
        }

        match cmd {
            CommentCommand::SubmitComment { text, author, session_token, signature: _ } => {
                info!("[CommentEpisode] SubmitComment from: {:?}", participant);
                
                // Basic validation
                if text.trim().is_empty() {
                    return Err(EpisodeError::InvalidCommand(CommentError::CommentEmpty));
                }
                
                if text.len() > 2000 {
                    return Err(EpisodeError::InvalidCommand(CommentError::CommentTooLong));
                }
                
                // CRITICAL: Verify user has valid authentication session
                let participant_key = format!("{}", participant);
                if !self.valid_sessions.contains_key(&participant_key) {
                    info!("[CommentEpisode] Comment rejected: No valid session for {}", participant_key);
                    return Err(EpisodeError::InvalidCommand(CommentError::InvalidSessionToken));
                }
                
                // Verify session token matches
                if let Some(stored_token) = self.valid_sessions.get(&participant_key) {
                    if stored_token != session_token {
                        info!("[CommentEpisode] Comment rejected: Session token mismatch for {}", participant_key);
                        return Err(EpisodeError::InvalidCommand(CommentError::InvalidSessionToken));
                    }
                } else {
                    info!("[CommentEpisode] Comment rejected: No stored session token for {}", participant_key);
                    return Err(EpisodeError::InvalidCommand(CommentError::InvalidSessionToken));
                }
                
                // Authentication passed - create new comment
                let comment = Comment {
                    id: self.next_id,
                    text: text.clone(),
                    author: author.clone(),
                    timestamp: metadata.accepting_time,
                    session_token: session_token.clone(),
                };
                
                // Store comment
                let comment_id = self.next_id;
                self.comments.push(comment);
                self.next_id += 1;
                
                info!("[CommentEpisode] ✅ Comment {} added successfully (authenticated user)", comment_id);
                
                Ok(CommentRollback::CommentSubmitted { comment_id })
            }
            
            CommentCommand::GetComments { session_token: _ } => {
                info!("[CommentEpisode] GetComments from: {:?}", participant);
                
                // For now, allow only authenticated users to read comments
                // TODO: When profile episode is implemented, support anonymous users
                
                Ok(CommentRollback::CommentsQueried {})
            }
            
            CommentCommand::GetCommentsByAuthor { author, session_token: _ } => {
                info!("[CommentEpisode] GetCommentsByAuthor {} from: {:?}", author, participant);
                
                // For now, allow only authenticated users to read comments
                // TODO: When profile episode is implemented, support anonymous users
                
                Ok(CommentRollback::CommentsQueried {})
            }
            
            CommentCommand::RegisterSession { public_key, session_token, auth_episode_id } => {
                info!("[CommentEpisode] RegisterSession for {} from auth episode {}", public_key, auth_episode_id);
                
                // Store the valid session
                self.valid_sessions.insert(public_key.clone(), session_token.clone());
                if self.auth_episode_id.is_none() {
                    self.auth_episode_id = Some(*auth_episode_id);
                }
                
                info!("[CommentEpisode] ✅ Session registered for {}", public_key);
                
                Ok(CommentRollback::SessionRegistered { public_key: public_key.clone() })
            }
            
            CommentCommand::RevokeSession { session_token } => {
                info!("[CommentEpisode] RevokeSession for token: {}", session_token);
                
                // Find and remove the session
                let mut revoked_key = None;
                for (key, token) in &self.valid_sessions {
                    if token == session_token {
                        revoked_key = Some(key.clone());
                        break;
                    }
                }
                
                if let Some(key) = revoked_key {
                    self.valid_sessions.remove(&key);
                    info!("[CommentEpisode] ✅ Session revoked for {}", key);
                    
                    Ok(CommentRollback::SessionRevoked { 
                        public_key: key, 
                        session_token: session_token.clone() 
                    })
                } else {
                    info!("[CommentEpisode] Session revocation failed: token not found");
                    Err(EpisodeError::InvalidCommand(CommentError::InvalidSessionToken))
                }
            }
        }
    }

    fn rollback(&mut self, rollback: Self::CommandRollback) -> bool {
        match rollback {
            CommentRollback::CommentSubmitted { comment_id } => {
                // Remove the comment that was just added
                if let Some(pos) = self.comments.iter().position(|c| c.id == comment_id) {
                    self.comments.remove(pos);
                    self.next_id = comment_id; // Reset next_id
                    true
                } else {
                    false
                }
            }
            CommentRollback::CommentsQueried {} => {
                // No rollback needed for read operations
                true
            }
            CommentRollback::SessionRegistered { public_key } => {
                // Remove the session that was just registered
                self.valid_sessions.remove(&public_key);
                true
            }
            CommentRollback::SessionRevoked { public_key, session_token } => {
                // Restore the session that was just revoked
                self.valid_sessions.insert(public_key, session_token);
                true
            }
        }
    }
}

impl CommentEpisode {
    /// Get all comments in chronological order
    pub fn get_comments(&self) -> &Vec<Comment> {
        &self.comments
    }
    
    /// Get comments by a specific author
    pub fn get_comments_by_author(&self, author: &str) -> Vec<&Comment> {
        self.comments.iter().filter(|c| c.author == author).collect()
    }
    
    /// Get the latest N comments
    pub fn get_latest_comments(&self, limit: usize) -> Vec<&Comment> {
        let mut comments: Vec<&Comment> = self.comments.iter().collect();
        comments.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        comments.into_iter().take(limit).collect()
    }
    
    /// Check if a user has a valid session
    pub fn has_valid_session(&self, public_key: &str) -> bool {
        self.valid_sessions.contains_key(public_key)
    }
    
    /// Get the session token for a user
    pub fn get_session_token(&self, public_key: &str) -> Option<&String> {
        self.valid_sessions.get(public_key)
    }
    
    /// Get count of authenticated users
    pub fn authenticated_user_count(&self) -> usize {
        self.valid_sessions.len()
    }
    
    /// Check if user can comment (has valid session)
    pub fn can_comment(&self, public_key: &str, session_token: &str) -> bool {
        if let Some(stored_token) = self.valid_sessions.get(public_key) {
            stored_token == session_token
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kdapp::pki::generate_keypair;

    #[test]
    fn test_comment_episode_initialization() {
        let ((_s1, p1), (_s2, p2)) = (generate_keypair(), generate_keypair());
        let metadata = PayloadMetadata { 
            accepting_hash: 0u64.into(), 
            accepting_daa: 0, 
            accepting_time: 1234567890, 
            tx_id: 1u64.into() 
        };
        
        let episode = CommentEpisode::initialize(vec![p1, p2], &metadata);
        
        assert_eq!(episode.comments.len(), 0);
        assert_eq!(episode.next_id, 1);
        assert_eq!(episode.authorized_participants.len(), 2);
        assert_eq!(episode.created_at, 1234567890);
    }

    #[test]
    fn test_submit_comment() {
        let ((_s1, p1), (_s2, _p2)) = (generate_keypair(), generate_keypair());
        let metadata = PayloadMetadata { 
            accepting_hash: 0u64.into(), 
            accepting_daa: 0, 
            accepting_time: 1234567890, 
            tx_id: 1u64.into() 
        };
        
        let mut episode = CommentEpisode::initialize(vec![p1], &metadata);
        
        // Submit a comment
        let cmd = CommentCommand::SubmitComment {
            text: "Hello blockchain!".to_string(),
            author: "test_author".to_string(),
            session_token: "sess_123".to_string(),
            signature: "test_sig".to_string(),
        };
        
        let rollback = episode.execute(&cmd, Some(p1), &metadata).unwrap();
        
        assert_eq!(episode.comments.len(), 1);
        assert_eq!(episode.comments[0].text, "Hello blockchain!");
        assert_eq!(episode.comments[0].id, 1);
        assert_eq!(episode.next_id, 2);
        
        // Test rollback
        episode.rollback(rollback);
        assert_eq!(episode.comments.len(), 0);
        assert_eq!(episode.next_id, 1);
    }

    #[test]
    fn test_empty_comment_rejected() {
        let ((_s1, p1), (_s2, _p2)) = (generate_keypair(), generate_keypair());
        let metadata = PayloadMetadata { 
            accepting_hash: 0u64.into(), 
            accepting_daa: 0, 
            accepting_time: 1234567890, 
            tx_id: 1u64.into() 
        };
        
        let mut episode = CommentEpisode::initialize(vec![p1], &metadata);
        
        // Try to submit empty comment
        let cmd = CommentCommand::SubmitComment {
            text: "   ".to_string(), // Just whitespace
            author: "test_author".to_string(),
            session_token: "sess_123".to_string(),
            signature: "test_sig".to_string(),
        };
        
        let result = episode.execute(&cmd, Some(p1), &metadata);
        assert!(result.is_err());
    }

    #[test]
    fn test_comment_too_long_rejected() {
        let ((_s1, p1), (_s2, _p2)) = (generate_keypair(), generate_keypair());
        let metadata = PayloadMetadata { 
            accepting_hash: 0u64.into(), 
            accepting_daa: 0, 
            accepting_time: 1234567890, 
            tx_id: 1u64.into() 
        };
        
        let mut episode = CommentEpisode::initialize(vec![p1], &metadata);
        
        // Try to submit very long comment
        let long_text = "a".repeat(2001); // Over 2000 character limit
        let cmd = CommentCommand::SubmitComment {
            text: long_text,
            author: "test_author".to_string(),
            session_token: "sess_123".to_string(),
            signature: "test_sig".to_string(),
        };
        
        let result = episode.execute(&cmd, Some(p1), &metadata);
        assert!(result.is_err());
    }
}