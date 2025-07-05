use clap::{Arg, Command};
use env_logger;
use std::error::Error;
use secp256k1::{Secp256k1, SecretKey, Keypair};
use log::info;
use kaspa_addresses;

use kaspa_auth::core::episode::SimpleAuth;
use kaspa_auth::core::commands::AuthCommand;
use kaspa_auth::{AuthServerConfig, run_auth_server};
use kaspa_auth::api::http::server::run_http_server;
use kdapp::pki::{generate_keypair, sign_message, to_message};
use kdapp::episode::{PayloadMetadata, Episode};
// use crate::cli::Cli; // Using inline clap structure instead
// use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logger with clean output - hide debug spam from kdapp internals
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("kaspa_auth=info,kdapp::generator=error,kdapp=warn")).init();

    let matches = Command::new("kaspa-auth")
        .version("0.1.0")
        .about("Kaspa Authentication Episode Demo")
        .subcommand(
            Command::new("test-episode")
                .about("Test auth episode locally (no Kaspa)")
                .arg(
                    Arg::new("participants")
                        .short('p')
                        .long("participants")
                        .value_name("COUNT")
                        .help("Number of participants")
                        .default_value("1")
                )
        )
        .subcommand(
            Command::new("http-server")
                .about("Run HTTP coordination server for authentication")
                .arg(
                    Arg::new("port")
                        .short('p')
                        .long("port")
                        .value_name("PORT")
                        .help("HTTP server port")
                        .default_value("8080")
                )
                .arg(
                    Arg::new("key")
                        .short('k')
                        .long("key")
                        .value_name("PRIVATE_KEY")
                        .help("Private key (hex format) - generates random if not provided")
                )
        )
        .subcommand(
            Command::new("authenticate")
                .about("🚀 One-command authentication with HTTP server (EASY MODE)")
                .arg(
                    Arg::new("server")
                        .short('s')
                        .long("server")
                        .value_name("URL")
                        .help("HTTP server URL")
                        .default_value("http://127.0.0.1:8080")
                )
                .arg(
                    Arg::new("key")
                        .short('k')
                        .long("key")
                        .value_name("PRIVATE_KEY")
                        .help("Private key (hex format) - generates random if not provided")
                )
                .arg(
                    Arg::new("keyfile")
                        .short('f')
                        .long("keyfile")
                        .value_name("FILE")
                        .help("Load private key from file (safer than --key)")
                )
        )
        .subcommand(
            Command::new("demo")
                .about("Run interactive demo")
        )
        .subcommand(
            Command::new("server")
                .about("Run auth server on Kaspa testnet-10")
                .arg(
                    Arg::new("name")
                        .short('n')
                        .long("name")
                        .value_name("NAME")
                        .help("Server name")
                        .default_value("auth-server")
                )
                .arg(
                    Arg::new("key")
                        .short('k')
                        .long("key")
                        .value_name("PRIVATE_KEY")
                        .help("Private key (hex format) - generates random if not provided")
                )
                .arg(
                    Arg::new("rpc-url")
                        .long("rpc-url")
                        .value_name("URL")
                        .help("Kaspa node RPC URL (e.g., grpc://127.0.0.1:16110)")
                )
        )
        .subcommand(
            Command::new("tournament")
                .about("Tournament authentication mode")
                .arg(
                    Arg::new("create")
                        .long("create")
                        .help("Create a new tournament")
                )
                .arg(
                    Arg::new("max-players")
                        .long("max-players")
                        .value_name("COUNT")
                        .default_value("100")
                )
        )
        .subcommand(
            Command::new("client")
                .about("Run auth client on Kaspa testnet-10")
                .arg(
                    Arg::new("auth")
                        .long("auth")
                        .action(clap::ArgAction::SetTrue)
                        .help("Initiate authentication flow")
                )
                .arg(
                    Arg::new("key")
                        .short('k')
                        .long("key")
                        .value_name("PRIVATE_KEY")
                        .help("Private key (hex format) - generates random if not provided")
                )
                .arg(
                    Arg::new("kaspa-private-key")
                        .long("kaspa-private-key")
                        .value_name("KASPA_PRIVATE_KEY")
                        .help("Kaspa private key for funding transactions (hex format)")
                )
                .arg(
                    Arg::new("rpc-url")
                        .long("rpc-url")
                        .value_name("URL")
                        .help("Kaspa node RPC URL (e.g., grpc://127.0.0.1:16110)")
                )
        )
        .get_matches();

    match matches.subcommand() {
        Some(("test-episode", sub_matches)) => {
            let participant_count: usize = sub_matches
                .get_one::<String>("participants")
                .unwrap()
                .parse()
                .unwrap_or(1);
            
            test_episode_logic(participant_count)?;
        }
        Some(("http-server", sub_matches)) => {
            let port: u16 = sub_matches
                .get_one::<String>("port")
                .unwrap()
                .parse()
                .unwrap_or(8080);
            
            let keypair = if let Some(key_hex) = sub_matches.get_one::<String>("key") {
                parse_private_key(key_hex)?
            } else {
                generate_random_keypair()
            };
            
            info!("🔑 HTTP Server public key: {}", hex::encode(keypair.public_key().serialize()));
            run_http_server(keypair, port).await?;
        }
        Some(("authenticate", sub_matches)) => {
            let server_url = sub_matches.get_one::<String>("server").unwrap().clone();
            
            // Get private key from various sources
            let keypair = if let Some(keyfile_path) = sub_matches.get_one::<String>("keyfile") {
                load_private_key_from_file(keyfile_path)?
            } else if let Some(key_hex) = sub_matches.get_one::<String>("key") {
                parse_private_key(key_hex)?
            } else {
                // Generate a random key for this session (safer than hardcoded)
                println!("🔑 No key provided - generating random keypair for this session");
                println!("📝 For production, use: --key YOUR_PRIVATE_KEY or --keyfile YOUR_KEYFILE");
                println!("⚠️  This random key will only work if server uses the same key!");
                println!();
                generate_random_keypair()
            };
            
            println!("🚀 Starting automatic authentication with server: {}", server_url);
            run_automatic_authentication(server_url, keypair).await?;
        }
        Some(("demo", _)) => {
            run_interactive_demo()?;
        }
        Some(("server", sub_matches)) => {
            let name = sub_matches.get_one::<String>("name").unwrap().clone();
            let rpc_url = sub_matches.get_one::<String>("rpc-url").cloned();
            let keypair = if let Some(key_hex) = sub_matches.get_one::<String>("key") {
                parse_private_key(key_hex)?
            } else {
                generate_random_keypair()
            };
            
            info!("🔑 Server public key: {}", hex::encode(keypair.public_key().serialize()));
            run_kaspa_server(keypair, name, rpc_url).await?;
        }
        Some(("client", sub_matches)) => {
            let should_auth = sub_matches.get_flag("auth");
            let rpc_url = sub_matches.get_one::<String>("rpc-url").cloned();
            
            // Get Kaspa keypair (for funding transactions)
            let kaspa_keypair = if let Some(kaspa_key_hex) = sub_matches.get_one::<String>("kaspa-private-key") {
                parse_private_key(kaspa_key_hex)?
            } else if should_auth {
                // If doing auth and no kaspa key provided, show how to generate one
                let keypair = generate_random_keypair();
                let kaspa_addr = kaspa_addresses::Address::new(
                    kaspa_addresses::Prefix::Testnet,
                    kaspa_addresses::Version::PubKey,
                    &keypair.x_only_public_key().0.serialize()
                );
                println!("No --kaspa-private-key provided. Generated:");
                println!("Kaspa Address: {}", kaspa_addr);
                println!("Private Key: {}", hex::encode(keypair.secret_key().secret_bytes()));
                println!();
                println!("Send testnet funds to this address, then run:");
                println!("cargo run -p kaspa-auth -- client --auth --kaspa-private-key {}", hex::encode(keypair.secret_key().secret_bytes()));
                return Ok(());
            } else {
                generate_random_keypair()
            };
            
            // Get auth keypair (for episode authentication)
            let auth_keypair = if let Some(key_hex) = sub_matches.get_one::<String>("key") {
                parse_private_key(key_hex)?
            } else {
                generate_random_keypair()
            };
            
            info!("🔑 Auth public key: {}", hex::encode(auth_keypair.public_key().serialize()));
            run_kaspa_client(kaspa_keypair, auth_keypair, should_auth, rpc_url).await?;
        }
        _ => {
            println!("No subcommand specified. Use --help for available commands.");
            println!("\nAvailable commands:");
            println!("  authenticate  - 🚀 Easy one-command authentication (RECOMMENDED)");
            println!("  test-episode  - Test locally (no Kaspa network)");
            println!("  http-server   - Run HTTP coordination server");
            println!("  demo         - Interactive demo (simulated)");
            println!("  server       - Run auth server on testnet-10");
            println!("  client       - Run auth client on testnet-10");
        }
    }

    Ok(())
}

