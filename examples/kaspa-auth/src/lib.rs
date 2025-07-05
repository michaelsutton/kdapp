// Core working modules
pub mod auth_commands;
pub mod episode_runner;
pub mod core;
pub mod crypto;
pub mod api;

// Framework modules (re-enable anytime)
pub mod cli;
pub mod commitments;
pub mod network;
pub mod storage;
pub mod time_bounded_auth;
pub mod economics;
pub mod state_management;
pub mod oracle;
pub mod events;
pub mod examples;

// Public API exports (only working functionality)
pub use auth_commands::AuthCommand;
pub use episode_runner::{AuthEventHandler, AuthServerConfig, run_auth_server, create_auth_generator};