use borsh::{BorshDeserialize, BorshSerialize};

/// Authentication-specific errors
#[derive(Debug, BorshDeserialize, BorshSerialize)]
pub enum AuthError {
    ChallengeNotFound,
    InvalidChallenge,
    SignatureVerificationFailed,
    AlreadyAuthenticated,
    NotAuthorized,
    RateLimited,
    InvalidSignature,
    ChallengeExpired,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::ChallengeNotFound => write!(f, "Challenge not found for this participant."),
            AuthError::InvalidChallenge => write!(f, "Invalid or expired challenge."),
            AuthError::SignatureVerificationFailed => write!(f, "Signature verification failed."),
            AuthError::AlreadyAuthenticated => write!(f, "Participant is already authenticated."),
            AuthError::NotAuthorized => write!(f, "Participant is not authorized."),
            AuthError::RateLimited => write!(f, "Rate limit exceeded. Please try again later."),
            AuthError::InvalidSignature => write!(f, "Invalid signature format."),
            AuthError::ChallengeExpired => write!(f, "Challenge has expired."),
        }
    }
}

impl std::error::Error for AuthError {}

/// Authentication result type
pub type AuthResult<T> = Result<T, AuthError>;