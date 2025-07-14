// Cargo.toml additions
/*
[dependencies]
ipfs-api = "0.17"
ipfs-api-backend-hyper = "0.6"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
sha2 = "0.10"
hex = "0.4"
futures = "0.3"
*/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ipfs_api::{IpfsApi, IpfsClient};
use ipfs_api_backend_hyper::TryFromUri;
use kaspa_auth::{AuthSystem, WalletAuth};
use kdapp::{Episode, EpisodeError, PayloadMetadata, PubKey};
use borsh::{BorshSerialize, BorshDeserialize};

// =============================================================================
// IPFS Integration for Comment-It
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsConfig {
    pub gateway_urls: Vec<String>,
    pub pinning_services: Vec<PinningService>,
    pub local_node_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinningService {
    pub name: String,
    pub api_url: String,
    pub api_key: String,
}

impl Default for IpfsConfig {
    fn default() -> Self {
        Self {
            gateway_urls: vec![
                "https://ipfs.io/ipfs/".to_string(),
                "https://gateway.pinata.cloud/ipfs/".to_string(),
                "https://cloudflare-ipfs.com/ipfs/".to_string(),
            ],
            pinning_services: vec![
                PinningService {
                    name: "Pinata".to_string(),
                    api_url: "https://api.pinata.cloud/pinning/pinFileToIPFS".to_string(),
                    api_key: std::env::var("PINATA_API_KEY").unwrap_or_default(),
                },
            ],
            local_node_url: Some("http://127.0.0.1:5001".to_string()),
        }
    }
}

// =============================================================================
// Enhanced Episode for Service Discovery with IPFS
// =============================================================================

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ServiceRegistry {
    pub services: HashMap<String, Vec<ServiceInfo>>,
    pub ipfs_content: HashMap<String, IpfsContent>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ServiceInfo {
    pub wallet_address: String,
    pub service_type: String,
    pub endpoints: Vec<ServiceEndpoint>,
    pub ipfs_frontend: Option<String>, // IPFS hash of frontend
    pub last_seen: u64,
    pub reputation: u32,
    pub signature: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct IpfsContent {
    pub hash: String,
    pub content_type: String,
    pub size: u64,
    pub pinned_by: Vec<String>, // Wallet addresses that pin this content
    pub upload_time: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum ServiceEndpoint {
    Http(String),
    Https(String),
    Tor(String),
    I2P(String),
    IpfsGateway(String),
    KaspaRelay(String),
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum ServiceCommand {
    RegisterService {
        service_type: String,
        endpoints: Vec<ServiceEndpoint>,
        ipfs_frontend: Option<String>,
    },
    UpdateEndpoints {
        endpoints: Vec<ServiceEndpoint>,
    },
    PinContent {
        ipfs_hash: String,
        content_type: String,
    },
    UpdateReputation {
        target_wallet: String,
        change: i32,
    },
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum ServiceCommandRollback {
    RegisterService { wallet: String },
    UpdateEndpoints { wallet: String, old_endpoints: Vec<ServiceEndpoint> },
    PinContent { ipfs_hash: String, wallet: String },
    UpdateReputation { target_wallet: String, old_reputation: u32 },
}

#[derive(Debug)]
pub enum ServiceError {
    Unauthorized,
    InvalidSignature,
    ServiceNotFound,
    InvalidIpfsHash,
}

impl Episode for ServiceRegistry {
    type Command = ServiceCommand;
    type CommandRollback = ServiceCommandRollback;
    type CommandError = ServiceError;

    fn execute(
        &mut self,
        cmd: &Self::Command,
        auth: Option<PubKey>,
        metadata: &PayloadMetadata,
    ) -> Result<Self::CommandRollback, EpisodeError<Self::CommandError>> {
        let wallet_pubkey = auth.ok_or(EpisodeError::Unauthorized)?;
        let wallet_address = format!("kaspa:{}", hex::encode(&wallet_pubkey.to_bytes()));

        match cmd {
            ServiceCommand::RegisterService { service_type, endpoints, ipfs_frontend } => {
                let service_info = ServiceInfo {
                    wallet_address: wallet_address.clone(),
                    service_type: service_type.clone(),
                    endpoints: endpoints.clone(),
                    ipfs_frontend: ipfs_frontend.clone(),
                    last_seen: metadata.accepting_time,
                    reputation: 100, // Starting reputation
                    signature: vec![], // TODO: Implement proper signature
                };

                self.services
                    .entry(service_type.clone())
                    .or_default()
                    .push(service_info);

                Ok(ServiceCommandRollback::RegisterService { wallet: wallet_address })
            }

            ServiceCommand::UpdateEndpoints { endpoints } => {
                let service_list = self.services.values_mut().flatten();
                if let Some(service) = service_list.find(|s| s.wallet_address == wallet_address) {
                    let old_endpoints = service.endpoints.clone();
                    service.endpoints = endpoints.clone();
                    service.last_seen = metadata.accepting_time;
                    
                    Ok(ServiceCommandRollback::UpdateEndpoints {
                        wallet: wallet_address,
                        old_endpoints,
                    })
                } else {
                    Err(EpisodeError::Command(ServiceError::ServiceNotFound))
                }
            }

            ServiceCommand::PinContent { ipfs_hash, content_type } => {
                // Validate IPFS hash format
                if !is_valid_ipfs_hash(ipfs_hash) {
                    return Err(EpisodeError::Command(ServiceError::InvalidIpfsHash));
                }

                let content = self.ipfs_content.entry(ipfs_hash.clone()).or_insert_with(|| {
                    IpfsContent {
                        hash: ipfs_hash.clone(),
                        content_type: content_type.clone(),
                        size: 0, // TODO: Fetch size from IPFS
                        pinned_by: vec![],
                        upload_time: metadata.accepting_time,
                    }
                });

                if !content.pinned_by.contains(&wallet_address) {
                    content.pinned_by.push(wallet_address.clone());
                }

                Ok(ServiceCommandRollback::PinContent {
                    ipfs_hash: ipfs_hash.clone(),
                    wallet: wallet_address,
                })
            }

            ServiceCommand::UpdateReputation { target_wallet, change } => {
                let service_list = self.services.values_mut().flatten();
                if let Some(service) = service_list.find(|s| s.wallet_address == *target_wallet) {
                    let old_reputation = service.reputation;
                    service.reputation = (service.reputation as i32 + change).max(0) as u32;
                    
                    Ok(ServiceCommandRollback::UpdateReputation {
                        target_wallet: target_wallet.clone(),
                        old_reputation,
                    })
                } else {
                    Err(EpisodeError::Command(ServiceError::ServiceNotFound))
                }
            }
        }
    }

    fn rollback(
        &mut self,
        rollback: &Self::CommandRollback,
        _: &PayloadMetadata,
    ) -> Result<(), EpisodeError<Self::CommandError>> {
        match rollback {
            ServiceCommandRollback::RegisterService { wallet } => {
                // Remove the service that was just registered
                for service_list in self.services.values_mut() {
                    service_list.retain(|s| s.wallet_address != *wallet);
                }
                Ok(())
            }

            ServiceCommandRollback::UpdateEndpoints { wallet, old_endpoints } => {
                let service_list = self.services.values_mut().flatten();
                if let Some(service) = service_list.find(|s| s.wallet_address == *wallet) {
                    service.endpoints = old_endpoints.clone();
                }
                Ok(())
            }

            ServiceCommandRollback::PinContent { ipfs_hash, wallet } => {
                if let Some(content) = self.ipfs_content.get_mut(ipfs_hash) {
                    content.pinned_by.retain(|w| w != wallet);
                    if content.pinned_by.is_empty() {
                        self.ipfs_content.remove(ipfs_hash);
                    }
                }
                Ok(())
            }

            ServiceCommandRollback::UpdateReputation { target_wallet, old_reputation } => {
                let service_list = self.services.values_mut().flatten();
                if let Some(service) = service_list.find(|s| s.wallet_address == *target_wallet) {
                    service.reputation = *old_reputation;
                }
                Ok(())
            }
        }
    }
}

// =============================================================================
// IPFS Client for Comment-It
// =============================================================================

pub struct CommentItIpfs {
    client: IpfsClient,
    config: IpfsConfig,
    auth_system: AuthSystem,
}

impl CommentItIpfs {
    pub async fn new(config: IpfsConfig, auth_system: AuthSystem) -> Result<Self, Box<dyn std::error::Error>> {
        let client = if let Some(ref local_url) = config.local_node_url {
            IpfsClient::from_str(local_url)?
        } else {
            // Use public gateway as fallback
            IpfsClient::from_str("https://ipfs.io")?
        };

        Ok(Self {
            client,
            config,
            auth_system,
        })
    }

    // Upload frontend to IPFS
    pub async fn upload_frontend(&self, frontend_files: &[(&str, Vec<u8>)]) -> Result<String, Box<dyn std::error::Error>> {
        // Create a directory structure for the frontend
        let mut files = Vec::new();
        
        for (filename, content) in frontend_files {
            let cursor = std::io::Cursor::new(content);
            files.push((filename.to_string(), cursor));
        }

        // Add directory to IPFS
        let response = self.client.add_path(&files).await?;
        
        // Get the root hash (directory hash)
        let root_hash = response.hash;
        
        // Pin the content
        self.client.pin_add(&root_hash, false).await?;
        
        // Also pin to external services
        self.pin_to_services(&root_hash).await?;
        
        Ok(root_hash)
    }

    // Pin content to external pinning services
    async fn pin_to_services(&self, ipfs_hash: &str) -> Result<(), Box<dyn std::error::Error>> {
        for service in &self.config.pinning_services {
            if service.api_key.is_empty() {
                continue;
            }

            let client = reqwest::Client::new();
            let response = client
                .post(&service.api_url)
                .header("Authorization", format!("Bearer {}", service.api_key))
                .json(&serde_json::json!({
                    "hashToPin": ipfs_hash,
                    "pinataMetadata": {
                        "name": "comment-it-frontend"
                    }
                }))
                .send()
                .await?;

            if response.status().is_success() {
                println!("Successfully pinned to {}", service.name);
            } else {
                eprintln!("Failed to pin to {}: {}", service.name, response.status());
            }
        }
        Ok(())
    }

    // Resolve content from IPFS
    pub async fn get_content(&self, ipfs_hash: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        match self.client.cat(ipfs_hash).await {
            Ok(content) => Ok(content),
            Err(_) => {
                // Fallback to HTTP gateways
                self.get_content_via_gateway(ipfs_hash).await
            }
        }
    }

    async fn get_content_via_gateway(&self, ipfs_hash: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        
        for gateway in &self.config.gateway_urls {
            let url = format!("{}{}", gateway, ipfs_hash);
            
            match client.get(&url).send().await {
                Ok(response) if response.status().is_success() => {
                    return Ok(response.bytes().await?.to_vec());
                }
                _ => continue,
            }
        }
        
        Err("All gateways failed".into())
    }

    // Get available comment-it services
    pub async fn discover_services(&self, service_registry: &ServiceRegistry) -> Vec<ServiceInfo> {
        service_registry
            .services
            .get("comment-it")
            .cloned()
            .unwrap_or_default()
    }

    // Create a resilient frontend loader
    pub fn generate_bootstrap_html(&self, ipfs_hash: &str) -> String {
        let gateways = self.config.gateway_urls.join("\",\"");
        
        format!(r#"<!DOCTYPE html>
<html>
<head>
    <title>Comment-It - Decentralized Comments</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; }}
        .loading {{ text-align: center; }}
        .error {{ color: red; }}
    </style>
</head>
<body>
    <div class="loading">
        <h1>Loading Comment-It...</h1>
        <p>Fetching from IPFS: {}</p>
        <div id="status">Trying IPFS gateways...</div>
    </div>
    
    <script>
        const IPFS_HASH = '{}';
        const GATEWAYS = ["{}"];
        
        async function loadFromIPFS() {{
            const status = document.getElementById('status');
            
            for (let i = 0; i < GATEWAYS.length; i++) {{
                const gateway = GATEWAYS[i];
                const url = gateway + IPFS_HASH + '/index.html';
                
                try {{
                    status.textContent = `Trying gateway ${{i + 1}}/${{GATEWAYS.length}}: ${{gateway}}`;
                    
                    const response = await fetch(url);
                    if (response.ok) {{
                        const html = await response.text();
                        document.documentElement.innerHTML = html;
                        return;
                    }}
                }} catch (e) {{
                    console.warn('Gateway failed:', gateway, e);
                }}
            }}
            
            status.innerHTML = '<div class="error">Failed to load from all IPFS gateways. Please try again later.</div>';
        }}
        
        // Start loading
        loadFromIPFS();
    </script>
</body>
</html>"#, ipfs_hash, ipfs_hash, gateways)
    }
}

// =============================================================================
// Enhanced Comment-It with IPFS Integration
// =============================================================================

pub struct DecentralizedCommentIt {
    pub ipfs: CommentItIpfs,
    pub service_registry: ServiceRegistry,
    pub auth_system: AuthSystem,
}

impl DecentralizedCommentIt {
    pub async fn new(ipfs_config: IpfsConfig, auth_system: AuthSystem) -> Result<Self, Box<dyn std::error::Error>> {
        let ipfs = CommentItIpfs::new(ipfs_config, auth_system.clone()).await?;
        let service_registry = ServiceRegistry {
            services: HashMap::new(),
            ipfs_content: HashMap::new(),
        };

        Ok(Self {
            ipfs,
            service_registry,
            auth_system,
        })
    }

    // Deploy a new version of the frontend
    pub async fn deploy_frontend(&mut self, frontend_files: &[(&str, Vec<u8>)]) -> Result<String, Box<dyn std::error::Error>> {
        // Upload to IPFS
        let ipfs_hash = self.ipfs.upload_frontend(frontend_files).await?;
        
        // Register in service registry
        let wallet_auth = self.auth_system.get_current_wallet()?;
        let register_cmd = ServiceCommand::RegisterService {
            service_type: "comment-it".to_string(),
            endpoints: vec![
                ServiceEndpoint::IpfsGateway(format!("ipfs://{}", ipfs_hash)),
                ServiceEndpoint::Https("https://your-backup-domain.com".to_string()),
            ],
            ipfs_frontend: Some(ipfs_hash.clone()),
        };

        // This would be executed through kdapp framework
        // self.service_registry.execute(&register_cmd, Some(wallet_auth.pubkey), &metadata)?;
        
        println!("Frontend deployed to IPFS: {}", ipfs_hash);
        println!("Access via: https://ipfs.io/ipfs/{}", ipfs_hash);
        
        Ok(ipfs_hash)
    }

    // Generate a resilient access page
    pub fn create_access_page(&self, ipfs_hash: &str) -> String {
        self.ipfs.generate_bootstrap_html(ipfs_hash)
    }
}

// =============================================================================
// Utility Functions
// =============================================================================

fn is_valid_ipfs_hash(hash: &str) -> bool {
    // Basic IPFS hash validation
    hash.starts_with("Qm") && hash.len() == 46 ||
    hash.starts_with("baf") && hash.len() >= 50 // CIDv1
}

// =============================================================================
// Example Usage
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the system
    let ipfs_config = IpfsConfig::default();
    let auth_system = AuthSystem::new()?;
    
    let mut comment_it = DecentralizedCommentIt::new(ipfs_config, auth_system).await?;
    
    // Example frontend files
    let frontend_files = vec![
        ("index.html", include_bytes!("../frontend/index.html").to_vec()),
        ("style.css", include_bytes!("../frontend/style.css").to_vec()),
        ("script.js", include_bytes!("../frontend/script.js").to_vec()),
    ];
    
    // Deploy to IPFS
    let ipfs_hash = comment_it.deploy_frontend(&frontend_files).await?;
    
    // Create access page
    let access_page = comment_it.create_access_page(&ipfs_hash);
    std::fs::write("access.html", access_page)?;
    
    println!("✅ Comment-It deployed successfully!");
    println!("📦 IPFS Hash: {}", ipfs_hash);
    println!("🌐 Access: https://ipfs.io/ipfs/{}", ipfs_hash);
    println!("🚀 Bootstrap: access.html");
    
    Ok(())
}
