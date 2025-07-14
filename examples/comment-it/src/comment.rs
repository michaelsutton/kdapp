use borsh::{BorshDeserialize, BorshSerialize};
use kdapp::{
    episode::{Episode, EpisodeError, PayloadMetadata},
    pki::PubKey,
};
use log::info;
use serde::{Deserialize, Serialize};
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
}

/// Rollback data for comment commands
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum CommentRollback {
    CommentSubmitted {
        comment_id: u64,
    },
}

/// Error types for comment operations
#[derive(Debug, Clone, Error)]
pub enum CommentError {
    #[error("Invalid session token")]
    InvalidSessionToken,
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    #[error("Comment is too long (max 1000 characters)")]
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
                
                if text.len() > 1000 {
                    return Err(EpisodeError::InvalidCommand(CommentError::CommentTooLong));
                }
                
                // TODO: Verify session token with kaspa-auth
                // TODO: Verify signature
                
                // Create new comment
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
                
                info!("[CommentEpisode] Comment {} added successfully", comment_id);
                
                Ok(CommentRollback::CommentSubmitted { comment_id })
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
        let long_text = "a".repeat(1001); // Over 1000 character limit
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