fn test_episode_logic(participant_count: usize) -> Result<(), Box<dyn Error>> {
    println!("🎯 Testing SimpleAuth Episode Logic");
    println!("Participants: {}", participant_count);

    // Generate keypairs for participants
    let mut keypairs = Vec::new();
    let mut pubkeys = Vec::new();
    
    for i in 0..participant_count {
        let (secret_key, pub_key) = generate_keypair();
        println!("Generated keypair {} for participant: {}", i + 1, pub_key);
        keypairs.push((secret_key, pub_key));
        pubkeys.push(pub_key);
    }

    // Create metadata
    let metadata = PayloadMetadata {
        accepting_hash: 0u64.into(),
        accepting_daa: 0,
        accepting_time: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        tx_id: 1u64.into(),
    };

    // Initialize episode
    let mut auth_episode = SimpleAuth::initialize(pubkeys.clone(), &metadata);
    println!("✅ Episode initialized");

    // Test authentication flow for first participant
    let (secret_key, pub_key) = &keypairs[0];
    
    println!("\n🔑 Testing authentication flow for participant: {}", pub_key);

    // Step 1: Request challenge
    println!("📨 Requesting challenge...");
    let rollback1 = auth_episode.execute(
        &AuthCommand::RequestChallenge,
        Some(*pub_key),
        &metadata,
    )?;
    
    let challenge = auth_episode.challenge.clone().unwrap();
    println!("🎲 Received challenge: {}", challenge);

    // Step 2: Sign challenge
    println!("✍️ Signing challenge...");
    let msg = to_message(&challenge.to_string());
    let signature = sign_message(secret_key, &msg);
    println!("📝 Signature created");

    // Step 3: Submit response
    println!("📤 Submitting signed response...");
    let rollback2 = auth_episode.execute(
        &AuthCommand::SubmitResponse {
            signature: hex::encode(signature.0.serialize_der()),
            nonce: challenge,
        },
        Some(*pub_key),
        &metadata,
    )?;

    // Check results
    if auth_episode.is_authenticated {
        println!("✅ Authentication successful!");
        if let Some(ref token) = auth_episode.session_token {
            println!("🎟️ Session token: {}", token);
        }
    } else {
        println!("❌ Authentication failed");
    }

    // Test rollback functionality
    println!("\n🔄 Testing rollback functionality...");
    let rollback_success = auth_episode.rollback(rollback2);
    println!("Rollback authentication: {}", if rollback_success { "✅" } else { "❌" });
    
    let rollback_success = auth_episode.rollback(rollback1);
    println!("Rollback challenge: {}", if rollback_success { "✅" } else { "❌" });

    println!("\n🎉 Episode logic test completed successfully!");
    Ok(())
}

