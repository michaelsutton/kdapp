use rand::{thread_rng, Rng};

/// Challenge generation utilities
pub struct ChallengeGenerator;

impl ChallengeGenerator {
    /// Generate a new random challenge
    pub fn generate() -> String {
        let mut rng = thread_rng();
        format!("auth_{}", rng.gen::<u64>())
    }
    
    /// Generate a challenge with a custom prefix
    pub fn generate_with_prefix(prefix: &str) -> String {
        let mut rng = thread_rng();
        format!("{}_{}", prefix, rng.gen::<u64>())
    }
    
    /// Generate a challenge with timestamp for expiry
    pub fn generate_with_timestamp() -> (String, u64) {
        let mut rng = thread_rng();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let challenge = format!("auth_{}_{}", timestamp, rng.gen::<u64>());
        (challenge, timestamp)
    }
    
    /// Validate if a challenge is still valid (not expired)
    pub fn is_valid(challenge: &str, max_age_seconds: u64) -> bool {
        if let Some(timestamp_str) = challenge.strip_prefix("auth_").and_then(|s| s.split('_').next()) {
            if let Ok(timestamp) = timestamp_str.parse::<u64>() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                return now.saturating_sub(timestamp) <= max_age_seconds;
            }
        }
        // If we can't parse timestamp, assume it's a simple challenge (always valid)
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_challenge_generation() {
        let challenge = ChallengeGenerator::generate();
        assert!(challenge.starts_with("auth_"));
        assert!(challenge.len() > 5);
    }

    #[test]
    fn test_prefixed_challenge_generation() {
        let challenge = ChallengeGenerator::generate_with_prefix("test");
        assert!(challenge.starts_with("test_"));
    }

    #[test]
    fn test_timestamped_challenge_generation() {
        let (challenge, timestamp) = ChallengeGenerator::generate_with_timestamp();
        assert!(challenge.starts_with("auth_"));
        assert!(challenge.contains(&timestamp.to_string()));
    }

    #[test]
    fn test_challenge_validation() {
        let (challenge, _) = ChallengeGenerator::generate_with_timestamp();
        assert!(ChallengeGenerator::is_valid(&challenge, 300)); // 5 minutes
        
        // Test basic challenge (should always be valid)
        let basic_challenge = ChallengeGenerator::generate();
        assert!(ChallengeGenerator::is_valid(&basic_challenge, 300));
    }
}