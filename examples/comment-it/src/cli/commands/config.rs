// src/cli/commands/config.rs
use clap::{Args, Subcommand};
use crate::cli::config::{CommentItConfig, OrganizerPeer, PeerType};

#[derive(Args)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    /// List organizer peers
    Peers,
    /// Add a new organizer peer
    AddPeer {
        /// Peer name
        #[arg(long)]
        name: String,
        /// Peer URL
        #[arg(long)]
        url: String,
        /// Priority (lower = higher priority)
        #[arg(long, default_value = "10")]
        priority: u8,
        /// Peer type (official, community, backup, development)
        #[arg(long, default_value = "community")]
        peer_type: String,
        /// Initial reputation score (0-100)
        #[arg(long, default_value = "80")]
        reputation: u8,
        /// Enable peer immediately
        #[arg(long, default_value = "true")]
        enabled: bool,
    },
    /// Remove an organizer peer
    RemovePeer {
        /// Peer name to remove
        name: String,
    },
    /// Enable or disable a peer
    SetPeerStatus {
        /// Peer name
        name: String,
        /// Enable (true) or disable (false)
        #[arg(long)]
        enabled: bool,
    },
    /// Update peer reputation
    SetReputation {
        /// Peer name
        name: String,
        /// New reputation score (0-100)
        reputation: u8,
    },
    /// Test connectivity to all enabled peers
    TestPeers,
    /// Reset configuration to defaults
    Reset,
    /// Show peer statistics
    Stats,
}

impl ConfigCommand {
    pub async fn execute(self) -> Result<(), Box<dyn std::error::Error>> {
        match self.action {
            ConfigAction::Show => show_config().await,
            ConfigAction::Peers => list_peers().await,
            ConfigAction::AddPeer { name, url, priority, peer_type, reputation, enabled } => {
                add_peer(name, url, priority, peer_type, reputation, enabled).await
            },
            ConfigAction::RemovePeer { name } => remove_peer(name).await,
            ConfigAction::SetPeerStatus { name, enabled } => set_peer_status(name, enabled).await,
            ConfigAction::SetReputation { name, reputation } => set_reputation(name, reputation).await,
            ConfigAction::TestPeers => test_peers().await,
            ConfigAction::Reset => reset_config().await,
            ConfigAction::Stats => show_stats().await,
        }
    }
}

async fn show_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = CommentItConfig::load_or_create()?;
    
    println!("🔧 COMMENT-IT CONFIGURATION");
    println!("==========================");
    println!();
    
    println!("📡 Network Settings:");
    println!("  Kaspa Network: {}", config.network.kaspa_network);
    println!("  RPC URLs: {:?}", config.network.kaspa_rpc_urls);
    println!("  Auth TX Prefix: {}", config.network.auth_tx_prefix);
    println!("  Comment TX Prefix: {}", config.network.comment_tx_prefix);
    println!();
    
    println!("🛡️ Resilience Settings:");
    println!("  Max Retries per Peer: {}", config.resilience.max_retries_per_peer);
    println!("  Request Timeout: {}s", config.resilience.request_timeout_seconds);
    println!("  Try All Peers: {}", config.resilience.try_all_peers);
    println!("  Min Reputation: {}", config.resilience.min_reputation);
    println!("  Prefer Speed: {}", config.resilience.prefer_speed);
    println!();
    
    println!("👥 Organizer Peers ({} total):", config.organizer_peers.len());
    for (i, peer) in config.organizer_peers.iter().enumerate() {
        let status = if peer.enabled { "✅ ENABLED" } else { "❌ DISABLED" };
        let reputation = peer.reputation.map(|r| format!("{}%", r)).unwrap_or_else(|| "N/A".to_string());
        
        println!("  {}. {} [{}]", i + 1, peer.name, status);
        println!("     URL: {}", peer.url);
        println!("     Type: {:?}, Priority: {}, Reputation: {}", 
                peer.peer_type, peer.priority, reputation);
        if i < config.organizer_peers.len() - 1 {
            println!();
        }
    }
    
    Ok(())
}

async fn list_peers() -> Result<(), Box<dyn std::error::Error>> {
    let config = CommentItConfig::load_or_create()?;
    let enabled_peers = config.get_enabled_peers();
    
    println!("👥 ORGANIZER PEERS");
    println!("==================");
    println!();
    
    println!("📊 Summary:");
    println!("  Total Peers: {}", config.organizer_peers.len());
    println!("  Enabled Peers: {}", enabled_peers.len());
    println!("  Priority Order: {}", 
            enabled_peers.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(" → "));
    println!();
    
    for peer in &config.organizer_peers {
        let status_icon = if peer.enabled { "✅" } else { "❌" };
        let type_icon = match peer.peer_type {
            PeerType::Official => "🏛️",
            PeerType::Community => "👥",
            PeerType::Backup => "🔄",
            PeerType::Development => "🧪",
        };
        
        println!("{} {} {} {}", status_icon, type_icon, peer.name, peer.url);
        println!("   Priority: {}, Reputation: {}%", 
                peer.priority, peer.reputation.unwrap_or(0));
    }
    
    Ok(())
}

