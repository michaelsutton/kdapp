// src/cli/config.rs
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentItConfig {
    /// Multiple organizer peers for resilience
    pub organizer_peers: Vec<OrganizerPeer>,
    
    /// Blockchain network settings
    pub network: NetworkConfig,
    
    /// Fallback and retry settings
    pub resilience: ResilienceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizerPeer {
    /// Peer identifier (e.g., "primary", "backup-1", "community-node")
    pub name: String,
    
    /// HTTP endpoint URL
    pub url: String,
    
    /// Priority level (lower = higher priority)
    pub priority: u8,
    
    /// Whether this peer is currently enabled
    pub enabled: bool,
    
    /// Peer type for different use cases
    pub peer_type: PeerType,
    
    /// Optional reputation score (0-100)
    pub reputation: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PeerType {
    /// Official project organizer
    Official,
    /// Community-run organizer
    Community,
    /// Backup/fallback organizer
    Backup,
    /// Development/testing organizer
    Development,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Kaspa network (testnet-10, mainnet)
    pub kaspa_network: String,
    
    /// Kaspa node RPC URLs (multiple for redundancy)
    pub kaspa_rpc_urls: Vec<String>,
    
    /// Transaction prefix for auth episodes
    pub auth_tx_prefix: String,
    
    /// Transaction prefix for comment episodes
    pub comment_tx_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResilienceConfig {
    /// Maximum retries per peer before trying next
    pub max_retries_per_peer: u32,
    
    /// Timeout per request in seconds
    pub request_timeout_seconds: u64,
    
    /// Whether to try all peers before giving up
    pub try_all_peers: bool,
    
    /// Minimum reputation score required
    pub min_reputation: u8,
    
    /// Whether to prefer faster peers over higher priority
    pub prefer_speed: bool,
}

impl Default for CommentItConfig {
    fn default() -> Self {
        Self {
            organizer_peers: vec![
                OrganizerPeer {
                    name: "local-development".to_string(),
                    url: "http://127.0.0.1:8080".to_string(),
                    priority: 1,
                    enabled: true,
                    peer_type: PeerType::Development,
                    reputation: Some(90),
                },
                OrganizerPeer {
                    name: "project-official".to_string(),
                    url: "https://comments1.kaspa.community".to_string(),
                    priority: 2,
                    enabled: false, // Disabled by default since it doesn't exist yet
                    peer_type: PeerType::Official,
                    reputation: Some(95),
                },
                OrganizerPeer {
                    name: "community-backup".to_string(),
                    url: "https://comments2.kaspa.community".to_string(),
                    priority: 3,
                    enabled: false, // Disabled by default since it doesn't exist yet
                    peer_type: PeerType::Community,
                    reputation: Some(85),
                },
            ],
            network: NetworkConfig {
                kaspa_network: "testnet-10".to_string(),
                kaspa_rpc_urls: vec![
                    "grpc://127.0.0.1:16110".to_string(),
                    "grpc://testnet-10.kaspanet.io:16110".to_string(),
                ],
                auth_tx_prefix: "0x41555448".to_string(), // "AUTH"
                comment_tx_prefix: "0x434F4D4D".to_string(), // "COMM"
            },
            resilience: ResilienceConfig {
                max_retries_per_peer: 3,
                request_timeout_seconds: 30,
                try_all_peers: true,
                min_reputation: 70,
                prefer_speed: false,
            },
        }
    }
}

impl CommentItConfig {
    /// Load configuration from file, create default if doesn't exist
    pub fn load_or_create() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::config_file_path();
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: CommentItConfig = serde_json::from_str(&content)?;
            println!("📋 Loaded configuration from: {}", config_path.display());
            Ok(config)
        } else {
            let config = Self::default();
            config.save()?;
            println!("=� Created default configuration at: {}", config_path.display());
            Ok(config)
        }
    }
    
    /// Save configuration to file
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_file_path();
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        println!("=� Configuration saved to: {}", config_path.display());
        Ok(())
    }
    
    /// Get enabled organizer peers sorted by priority
    pub fn get_enabled_peers(&self) -> Vec<&OrganizerPeer> {
        let mut peers: Vec<&OrganizerPeer> = self.organizer_peers
            .iter()
            .filter(|p| p.enabled)
            .filter(|p| p.reputation.unwrap_or(0) >= self.resilience.min_reputation)
            .collect();
        
        peers.sort_by_key(|p| p.priority);
        peers
    }
    
    /// Get configuration file path
    fn config_file_path() -> std::path::PathBuf {
        let mut path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        path.push(".comment-it");
        path.push("config.json");
        path
    }
    
    /// Add a new organizer peer
    pub fn add_peer(&mut self, peer: OrganizerPeer) {
        self.organizer_peers.push(peer);
    }
    
    /// Remove a peer by name
    pub fn remove_peer(&mut self, name: &str) -> bool {
        let initial_len = self.organizer_peers.len();
        self.organizer_peers.retain(|p| p.name != name);
        self.organizer_peers.len() < initial_len
    }
    
    /// Update peer status
    pub fn update_peer_status(&mut self, name: &str, enabled: bool) -> bool {
        if let Some(peer) = self.organizer_peers.iter_mut().find(|p| p.name == name) {
            peer.enabled = enabled;
            true
        } else {
            false
        }
    }
    
    /// Update peer reputation
    pub fn update_peer_reputation(&mut self, name: &str, reputation: u8) -> bool {
        if let Some(peer) = self.organizer_peers.iter_mut().find(|p| p.name == name) {
            peer.reputation = Some(reputation.min(100));
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = CommentItConfig::default();
        assert!(!config.organizer_peers.is_empty());
        assert!(config.get_enabled_peers().len() >= 1);
    }
    
    #[test]
    fn test_peer_filtering() {
        let mut config = CommentItConfig::default();
        
        // Disable all peers
        for peer in &mut config.organizer_peers {
            peer.enabled = false;
        }
        
        assert_eq!(config.get_enabled_peers().len(), 0);
        
        // Enable one peer
        config.organizer_peers[0].enabled = true;
        assert_eq!(config.get_enabled_peers().len(), 1);
    }
}