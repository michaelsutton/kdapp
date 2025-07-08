use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthEvent {
    EpisodeCreated { #[serde_as(as = "DisplayFromStr")] episode_id: u64, participants: Vec<String> },
    ChallengeIssued { #[serde_as(as = "DisplayFromStr")] episode_id: u64, challenge: String, requester: String },
    AuthenticationAttempted { #[serde_as(as = "DisplayFromStr")] episode_id: u64, success: bool, participant: String },
    SessionCreated { #[serde_as(as = "DisplayFromStr")] episode_id: u64, session_token: String, expires_at: u64 },
    OracleDataSubmitted { #[serde_as(as = "DisplayFromStr")] episode_id: u64, data_source: String, data_hash: String },
    EpisodeExpired { #[serde_as(as = "DisplayFromStr")] episode_id: u64 },
}

#[derive(Clone)]
pub struct EventEmitter {
    sender: broadcast::Sender<AuthEvent>,
}

impl EventEmitter {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self { sender }
    }
    
    pub fn emit(&self, event: AuthEvent) {
        let _ = self.sender.send(event);
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<AuthEvent> {
        self.sender.subscribe()
    }
}

impl AuthEvent {
    pub fn from_command(cmd: &crate::core::commands::AuthCommand, episode_id: u64) -> Self {
        match cmd {
            crate::core::commands::AuthCommand::RequestChallenge => {
                AuthEvent::ChallengeIssued { 
                    episode_id, 
                    challenge: "generated".to_string(),
                    requester: "unknown".to_string() 
                }
            },
            crate::core::commands::AuthCommand::SubmitResponse { .. } => {
                AuthEvent::AuthenticationAttempted { 
                    episode_id, 
                    success: true,
                    participant: "unknown".to_string() 
                }
            },
        }
    }
}