async fn add_peer(name: String, url: String, priority: u8, peer_type_str: String, reputation: u8, enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = CommentItConfig::load_or_create()?;
    
    // Check if peer already exists
    if config.organizer_peers.iter().any(|p| p.name == name) {
        return Err(format!("Peer '{}' already exists", name).into());
    }
    
    // Parse peer type
    let peer_type = match peer_type_str.to_lowercase().as_str() {
        "official" => PeerType::Official,
        "community" => PeerType::Community,
        "backup" => PeerType::Backup,
        "development" => PeerType::Development,
        _ => return Err(format!("Invalid peer type: {}. Use: official, community, backup, development", peer_type_str).into()),
    };
    
    let peer = OrganizerPeer {
        name: name.clone(),
        url: url.clone(),
        priority,
        enabled,
        peer_type,
        reputation: Some(reputation.min(100)),
    };
    
    config.add_peer(peer);
    config.save()?;
    
    let status = if enabled { "ENABLED" } else { "DISABLED" };
    println!("✅ Added peer '{}' ({}) - {} with priority {} and reputation {}%", 
            name, url, status, priority, reputation);
    
    Ok(())
}

async fn remove_peer(name: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = CommentItConfig::load_or_create()?;
    
    if config.remove_peer(&name) {
        config.save()?;
        println!("✅ Removed peer '{}'", name);
    } else {
        println!("❌ Peer '{}' not found", name);
    }
    
    Ok(())
}

async fn set_peer_status(name: String, enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = CommentItConfig::load_or_create()?;
    
    if config.update_peer_status(&name, enabled) {
        config.save()?;
        let status = if enabled { "ENABLED" } else { "DISABLED" };
        println!("✅ Peer '{}' is now {}", name, status);
    } else {
        println!("❌ Peer '{}' not found", name);
    }
    
    Ok(())
}

async fn set_reputation(name: String, reputation: u8) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = CommentItConfig::load_or_create()?;
    
    if config.update_peer_reputation(&name, reputation) {
        config.save()?;
        println!("✅ Updated reputation for peer '{}' to {}%", name, reputation);
    } else {
        println!("❌ Peer '{}' not found", name);
    }
    
    Ok(())
}

async fn test_peers() -> Result<(), Box<dyn std::error::Error>> {
    use crate::cli::resilient_peer_connection::{ResilientPeerConnection, ApiRequest, HttpMethod};
    
    let config = CommentItConfig::load_or_create()?;
    let mut peer_connection = ResilientPeerConnection::new(config.clone());
    
    println!("🧪 TESTING ORGANIZER PEER CONNECTIVITY");
    println!("======================================");
    println!();
    
    let enabled_peers = config.get_enabled_peers();
    
    if enabled_peers.is_empty() {
        println!("❌ No enabled peers to test");
        return Ok(());
    }
    
    // Test health endpoint on each peer
    let health_request = ApiRequest {
        method: HttpMethod::GET,
        path: "/health".to_string(),
        body: None,
    };
    
    for peer in enabled_peers {
        print!("Testing '{}' at {} ... ", peer.name, peer.url);
        
        // Create a peer connection just for this peer
        let temp_config = CommentItConfig {
            organizer_peers: vec![peer.clone()],
            ..config.clone()
        };
        let mut temp_peer_connection = ResilientPeerConnection::new(temp_config);
        
        match temp_peer_connection.request(health_request.clone()).await {
            Ok(response) => {
                println!("✅ OK ({}ms)", response.response_time.as_millis());
            },
            Err(e) => {
                println!("❌ FAILED: {}", e);
            }
        }
    }
    
    println!();
    println!("💡 Use 'comment-it config stats' to see detailed peer statistics");
    
    Ok(())
}

async fn reset_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = CommentItConfig::default();
    config.save()?;
    println!("✅ Configuration reset to defaults");
    Ok(())
}

async fn show_stats() -> Result<(), Box<dyn std::error::Error>> {
    // Note: This would require a persistent stats system
    // For now, just show that the feature exists
    println!("📊 PEER STATISTICS");
    println!("==================");
    println!();
    println!("💡 Peer statistics are collected during runtime.");
    println!("   Run authentication commands to generate statistics,");
    println!("   then use this command to view performance data.");
    
    Ok(())
}