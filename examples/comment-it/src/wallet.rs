// src/wallet.rs - Unified Wallet Management System (from kaspa-auth)
use secp256k1::Keypair;
use std::path::{Path, PathBuf};
use std::fs;
use kaspa_addresses::{Address, Prefix, Version};
use kaspa_consensus_core::network::{NetworkId, NetworkType};
use log::{info, warn};

#[derive(Debug, Clone)]
pub struct WalletConfig {
    pub wallet_dir: PathBuf,
    pub keypair_file: PathBuf,
    pub network_id: NetworkId,
}

impl Default for WalletConfig {
    fn default() -> Self {
        let wallet_dir = Path::new(".kaspa-auth").to_path_buf();
        let keypair_file = wallet_dir.join("wallet.key");
        let network_id = NetworkId::with_suffix(NetworkType::Testnet, 10);
        
        Self {
            wallet_dir,
            keypair_file,
            network_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KaspaAuthWallet {
    pub keypair: Keypair,
    pub config: WalletConfig,
    pub was_created: bool, // True if wallet was created this session
}

impl KaspaAuthWallet {
    /// Load existing wallet or create new one with smooth UX
    pub fn load_or_create() -> Result<Self, Box<dyn std::error::Error>> {
        let config = WalletConfig::default();
        Self::load_or_create_with_config(config)
    }
    
    /// Load wallet for specific role (server/client) with separate wallet files
    pub fn load_or_create_with_role(role: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = WalletConfig::default();
        
        // Use separate wallet files for server vs client
        config.keypair_file = config.wallet_dir.join(format!("{}-wallet.key", role));
        
        println!("📁 Loading {} wallet from: {}", role, config.keypair_file.display());
        Self::load_or_create_with_config(config)
    }
    
    /// Load existing wallet or create new one with custom config
    pub fn load_or_create_with_config(config: WalletConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Check if this is first run
        let is_first_run = !config.keypair_file.exists();
        
        if is_first_run {
            Self::create_new_wallet_ux(config)
        } else {
            Self::load_existing_wallet_ux(config)
        }
    }
    
    /// Create new wallet with welcoming UX
    fn create_new_wallet_ux(config: WalletConfig) -> Result<Self, Box<dyn std::error::Error>> {
        println!("🎉 Welcome to Kaspa Authentication!");
        println!("📁 Setting up your wallet directory: {}", config.wallet_dir.display());
        
        // Create wallet directory
        fs::create_dir_all(&config.wallet_dir)?;
        
        println!("🔑 Generating secure keypair...");
        
        // Generate new keypair
        use secp256k1::{Secp256k1, SecretKey};
        use rand::rngs::OsRng;
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut OsRng);
        let keypair = Keypair::from_secret_key(&secp, &secret_key);
        
        // Save the secret key
        fs::write(&config.keypair_file, secret_key.as_ref())?;
        
        // Generate Kaspa address
        let network_prefix = Prefix::from(config.network_id);
        let kaspa_address = Address::new(network_prefix, Version::PubKey, &keypair.public_key().serialize()[1..]);
        
        println!("💾 Wallet saved to: {}", config.keypair_file.display());
        println!("🔑 Public Key: {}", hex::encode(keypair.public_key().serialize()));
        println!("💰 Funding Address: {}", kaspa_address);
        println!("🌐 Network: {}", config.network_id);
        println!("💡 Fund this address at: https://faucet.kaspanet.io/");
        println!("✅ Wallet setup complete!");
        println!();
        
        Ok(Self {
            keypair,
            config,
            was_created: true,
        })
    }
    
    /// Load existing wallet with status UX
    fn load_existing_wallet_ux(config: WalletConfig) -> Result<Self, Box<dyn std::error::Error>> {
        println!("📁 Loading wallet from: {}", config.keypair_file.display());
        
        // Load existing keypair
        let key_data = fs::read(&config.keypair_file)?;
        if key_data.len() != 32 {
            return Err("Invalid wallet file format".into());
        }
        
        use secp256k1::{Secp256k1, SecretKey};
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&key_data)?;
        let keypair = Keypair::from_secret_key(&secp, &secret_key);
        
        // Generate Kaspa address for display
        let network_prefix = Prefix::from(config.network_id);
        let kaspa_address = Address::new(network_prefix, Version::PubKey, &keypair.public_key().serialize()[1..]);
        
        println!("✅ Wallet loaded successfully");
        println!("🔑 Public Key: {}", hex::encode(keypair.public_key().serialize()));
        println!("💰 Funding Address: {}", kaspa_address);
        println!("🌐 Network: {}", config.network_id);
        println!();
        
        Ok(Self {
            keypair,
            config,
            was_created: false,
        })
    }
    
    /// Get the Kaspa address for this wallet
    pub fn get_kaspa_address(&self) -> Address {
        let network_prefix = Prefix::from(self.config.network_id);
        Address::new(network_prefix, Version::PubKey, &self.keypair.public_key().serialize()[1..])
    }
    
    /// Get public key as hex string
    pub fn get_public_key_hex(&self) -> String {
        hex::encode(self.keypair.public_key().serialize())
    }
    
    /// Check if wallet needs funding - currently returns true for new wallets
    /// Future enhancement: integrate with Kaspa RPC to check actual balance
    pub fn check_funding_status(&self) -> bool {
        // Currently suggests funding for newly created wallets
        // Real implementation would query UTXO set via Kaspa RPC
        self.was_created
    }
    
    /// Display funding reminder
    pub fn show_funding_reminder(&self) {
        if self.check_funding_status() {
            println!("💡 REMINDER: Fund your address to test economic features:");
            println!("   Address: {}", self.get_kaspa_address());
            println!("   Faucet: https://faucet.kaspanet.io/");
            println!();
        }
    }
    
    /// Load wallet for specific command with appropriate messaging
    pub fn load_for_command(command: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Use separate wallet files for organizer vs participant peers
        let wallet = match command {
            "organizer-peer" | "http-peer" => Self::load_or_create_with_role("organizer-peer")?,
            "participant-peer" | "web-participant" | "authenticate" => Self::load_or_create_with_role("participant-peer")?,
            // Legacy compatibility
            "server" | "http-server" => Self::load_or_create_with_role("organizer-peer")?,
            "client" => Self::load_or_create_with_role("participant-peer")?,
            _ => Self::load_or_create()?,
        };
        
        match command {
            "organizer-peer" | "http-peer" | "server" | "http-server" => {
                let kaspa_addr = wallet.get_kaspa_address();
                if wallet.was_created {
                    println!("🆕 Creating NEW organizer-peer wallet");
                    println!("🔑 New Kaspa address: {}", kaspa_addr);
                    println!("💾 Wallet saved to: .kaspa-auth/organizer-peer-wallet.key");
                } else {
                    println!("🔄 REUSING existing organizer-peer wallet");
                    println!("🔑 Existing Kaspa address: {}", kaspa_addr);
                    println!("📁 Loaded from: .kaspa-auth/organizer-peer-wallet.key");
                }
                wallet.show_funding_reminder();
            },
            "participant-peer" | "web-participant" | "authenticate" | "client" => {
                let kaspa_addr = wallet.get_kaspa_address();
                if wallet.was_created {
                    println!("🆕 Creating NEW participant-peer wallet");
                    println!("🔑 New Kaspa address: {}", kaspa_addr);
                    println!("💾 Wallet saved to: .kaspa-auth/participant-peer-wallet.key");
                } else {
                    println!("🔄 REUSING existing participant-peer wallet");
                    println!("🔑 Existing Kaspa address: {}", kaspa_addr);
                    println!("📁 Loaded from: .kaspa-auth/participant-peer-wallet.key");
                }
            },
            _ => {
                println!("🔑 Using {} wallet ({})", command, if wallet.was_created { "NEW" } else { "EXISTING" });
            }
        }
        
        Ok(wallet)
    }
    
    /// Create wallet from provided private key (for --key option)
    pub fn from_private_key(private_key_hex: &str) -> Result<Self, Box<dyn std::error::Error>> {
        use secp256k1::{Secp256k1, SecretKey};
        
        let secp = Secp256k1::new();
        let secret_bytes = hex::decode(private_key_hex)?;
        let secret_key = SecretKey::from_slice(&secret_bytes)?;
        let keypair = Keypair::from_secret_key(&secp, &secret_key);
        
        println!("🔑 Using provided private key");
        println!("🔑 Public Key: {}", hex::encode(keypair.public_key().serialize()));
        
        Ok(Self {
            keypair,
            config: WalletConfig::default(),
            was_created: false,
        })
    }
    
    /// Create wallet from private key and save to specific file
    pub fn from_private_key_and_save(private_key_hex: &str, wallet_file: &str) -> Result<Self, Box<dyn std::error::Error>> {
        use secp256k1::{Secp256k1, SecretKey};
        
        println!("🔍 DEBUG: Importing private key: {}...", &private_key_hex[0..8]);
        
        let secp = Secp256k1::new();
        let secret_bytes = hex::decode(private_key_hex)?;
        println!("🔍 DEBUG: Decoded {} bytes from hex", secret_bytes.len());
        
        let secret_key = SecretKey::from_slice(&secret_bytes)?;
        let keypair = Keypair::from_secret_key(&secp, &secret_key);
        
        let public_key_bytes = keypair.public_key().serialize();
        println!("🔍 DEBUG: Full public key (33 bytes): {}", hex::encode(&public_key_bytes));
        println!("🔍 DEBUG: Public key without prefix (32 bytes): {}", hex::encode(&public_key_bytes[1..]));
        
        // Create custom config with the specific file path
        let mut config = WalletConfig::default();
        config.keypair_file = config.wallet_dir.join(wallet_file);
        
        println!("🔍 DEBUG: Network ID: {}", config.network_id);
        let network_prefix = Prefix::from(config.network_id);
        println!("🔍 DEBUG: Network prefix: {:?}", network_prefix);
        
        // Create wallet directory if it doesn't exist
        fs::create_dir_all(&config.wallet_dir)?;
        
        // Save the private key to the file
        fs::write(&config.keypair_file, secret_key.as_ref())?;
        
        // Generate Kaspa address for display
        let kaspa_address = Address::new(network_prefix, Version::PubKey, &keypair.public_key().serialize()[1..]);
        
        println!("💾 Wallet saved to: {}", config.keypair_file.display());
        println!("🔑 Public Key: {}", hex::encode(keypair.public_key().serialize()));
        println!("💰 Kaspa Address: {}", kaspa_address);
        
        Ok(Self {
            keypair,
            config,
            was_created: false, // Not created this session, imported
        })
    }
}

/// Get wallet for any command with unified UX
pub fn get_wallet_for_command(command: &str, private_key: Option<&str>) -> Result<KaspaAuthWallet, Box<dyn std::error::Error>> {
    match private_key {
        Some(key_hex) => {
            println!("🔑 Using provided private key for {}", command);
            KaspaAuthWallet::from_private_key(key_hex)
        },
        None => {
            KaspaAuthWallet::load_for_command(command)
        }
    }
}

/// Check if wallet exists for command WITHOUT creating it
pub fn wallet_exists_for_command(command: &str) -> bool {
    let config = WalletConfig::default();
    let wallet_file = match command {
        "organizer-peer" | "http-peer" => config.wallet_dir.join("organizer-peer-wallet.key"),
        "participant-peer" | "web-participant" | "authenticate" => config.wallet_dir.join("participant-peer-wallet.key"),
        "server" | "http-server" => config.wallet_dir.join("organizer-peer-wallet.key"),
        "client" => config.wallet_dir.join("participant-peer-wallet.key"),
        _ => config.keypair_file,
    };
    
    wallet_file.exists()
}