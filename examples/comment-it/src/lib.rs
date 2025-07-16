// Core working modules
pub mod episode_runner;
pub mod core;
pub mod crypto;
pub mod api;
pub mod comment;
pub mod organizer;

// Framework modules (re-enable anytime)
pub mod cli;
pub mod wallet;
pub mod auth;

// Future modules (moved to future examples)
// pub mod commitments;     // → kaspa-poker-tournament
// pub mod economics;       // → kaspa-poker-tournament  
// pub mod oracle;          // → episode-contract
// pub mod time_bounded_auth; // → episode-contract
// pub mod state_management; // → episode-contract
// pub mod network;         // → future networking example
// pub mod storage;         // → future storage example
// pub mod examples;        // → individual example projects

// Public API exports (only working functionality)
pub use core::commands::AuthCommand;
pub use episode_runner::{AuthEventHandler, AuthServerConfig, run_auth_server, create_auth_generator};
pub use auth::{run_http_coordinated_authentication, run_session_revocation, AuthenticationResult};