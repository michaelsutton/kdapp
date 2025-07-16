// Commitment-Reveal Pattern for Future Poker Implementation
// This demonstrates the pattern that will be used for secure card dealing

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use rand::{thread_rng, Rng};

/// Commitment-reveal challenge for demonstrating the pattern
/// This will be expanded for poker card dealing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRevealChallenge {
    /// The commitment hash (sent first)
    pub commitment: String,
    /// The actual value (revealed later) 
    pub reveal_value: Option<String>,
    /// The nonce used for commitment (revealed with value)
    pub reveal_nonce: Option<String>,
    /// Whether this commitment has been revealed
    pub is_revealed: bool,
}

impl CommitRevealChallenge {
    /// Create a new commitment-reveal challenge
    /// This pattern will be used for poker to commit to card shuffles
    pub fn new(value: &str) -> Self {
        let nonce = generate_nonce();
        let commitment = create_commitment(value, &nonce);
        
        Self {
            commitment,
            reveal_value: Some(value.to_string()),
            reveal_nonce: Some(nonce),
            is_revealed: false,
        }
    }
    
    /// Create a commitment without storing the reveal data
    /// Used when only the commitment is needed initially
    pub fn commit_only(value: &str) -> (String, String) {
        let nonce = generate_nonce();
        let commitment = create_commitment(value, &nonce);
        (commitment, nonce)
    }
    
    /// Verify that a revealed value matches the commitment
    /// Critical for poker - ensures cards can't be changed after commitment
    pub fn verify_reveal(&self, revealed_value: &str, revealed_nonce: &str) -> bool {
        let expected_commitment = create_commitment(revealed_value, revealed_nonce);
        expected_commitment == self.commitment
    }
    
    /// Reveal the committed value
    /// In poker, this happens when cards need to be shown
    pub fn reveal(&mut self) -> Result<(String, String), &'static str> {
        if self.is_revealed {
            return Err("Already revealed");
        }
        
        match (&self.reveal_value, &self.reveal_nonce) {
            (Some(value), Some(nonce)) => {
                self.is_revealed = true;
                Ok((value.clone(), nonce.clone()))
            }
            _ => Err("No reveal data available"),
        }
    }
}

/// Generate a cryptographically secure nonce
fn generate_nonce() -> String {
    let mut rng = thread_rng();
    format!("nonce_{}", rng.gen::<u64>())
}

/// Create a commitment hash from value and nonce
/// Uses SHA256 for cryptographic security
fn create_commitment(value: &str, nonce: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hasher.update(nonce.as_bytes());
    format!("commit_{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_reveal_cycle() {
        let original_value = "auth_challenge_12345";
        let mut challenge = CommitRevealChallenge::new(original_value);
        
        // Should not be revealed initially
        assert!(!challenge.is_revealed);
        
        // Reveal should work
        let (revealed_value, revealed_nonce) = challenge.reveal().unwrap();
        assert_eq!(revealed_value, original_value);
        assert!(challenge.is_revealed);
        
        // Should be able to verify the reveal
        assert!(challenge.verify_reveal(&revealed_value, &revealed_nonce));
    }
    
    #[test]
    fn test_commitment_verification() {
        let value = "test_value";
        let (commitment, nonce) = CommitRevealChallenge::commit_only(value);
        
        let challenge = CommitRevealChallenge {
            commitment: commitment.clone(),
            reveal_value: None,
            reveal_nonce: None,
            is_revealed: false,
        };
        
        // Correct value and nonce should verify
        assert!(challenge.verify_reveal(value, &nonce));
        
        // Incorrect value should not verify
        assert!(!challenge.verify_reveal("wrong_value", &nonce));
        
        // Incorrect nonce should not verify
        assert!(!challenge.verify_reveal(value, "wrong_nonce"));
    }
}

// Future Poker Usage Example:
/*
use commitment_reveal::CommitRevealChallenge;

pub struct PokerDealer {
    deck_commitment: CommitRevealChallenge,
    // ... other fields
}

impl PokerDealer {
    pub fn new() -> Self {
        // Shuffle deck and commit to the order
        let shuffled_deck = shuffle_deck();
        let deck_commitment = CommitRevealChallenge::new(&serialize_deck(&shuffled_deck));
        
        Self {
            deck_commitment,
        }
    }
    
    pub fn reveal_cards(&mut self, count: usize) -> Result<Vec<Card>, &'static str> {
        // Reveal cards from the committed deck
        let (deck_data, nonce) = self.deck_commitment.reveal()?;
        let deck = deserialize_deck(&deck_data);
        Ok(deck.into_iter().take(count).collect())
    }
}
*/