use sha2::{Sha256, Digest};

/// Commitment-reveal pattern utilities for secure multi-party protocols
pub struct CommitmentScheme;

impl CommitmentScheme {
    /// Create a commitment to a value with a nonce
    pub fn commit(value: &str, nonce: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(value.as_bytes());
        hasher.update(nonce.as_bytes());
        hex::encode(hasher.finalize())
    }
    
    /// Verify a commitment against a value and nonce
    pub fn verify(commitment: &str, value: &str, nonce: &str) -> bool {
        let expected = Self::commit(value, nonce);
        commitment == expected
    }
    
    /// Generate a random nonce for commitments
    pub fn generate_nonce() -> String {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        format!("nonce_{}", rng.gen::<u64>())
    }
    
    /// Create a commitment with auto-generated nonce
    pub fn commit_with_nonce(value: &str) -> (String, String) {
        let nonce = Self::generate_nonce();
        let commitment = Self::commit(value, &nonce);
        (commitment, nonce)
    }
}

/// Commitment-reveal protocol for secure auctions, voting, etc.
pub struct CommitRevealProtocol {
    commitments: std::collections::HashMap<String, String>,
    reveals: std::collections::HashMap<String, (String, String)>,
}

impl CommitRevealProtocol {
    pub fn new() -> Self {
        Self {
            commitments: std::collections::HashMap::new(),
            reveals: std::collections::HashMap::new(),
        }
    }
    
    /// Add a commitment from a participant
    pub fn add_commitment(&mut self, participant: &str, commitment: &str) {
        self.commitments.insert(participant.to_string(), commitment.to_string());
    }
    
    /// Add a reveal from a participant
    pub fn add_reveal(&mut self, participant: &str, value: &str, nonce: &str) -> bool {
        if let Some(commitment) = self.commitments.get(participant) {
            if CommitmentScheme::verify(commitment, value, nonce) {
                self.reveals.insert(participant.to_string(), (value.to_string(), nonce.to_string()));
                return true;
            }
        }
        false
    }
    
    /// Check if all participants have revealed
    pub fn all_revealed(&self) -> bool {
        self.commitments.len() == self.reveals.len() && !self.commitments.is_empty()
    }
    
    /// Get all revealed values
    pub fn get_reveals(&self) -> Vec<(String, String)> {
        self.reveals.iter()
            .map(|(participant, (value, _))| (participant.clone(), value.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_creation_and_verification() {
        let value = "secret_value";
        let nonce = "random_nonce";
        
        let commitment = CommitmentScheme::commit(value, nonce);
        assert!(CommitmentScheme::verify(&commitment, value, nonce));
        
        // Should fail with wrong value
        assert!(!CommitmentScheme::verify(&commitment, "wrong_value", nonce));
        
        // Should fail with wrong nonce
        assert!(!CommitmentScheme::verify(&commitment, value, "wrong_nonce"));
    }

    #[test]
    fn test_commitment_with_auto_nonce() {
        let value = "secret_value";
        let (commitment, nonce) = CommitmentScheme::commit_with_nonce(value);
        
        assert!(CommitmentScheme::verify(&commitment, value, &nonce));
    }

    #[test]
    fn test_commit_reveal_protocol() {
        let mut protocol = CommitRevealProtocol::new();
        
        // Alice commits to "bid_100"
        let alice_value = "bid_100";
        let (alice_commitment, alice_nonce) = CommitmentScheme::commit_with_nonce(alice_value);
        protocol.add_commitment("alice", &alice_commitment);
        
        // Bob commits to "bid_150"
        let bob_value = "bid_150";
        let (bob_commitment, bob_nonce) = CommitmentScheme::commit_with_nonce(bob_value);
        protocol.add_commitment("bob", &bob_commitment);
        
        assert!(!protocol.all_revealed());
        
        // Alice reveals
        assert!(protocol.add_reveal("alice", alice_value, &alice_nonce));
        assert!(!protocol.all_revealed());
        
        // Bob reveals
        assert!(protocol.add_reveal("bob", bob_value, &bob_nonce));
        assert!(protocol.all_revealed());
        
        let reveals = protocol.get_reveals();
        assert_eq!(reveals.len(), 2);
        assert!(reveals.contains(&("alice".to_string(), alice_value.to_string())));
        assert!(reveals.contains(&("bob".to_string(), bob_value.to_string())));
    }
    
    #[test]
    fn test_invalid_reveal() {
        let mut protocol = CommitRevealProtocol::new();
        
        let value = "secret_value";
        let (commitment, _nonce) = CommitmentScheme::commit_with_nonce(value);
        protocol.add_commitment("alice", &commitment);
        
        // Try to reveal with wrong nonce
        assert!(!protocol.add_reveal("alice", value, "wrong_nonce"));
        assert!(!protocol.all_revealed());
    }
}