fn run_interactive_demo() -> Result<(), Box<dyn Error>> {
    println!("🚀 Kaspa Auth Interactive Demo");
    println!("This will simulate a two-party authentication flow");
    
    // Generate two keypairs (Alice and Bob)
    let (alice_sk, alice_pk) = generate_keypair();
    let (_, bob_pk) = generate_keypair();
    
    println!("\n👥 Participants:");
    println!("Alice (requester): {}", alice_pk);
    println!("Bob (verifier): {}", bob_pk);

    let metadata = PayloadMetadata {
        accepting_hash: 0u64.into(),
        accepting_daa: 0,
        accepting_time: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        tx_id: 1u64.into(),
    };

    // Initialize episode with both participants
    let mut auth_episode = SimpleAuth::initialize(vec![alice_pk, bob_pk], &metadata);
    
    println!("\n📡 Episode initialized on simulated Kaspa network");
    
    // Alice requests authentication
    println!("\n🔐 Alice initiates authentication...");
    let _rollback = auth_episode.execute(
        &AuthCommand::RequestChallenge,
        Some(alice_pk),
        &metadata,
    )?;
    
    let challenge = auth_episode.challenge.clone().unwrap();
    println!("📨 Bob sends challenge to Alice: {}", challenge);
    
    // Alice signs the challenge
    println!("✍️ Alice signs the challenge...");
    let msg = to_message(&challenge.to_string());
    let signature = sign_message(&alice_sk, &msg);
    
    // Alice submits signed response
    println!("📤 Alice submits signed response to Bob...");
    let _rollback = auth_episode.execute(
        &AuthCommand::SubmitResponse {
            signature: hex::encode(signature.0.serialize_der()),
            nonce: challenge,
        },
        Some(alice_pk),
        &metadata,
    )?;
    
    // Show final result
    println!("\n🎯 Final Result:");
    if auth_episode.is_authenticated {
        println!("✅ Alice successfully authenticated!");
        if let Some(ref token) = auth_episode.session_token {
            println!("🎟️ Session token issued: {}", token);
        }
        println!("🎉 Authentication complete - Alice can now access protected resources");
    } else {
        println!("❌ Authentication failed");
    }
    
    Ok(())
}

