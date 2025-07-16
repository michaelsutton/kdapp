pub mod commands;
pub mod config;
pub mod utils;

use clap::{Parser, Subcommand};
use commands::*;

#[derive(Parser)]
#[command(name = "kaspa-auth")]
#[command(version = "0.1.0")]
#[command(about = "Kaspa Authentication Episode Demo")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Test auth episode locally (no Kaspa)
    TestEpisode(test::TestEpisodeCommand),
    /// Run HTTP coordination organizer peer for authentication
    HttpOrganizerPeer(http_organizer_peer::HttpOrganizerPeerCommand),
    /// 🚀 One-command authentication with HTTP server (EASY MODE)
    Authenticate(authenticate::AuthenticateCommand),
    /// 🔄 Complete login → session → logout cycle with timeouts
    AuthenticateFullFlow(authenticate_full_flow::AuthenticateFullFlowCommand),
    /// Run interactive demo
    Demo(demo::DemoCommand),
    /// Run auth organizer peer on Kaspa testnet-10
    OrganizerPeer(organizer_peer::OrganizerPeerCommand),
    /// Run auth participant peer on Kaspa testnet-10
    ParticipantPeer(participant_peer::ParticipantPeerCommand),
}

impl Commands {
    pub async fn execute(self) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Commands::TestEpisode(cmd) => cmd.execute().await,
            Commands::HttpOrganizerPeer(cmd) => cmd.execute().await,
            Commands::Authenticate(cmd) => cmd.execute().await,
            Commands::AuthenticateFullFlow(cmd) => cmd.execute().await,
            Commands::Demo(cmd) => cmd.execute().await,
            Commands::OrganizerPeer(cmd) => cmd.execute().await,
            Commands::ParticipantPeer(cmd) => cmd.execute().await,
        }
    }
}