// Helper functions for Kaspa integration

/// Parse a private key from hex string
fn parse_private_key(hex_str: &str) -> Result<Keypair, Box<dyn Error>> {
    let secp = Secp256k1::new();
    let secret_bytes = hex::decode(hex_str)?;
    let secret_key = SecretKey::from_slice(&secret_bytes)?;
    Ok(Keypair::from_secret_key(&secp, &secret_key))
}

/// Generate a random keypair for development
fn generate_random_keypair() -> Keypair {
    let secp = Secp256k1::new();
    let secret_key = SecretKey::new(&mut rand::thread_rng());
    Keypair::from_secret_key(&secp, &secret_key)
}

/// Load private key from file (secure alternative to command line)
fn load_private_key_from_file(path: &str) -> Result<Keypair, Box<dyn Error>> {
    use std::fs;
    let key_hex = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read keyfile {}: {}", path, e))?
        .trim()
        .to_string();
    parse_private_key(&key_hex)
}

/// Run Kaspa authentication server
async fn run_kaspa_server(signer: Keypair, name: String, rpc_url: Option<String>) -> Result<(), Box<dyn Error>> {
    println!("🎯 Starting Kaspa Auth Server: {}", name);
    if let Some(url) = &rpc_url {
        println!("📡 Connecting to node: {}", url);
    } else {
        println!("📡 Connecting to testnet-10 (public node)...");
    }
    
    let config = AuthServerConfig::new(signer, name, rpc_url);
    run_auth_server(config).await?;
    
    Ok(())
}

/// Run Kaspa authentication client
async fn run_kaspa_client(kaspa_signer: Keypair, auth_signer: Keypair, should_auth: bool, rpc_url: Option<String>) -> Result<(), Box<dyn Error>> {
    println!("🔑 Starting Kaspa Auth Client");
    if let Some(url) = &rpc_url {
        println!("📡 Connecting to node: {}", url);
    } else {
        println!("📡 Connecting to testnet-10 (public node)...");
    }
    
    if should_auth {
        println!("🚀 Initiating authentication flow...");
        run_client_authentication(kaspa_signer, auth_signer).await?;
    } else {
        println!("👂 Listening for authentication requests...");
        // For now, just run a server instance
        let config = AuthServerConfig::new(kaspa_signer, "auth-client".to_string(), rpc_url);
        run_auth_server(config).await?;
    }
    
    Ok(())
}

/// Implement REAL client authentication flow using kdapp blockchain architecture
async fn run_client_authentication(kaspa_signer: Keypair, auth_signer: Keypair) -> Result<(), Box<dyn Error>> {
    use kdapp::{
        engine::EpisodeMessage,
        generator::{self, TransactionGenerator},
        proxy::connect_client,
    };
    use kaspa_addresses::{Address, Prefix, Version};
    use kaspa_consensus_core::{network::NetworkId, tx::{TransactionOutpoint, UtxoEntry}};
    use kaspa_wrpc_client::prelude::*;
    use kaspa_rpc_core::api::rpc::RpcApi;
    use kaspa_auth::episode_runner::{AUTH_PATTERN, AUTH_PREFIX};
    use rand::Rng;
    
    let client_pubkey = kdapp::pki::PubKey(auth_signer.public_key());
    println!("🔑 Auth public key: {}", client_pubkey);
    
    // Connect to Kaspa network (real blockchain!)
    let network = NetworkId::with_suffix(kaspa_consensus_core::network::NetworkType::Testnet, 10);
    println!("📡 Connecting to testnet-10 blockchain...");
    
    let kaspad = connect_client(network, None).await?;
    
    // Create Kaspa address for funding transactions
    let kaspa_addr = Address::new(Prefix::Testnet, Version::PubKey, &kaspa_signer.x_only_public_key().0.serialize());
    println!("💰 Kaspa address: {}", kaspa_addr);
    
    // Get UTXOs for transaction funding
    println!("🔍 Fetching UTXOs...");
    let entries = kaspad.get_utxos_by_addresses(vec![kaspa_addr.clone()]).await?;
    
    if entries.is_empty() {
        return Err("No UTXOs found! Please fund the Kaspa address first.".into());
    }
    
    let mut utxo = entries.first().map(|entry| {
        (TransactionOutpoint::from(entry.outpoint.clone()), UtxoEntry::from(entry.utxo_entry.clone()))
    }).unwrap();
    
    println!("✅ UTXO found: {}", utxo.0);
    
    // Create real transaction generator (kdapp architecture!)
    let generator = TransactionGenerator::new(kaspa_signer, AUTH_PATTERN, AUTH_PREFIX);
    
    // Step 1: Initialize the episode first (like tictactoe example)
    println!("🚀 Initializing authentication episode...");
    
    let episode_id = rand::thread_rng().gen();
    let new_episode = EpisodeMessage::<SimpleAuth>::NewEpisode { 
        episode_id, 
        participants: vec![client_pubkey] 
    };
    
    let tx = generator.build_command_transaction(utxo, &kaspa_addr, &new_episode, 5000);
    println!("🚀 Submitting NewEpisode transaction: {}", tx.id());
    
    let _res = kaspad.submit_transaction(tx.as_ref().into(), false).await?;
    utxo = generator::get_first_output_utxo(&tx);
    
    println!("✅ Episode {} initialized on blockchain!", episode_id);
    
    // Step 2: Send RequestChallenge command to blockchain
    println!("📨 Sending RequestChallenge command to blockchain...");
    
    let auth_command = AuthCommand::RequestChallenge;
    let step = EpisodeMessage::<SimpleAuth>::new_signed_command(
        episode_id, 
        auth_command, 
        auth_signer.secret_key(), 
        client_pubkey
    );
    
    let tx = generator.build_command_transaction(utxo, &kaspa_addr, &step, 5000);
    println!("🚀 Submitting RequestChallenge transaction: {}", tx.id());
    
    let _res = kaspad.submit_transaction(tx.as_ref().into(), false).await?;
    utxo = generator::get_first_output_utxo(&tx);
    
    println!("✅ RequestChallenge transaction submitted to blockchain!");
    println!("⏳ Waiting for challenge response from auth server...");
    
    // Set up episode state listener (like tictactoe example)
    use std::sync::{mpsc::channel, Arc, atomic::AtomicBool};
    use tokio::sync::mpsc::UnboundedSender;
    use kdapp::{engine::{self}, episode::EpisodeEventHandler};
    use kaspa_auth::core::episode::SimpleAuth;
    
    let (sender, receiver) = channel();
    let (response_sender, mut response_receiver) = tokio::sync::mpsc::unbounded_channel();
    let exit_signal = Arc::new(AtomicBool::new(false));
    
    // Simple event handler to capture episode state
    struct ClientAuthHandler {
        sender: UnboundedSender<(kdapp::episode::EpisodeId, SimpleAuth)>,
    }
    
    impl EpisodeEventHandler<SimpleAuth> for ClientAuthHandler {
        fn on_initialize(&self, episode_id: kdapp::episode::EpisodeId, episode: &SimpleAuth) {
            let _ = self.sender.send((episode_id, episode.clone()));
        }
        
        fn on_command(&self, episode_id: kdapp::episode::EpisodeId, episode: &SimpleAuth, 
                      _cmd: &AuthCommand, _authorization: Option<kdapp::pki::PubKey>, 
                      _metadata: &kdapp::episode::PayloadMetadata) {
            let _ = self.sender.send((episode_id, episode.clone()));
        }
        
        fn on_rollback(&self, _episode_id: kdapp::episode::EpisodeId, _episode: &SimpleAuth) {}
    }
    
    // Start a simple engine to listen for episode updates
    let mut engine = engine::Engine::<SimpleAuth, ClientAuthHandler>::new(receiver);
    let handler = ClientAuthHandler { sender: response_sender };
    
    let engine_task = tokio::task::spawn_blocking(move || {
        engine.start(vec![handler]);
    });
    
    // Connect client proxy to listen for episode updates
    let client_kaspad = connect_client(network, None).await?;
    let engines = std::iter::once((AUTH_PREFIX, (AUTH_PATTERN, sender))).collect();
    
    let exit_signal_clone = exit_signal.clone();
    tokio::spawn(async move {
        kdapp::proxy::run_listener(client_kaspad, engines, exit_signal_clone).await;
    });
    
    // Wait for challenge to be generated by server
    println!("👂 Listening for episode state updates...");
    println!("🔍 Looking for episode ID: {}", episode_id);
    let mut challenge = String::new();
    let mut attempt_count = 0;
    let max_attempts = 10; // 1 second timeout - HTTP coordination is primary
    
    // Wait for episode state with challenge
    'outer: loop {
        attempt_count += 1;
        
        if let Ok((received_episode_id, episode_state)) = response_receiver.try_recv() {
            println!("📨 Received episode state update for ID: {} (expecting: {})", received_episode_id, episode_id);
            if received_episode_id == episode_id {
                if let Some(server_challenge) = &episode_state.challenge {
                    challenge = server_challenge.clone();
                    println!("🎲 Real challenge received from server: {}", challenge);
                    break;
                } else {
                    println!("📡 Episode state update received, but no challenge yet. Auth status: {}", episode_state.is_authenticated);
                }
            } else {
                println!("🔄 Episode ID mismatch, continuing to listen...");
            }
        }
        
        if attempt_count % 25 == 0 {
            println!("⏰ Still listening... attempt {} of {}", attempt_count, max_attempts);
        }
        
        if attempt_count >= max_attempts {
            println!("⚠️ Timeout waiting for challenge. Using HTTP fallback...");
            
            // Step 1: Register episode with HTTP server
            let client = reqwest::Client::new();
            let public_key_hex = hex::encode(client_pubkey.0.serialize());
            
            println!("📝 Registering episode {} with HTTP server...", episode_id);
            
            // First, try to register the episode with HTTP server using a custom endpoint
            // Since we generated the episode ID on blockchain, we need to register it
            let register_url = format!("http://127.0.0.1:8080/auth/register-episode");
            let register_response = client
                .post(&register_url)
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "episode_id": episode_id,
                    "public_key": public_key_hex
                }))
                .send()
                .await;
            
            if register_response.is_ok() {
                println!("✅ Episode registered with HTTP server");
            } else {
                println!("⚠️ Could not register episode, trying legacy endpoint...");
            }
            
            // Step 2: Try to get challenge via HTTP
            for retry_attempt in 1..=5 {
                println!("🔄 HTTP retry attempt {} of 5...", retry_attempt);
                
                // Try both the new status endpoint and legacy challenge endpoint
                let status_url = format!("http://127.0.0.1:8080/auth/status/{}", episode_id);
                let challenge_url = format!("http://127.0.0.1:8080/challenge/{}", episode_id);
                
                // First try the status endpoint
                match client.get(&status_url).send().await {
                    Ok(response) if response.status().is_success() => {
                        if let Ok(status_json) = response.text().await {
                            println!("📡 HTTP status response: {}", status_json);
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&status_json) {
                                if let Some(server_challenge) = parsed["challenge"].as_str() {
                                    challenge = server_challenge.to_string();
                                    println!("🎯 Challenge retrieved via HTTP status: {}", challenge);
                                    break 'outer;
                                }
                            }
                        }
                    }
                    _ => {
                        // If status fails, try legacy challenge endpoint
                        match client.get(&challenge_url).send().await {
                            Ok(response) if response.status().is_success() => {
                                if let Ok(challenge_json) = response.text().await {
                                    println!("📡 HTTP legacy response: {}", challenge_json);
                                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&challenge_json) {
                                        if let Some(server_challenge) = parsed["challenge"].as_str() {
                                            challenge = server_challenge.to_string();
                                            println!("🎯 Challenge retrieved via HTTP legacy: {}", challenge);
                                            break 'outer;
                                        }
                                    }
                                }
                            }
                            _ => {
                                println!("❌ HTTP attempt {} failed", retry_attempt);
                            }
                        }
                    }
                }
                
                // Wait before retry
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            
            // All attempts failed - exit with error
            return Err("❌ AUTHENTICATION FAILED: Could not retrieve challenge from server after multiple attempts. Please ensure the auth server is running and accessible.".into());
        }
        
        // Add timeout to prevent infinite waiting
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    // Stop listening after we get the challenge
    exit_signal.store(true, std::sync::atomic::Ordering::Relaxed);
    
    // Step 3: Sign challenge and send SubmitResponse command to blockchain
    println!("✍️ Signing challenge...");
    let msg = to_message(&challenge);
    let signature = sign_message(&auth_signer.secret_key(), &msg);
    let signature_hex = hex::encode(signature.0.serialize_der());
    
    println!("📤 Sending SubmitResponse command to blockchain...");
    let auth_command = AuthCommand::SubmitResponse {
        signature: signature_hex,
        nonce: challenge,
    };
    
    let step = EpisodeMessage::<SimpleAuth>::new_signed_command(
        episode_id, 
        auth_command, 
        auth_signer.secret_key(), 
        client_pubkey
    );
    
    let tx = generator.build_command_transaction(utxo, &kaspa_addr, &step, 5000);
    println!("🚀 Submitting SubmitResponse transaction: {}", tx.id());
    
    let _res = kaspad.submit_transaction(tx.as_ref().into(), false).await?;
    
    println!("✅ Authentication commands submitted to Kaspa blockchain!");
    println!("🎯 Real kdapp architecture: Generator → Proxy → Engine → Episode");
    println!("📊 Transactions are now being processed by auth server's kdapp engine");
    
    Ok(())
}

/// 🚀 Automatic authentication - handles entire flow seamlessly
async fn run_automatic_authentication(server_url: String, keypair: Keypair) -> Result<(), Box<dyn Error>> {
    use serde_json::Value;
    
    let client = reqwest::Client::new();
    let public_key_hex = hex::encode(keypair.public_key().serialize());
    
    println!("🔑 Using public key: {}", public_key_hex);
    println!();
    
    // Step 1: Create episode
    println!("📝 Step 1: Creating authentication episode...");
    let start_response = client
        .post(&format!("{}/auth/start", server_url))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "public_key": public_key_hex
        }))
        .send()
        .await?;
    
    if !start_response.status().is_success() {
        return Err(format!("Failed to create episode: {}", start_response.status()).into());
    }
    
    let start_data: Value = start_response.json().await?;
    let episode_id = start_data["episode_id"].as_u64()
        .ok_or("Invalid episode_id in response")?;
    
    println!("✅ Episode created: {}", episode_id);
    
    // Step 2: Request challenge
    println!("🎲 Step 2: Requesting challenge from blockchain...");
    let challenge_response = client
        .post(&format!("{}/auth/request-challenge", server_url))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "episode_id": episode_id,
            "public_key": public_key_hex
        }))
        .send()
        .await?;
    
    if !challenge_response.status().is_success() {
        return Err(format!("Failed to request challenge: {}", challenge_response.status()).into());
    }
    
    println!("✅ Challenge requested, waiting for blockchain processing...");
    
    // Step 3: Wait for challenge to be ready
    println!("⏳ Step 3: Waiting for challenge generation...");
    let mut challenge = String::new();
    for attempt in 1..=10 {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        
        let status_response = client
            .get(&format!("{}/auth/status/{}", server_url, episode_id))
            .send()
            .await?;
        
        if status_response.status().is_success() {
            let status_data: Value = status_response.json().await?;
            if let Some(challenge_value) = status_data["challenge"].as_str() {
                challenge = challenge_value.to_string();
                println!("✅ Challenge received: {}", challenge);
                break;
            }
        }
        
        if attempt < 10 {
            print!("⏳ Attempt {}/10, still waiting...\r", attempt);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        } else {
            return Err("Timeout waiting for challenge generation".into());
        }
    }
    println!();
    
    // Step 4: Sign challenge locally (SECURE - no private key sent!)
    println!("✍️  Step 4: Signing challenge locally (private key stays secure)...");
    let message = to_message(&challenge);
    let signature = sign_message(&keypair.secret_key(), &message);
    let signature_hex = hex::encode(signature.0.serialize_der());
    
    println!("✅ Challenge signed locally");
    
    // Step 5: Submit verification
    println!("📤 Step 5: Submitting authentication response...");
    let verify_response = client
        .post(&format!("{}/auth/verify", server_url))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "episode_id": episode_id,
            "signature": signature_hex,
            "nonce": challenge
        }))
        .send()
        .await?;
    
    if !verify_response.status().is_success() {
        return Err(format!("Failed to submit verification: {}", verify_response.status()).into());
    }
    
    println!("✅ Authentication response submitted");
    
    // Step 6: Check final status
    println!("🔍 Step 6: Checking authentication result...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    let final_status = client
        .get(&format!("{}/auth/status/{}", server_url, episode_id))
        .send()
        .await?;
    
    if final_status.status().is_success() {
        let final_data: Value = final_status.json().await?;
        let authenticated = final_data["authenticated"].as_bool().unwrap_or(false);
        
        if authenticated {
            let session_token = final_data["session_token"].as_str().unwrap_or("none");
            println!();
            println!("🎉 SUCCESS! Authentication completed!");
            println!("✅ Authenticated: true");
            println!("🎟️  Session token: {}", session_token);
            println!("📊 Episode ID: {}", episode_id);
            println!();
            println!("🚀 You are now authenticated with the Kaspa blockchain!");
        } else {
            println!("❌ Authentication failed - please check server logs");
            return Err("Authentication verification failed".into());
        }
    } else {
        return Err("Failed to check final authentication status".into());
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episode_creation() {
        let result = test_episode_logic(2);
        assert!(result.is_ok());
    }

    #[test]
    fn test_random_keypair_generation() {
        let keypair = generate_random_keypair();
        // Just verify that we can create a keypair
        assert!(!keypair.public_key().serialize().is_empty());
    }

    #[test]
    fn test_private_key_parsing() {
        // Test with a valid hex private key
        let test_key = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let result = parse_private_key(test_key);
        assert!(result.is_ok());
    }
}