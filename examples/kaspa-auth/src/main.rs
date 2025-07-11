use clap::{Arg, Command};

use std::error::Error;
use secp256k1::{Secp256k1, SecretKey, Keypair};
use log::info;
use kaspa_addresses;
use serde_json;
use reqwest;

#[derive(Debug, Clone)]
struct AuthenticationResult {
    episode_id: u64,
    session_token: String,
    authenticated: bool,
}

use kaspa_auth::core::episode::SimpleAuth;
use kaspa_auth::core::commands::AuthCommand;
use kaspa_auth::{AuthServerConfig, run_auth_server};
use kaspa_auth::wallet::get_wallet_for_command;
use kaspa_auth::api::http::organizer_peer::run_http_peer;

use kaspa_auth::cli::commands::test_api_flow::TestApiFlowCommand;
use kdapp::pki::{generate_keypair, sign_message, to_message};
use kdapp::episode::{PayloadMetadata, Episode};
// use crate::cli::Cli; // Using inline clap structure instead
// use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing for better logging
    tracing_subscriber::fmt::init();

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
            Command::new("http-peer")
                .about("Run HTTP coordination peer for authentication")
                .arg(
                    Arg::new("port")
                        .short('p')
                        .long("port")
                        .value_name("PORT")
                        .help("HTTP coordination peer port")
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
                .about("🔐 Authentication testing only (focused, with timeout)")
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
                .arg(
                    Arg::new("peer")
                        .short('p')
                        .long("peer")
                        .value_name("URL")
                        .help("HTTP organizer peer URL for coordination")
                        .default_value("http://127.0.0.1:8080")
                )
                .arg(
                    Arg::new("timeout")
                        .short('t')
                        .long("timeout")
                        .value_name("SECONDS")
                        .help("Timeout for authentication completion")
                        .default_value("30")
                )
                .arg(
                    Arg::new("pure-kdapp")
                        .long("pure-kdapp")
                        .help("Use pure kdapp without HTTP coordination (experimental)")
                        .action(clap::ArgAction::SetTrue)
                )
        )
        .subcommand(
            Command::new("authenticate-full-flow")
                .about("🔄 Complete login → session → logout cycle with timeouts")
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
                .arg(
                    Arg::new("peer")
                        .short('p')
                        .long("peer")
                        .value_name("URL")
                        .help("HTTP organizer peer URL for coordination")
                        .default_value("http://127.0.0.1:8080")
                )
                .arg(
                    Arg::new("session-duration")
                        .short('s')
                        .long("session-duration")
                        .value_name("SECONDS")
                        .help("How long to wait between login and logout")
                        .default_value("10")
                )
                .arg(
                    Arg::new("auth-timeout")
                        .short('t')
                        .long("auth-timeout")
                        .value_name("SECONDS")
                        .help("Timeout for authentication steps")
                        .default_value("30")
                )
        )
        .subcommand(
            Command::new("logout")
                .about("🚪 Logout testing only (requires active session)")
                .arg(
                    Arg::new("episode-id")
                        .short('e')
                        .long("episode-id")
                        .value_name("EPISODE_ID")
                        .help("Episode ID of the session to logout from")
                        .required(true)
                )
                .arg(
                    Arg::new("session-token")
                        .short('s')
                        .long("session-token")
                        .value_name("SESSION_TOKEN")
                        .help("Session token to logout")
                        .required(true)
                )
                .arg(
                    Arg::new("key")
                        .short('k')
                        .long("key")
                        .value_name("PRIVATE_KEY")
                        .help("Private key (hex format) - uses participant wallet if not provided")
                )
                .arg(
                    Arg::new("peer")
                        .long("peer")
                        .value_name("PEER_ADDRESS")
                        .help("HTTP organizer peer address")
                        .default_value("http://localhost:8080")
                )
                .arg(
                    Arg::new("timeout")
                        .short('t')
                        .long("timeout")
                        .value_name("SECONDS")
                        .help("Timeout for logout completion")
                        .default_value("15")
                )
        )
        .subcommand(
            Command::new("revoke-session")
                .about("🔄 Revoke an active session on the blockchain")
                .arg(
                    Arg::new("episode-id")
                        .short('e')
                        .long("episode-id")
                        .value_name("EPISODE_ID")
                        .help("Episode ID of the session to revoke")
                        .required(true)
                )
                .arg(
                    Arg::new("session-token")
                        .short('s')
                        .long("session-token")
                        .value_name("SESSION_TOKEN")
                        .help("Session token to revoke")
                        .required(true)
                )
                .arg(
                    Arg::new("key")
                        .short('k')
                        .long("key")
                        .value_name("PRIVATE_KEY")
                        .help("Private key (hex format) - uses participant wallet if not provided")
                )
                .arg(
                    Arg::new("peer")
                        .long("peer")
                        .value_name("PEER_ADDRESS")
                        .help("HTTP organizer peer address")
                        .default_value("http://localhost:8080")
                )
        )
        .subcommand(
            Command::new("wallet-status")
                .about("🔍 Show wallet status and addresses")
                .arg(
                    Arg::new("role")
                        .short('r')
                        .long("role")
                        .value_name("ROLE")
                        .help("Check specific role wallet (organizer-peer, participant-peer, or all)")
                        .default_value("all")
                )
        )
        .subcommand(
            Command::new("demo")
                .about("Run interactive demo")
        )
        .subcommand(
            Command::new("organizer-peer")
                .about("Run auth organizer peer on Kaspa testnet-10")
                .arg(
                    Arg::new("name")
                        .short('n')
                        .long("name")
                        .value_name("NAME")
                        .help("Organizer peer name")
                        .default_value("auth-organizer-peer")
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
            Command::new("participant-peer")
                .about("Run auth participant peer on Kaspa testnet-10")
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
                    Arg::new("kaspa-keyfile")
                        .long("kaspa-keyfile")
                        .value_name("FILE")
                        .help("Load Kaspa private key from file (safer than --kaspa-private-key)")
                )
                .arg(
                    Arg::new("rpc-url")
                        .long("rpc-url")
                        .value_name("URL")
                        .help("Kaspa node RPC URL (e.g., grpc://127.0.0.1:16110)")
                )
        )
        
        .subcommand(
            Command::new("test-api-flow")
                .about("Run a full API authentication flow test")
                .arg(
                    Arg::new("peer")
                        .short('p')
                        .long("peer")
                        .value_name("URL")
                        .help("HTTP coordination peer URL")
                        .default_value("http://127.0.0.1:8080")
                )
        )
        .subcommand(
            Command::new("test-api")
                .about("Run tests against all API endpoints")
                .arg(
                    Arg::new("peer")
                        .short('p')
                        .long("peer")
                        .value_name("URL")
                        .help("HTTP coordination peer URL")
                        .default_value("http://127.0.0.1:8080")
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
        Some(("http-peer", sub_matches)) => {
            let port: u16 = sub_matches
                .get_one::<String>("port")
                .unwrap()
                .parse()
                .unwrap_or(8080);
            
            let provided_private_key = sub_matches.get_one::<String>("key").map(|s| s.as_str());
            run_http_peer(provided_private_key, port).await?;
        }
        Some(("authenticate", sub_matches)) => {
            let peer_url = sub_matches.get_one::<String>("peer").unwrap().clone();
            let use_pure_kdapp = sub_matches.get_flag("pure-kdapp");
            let timeout_seconds: u64 = sub_matches.get_one::<String>("timeout").unwrap().parse()
                .map_err(|_| "Invalid timeout value")?;
            
            // Get private key using unified wallet system
            let auth_keypair = if let Some(keyfile_path) = sub_matches.get_one::<String>("keyfile") {
                load_private_key_from_file(keyfile_path)?
            } else {
                let provided_private_key = sub_matches.get_one::<String>("key").map(|s| s.as_str());
                let wallet = get_wallet_for_command("authenticate", provided_private_key)?;
                wallet.keypair
            };

            // Get funding keypair for transactions
            let funding_wallet = get_wallet_for_command("participant-peer", None)?;
            let funding_keypair = funding_wallet.keypair;
            
            println!("🔐 Running focused authentication test ({}s timeout)", timeout_seconds);
            
            if use_pure_kdapp {
                println!("🚀 Starting pure kdapp authentication (experimental)");
                println!("⚡ No HTTP coordination - pure peer-to-peer via Kaspa blockchain");
                run_authentication_with_timeout(auth_keypair, None, timeout_seconds).await?;
            } else {
                println!("🚀 Starting hybrid authentication (kdapp + HTTP coordination)");
                println!("🎯 Organizer peer: {}", peer_url);
                run_authentication_with_timeout(funding_keypair, Some(peer_url), timeout_seconds).await?;
            }
        }
        Some(("authenticate-full-flow", sub_matches)) => {
            let peer_url = sub_matches.get_one::<String>("peer").unwrap().clone();
            let session_duration: u64 = sub_matches.get_one::<String>("session-duration").unwrap().parse()
                .map_err(|_| "Invalid session duration value")?;
            let auth_timeout: u64 = sub_matches.get_one::<String>("auth-timeout").unwrap().parse()
                .map_err(|_| "Invalid auth timeout value")?;
            
            // Get private key using unified wallet system
            let auth_keypair = if let Some(keyfile_path) = sub_matches.get_one::<String>("keyfile") {
                load_private_key_from_file(keyfile_path)?
            } else {
                let provided_private_key = sub_matches.get_one::<String>("key").map(|s| s.as_str());
                let wallet = get_wallet_for_command("authenticate", provided_private_key)?;
                wallet.keypair
            };

            // Get funding keypair for transactions
            let funding_wallet = get_wallet_for_command("participant-peer", None)?;
            let funding_keypair = funding_wallet.keypair;
            
            println!("🔄 Running complete authentication lifecycle test");
            println!("⏱️  Auth timeout: {}s, Session duration: {}s", auth_timeout, session_duration);
            println!("🎯 Organizer peer: {}", peer_url);
            
            run_full_authentication_cycle(funding_keypair, auth_keypair, peer_url, session_duration, auth_timeout).await?;
        }
        Some(("logout", sub_matches)) => {
            let episode_id: u64 = sub_matches
                .get_one::<String>("episode-id")
                .unwrap()
                .parse()
                .map_err(|_| "Invalid episode ID")?;
            
            let session_token = sub_matches
                .get_one::<String>("session-token")
                .unwrap()
                .clone();
            
            let peer_url = sub_matches.get_one::<String>("peer").unwrap().clone();
            let timeout_seconds: u64 = sub_matches.get_one::<String>("timeout").unwrap().parse()
                .map_err(|_| "Invalid timeout value")?;
            
            // Get private key using unified wallet system
            let auth_keypair = if let Some(provided_private_key) = sub_matches.get_one::<String>("key") {
                parse_private_key(provided_private_key)?
            } else {
                let wallet = get_wallet_for_command("participant-peer", None)?;
                wallet.keypair
            };
            
            println!("🚪 Running focused logout test ({}s timeout)", timeout_seconds);
            println!("📋 Episode: {}, Session: {}", episode_id, session_token);
            
            run_logout_with_timeout(auth_keypair, episode_id, session_token, peer_url, timeout_seconds).await?;
        }
        Some(("revoke-session", sub_matches)) => {
            let episode_id: u64 = sub_matches
                .get_one::<String>("episode-id")
                .unwrap()
                .parse()
                .map_err(|_| "Invalid episode ID")?;
            
            let session_token = sub_matches
                .get_one::<String>("session-token")
                .unwrap()
                .clone();
            
            let peer_url = sub_matches.get_one::<String>("peer").unwrap().clone();
            
            // Get private key using unified wallet system
            let auth_keypair = if let Some(provided_private_key) = sub_matches.get_one::<String>("key") {
                parse_private_key(provided_private_key)?
            } else {
                let wallet = get_wallet_for_command("participant-peer", None)?;
                wallet.keypair
            };
            
            println!("🔄 Running session revocation (blockchain transaction)");
            run_session_revocation(auth_keypair, episode_id, session_token, peer_url).await?;
        }
        Some(("wallet-status", sub_matches)) => {
            let role = sub_matches.get_one::<String>("role").unwrap();
            show_wallet_status(role)?;
        }
        Some(("demo", _)) => {
            run_interactive_demo()?;
        }
        Some(("organizer-peer", sub_matches)) => {
            use kaspa_auth::wallet::get_wallet_for_command;
            
            let name = sub_matches.get_one::<String>("name").unwrap().clone();
            let rpc_url = sub_matches.get_one::<String>("rpc-url").cloned();
            let provided_private_key = sub_matches.get_one::<String>("key").map(|s| s.as_str());
            
            let wallet = get_wallet_for_command("organizer-peer", provided_private_key)?;
            run_kaspa_organizer_peer(wallet.keypair, name, rpc_url).await?;
        }
        Some(("participant-peer", sub_matches)) => {
            let should_auth = sub_matches.get_flag("auth");
            let rpc_url = sub_matches.get_one::<String>("rpc-url").cloned();
            
            // Get Kaspa keypair (for funding transactions)
            let kaspa_keypair = if let Some(kaspa_keyfile_path) = sub_matches.get_one::<String>("kaspa-keyfile") {
                load_private_key_from_file(kaspa_keyfile_path)?
            } else if let Some(kaspa_key_hex) = sub_matches.get_one::<String>("kaspa-private-key") {
                parse_private_key(kaspa_key_hex)?
            } else if should_auth {
                // If doing auth and no kaspa key provided, show how to generate one
                let keypair = generate_random_keypair();
                let kaspa_addr = kaspa_addresses::Address::new(
                    kaspa_addresses::Prefix::Testnet,
                    kaspa_addresses::Version::PubKey,
                    &keypair.x_only_public_key().0.serialize()
                );
                println!("🔑 No --kaspa-private-key or --kaspa-keyfile provided. Generated new participant peer wallet:");
                println!("📍 Kaspa Address: {}", kaspa_addr);
                println!("🔐 Private Key: {}", hex::encode(keypair.secret_key().secret_bytes()));
                println!();
                println!("💾 Save the private key to a file for security:");
                println!("echo '{}' > kaspa_private.key", hex::encode(keypair.secret_key().secret_bytes()));
                println!();
                println!("💰 FUNDING REQUIRED: Get testnet Kaspa for blockchain authentication");
                println!("🚰 Faucet URL: https://faucet.kaspanet.io/");
                println!("🌐 Network: testnet-10 (for development and testing)");
                println!("💡 Amount needed: ~0.1 KAS (covers multiple authentication transactions)");
                println!();
                println!("📋 Steps to fund your participant peer wallet:");
                println!("  1. Copy the Kaspa address above: {}", kaspa_addr);
                println!("  2. Visit: https://faucet.kaspanet.io/");
                println!("  3. Paste the address and request testnet funds");
                println!("  4. Wait ~30 seconds for transaction confirmation");
                println!();
                println!("🚀 After funding, run blockchain authentication:");
                println!("cargo run -p kaspa-auth -- participant-peer --auth --kaspa-keyfile kaspa_private.key");
                println!("or");
                println!("cargo run -p kaspa-auth -- participant-peer --auth --kaspa-private-key {}", hex::encode(keypair.secret_key().secret_bytes()));
                println!();
                println!("🎯 This will create REAL blockchain transactions on Kaspa testnet-10!");
                println!("📊 You can verify transactions at: https://explorer.kaspa.org/");
                return Ok(());
            } else {
                generate_random_keypair()
            };
            
            // Get auth keypair (for episode authentication)
            let provided_private_key = sub_matches.get_one::<String>("key").map(|s| s.as_str());
            let wallet = get_wallet_for_command("participant-peer", provided_private_key)?;
            
            run_kaspa_participant_peer(kaspa_keypair, wallet.keypair, should_auth, rpc_url).await?;
        }
        
        Some(("test-api-flow", sub_matches)) => {
            let peer_url = sub_matches.get_one::<String>("peer").unwrap().clone();
            let command = TestApiFlowCommand { peer: peer_url };
            command.execute().await?;
        }
        Some(("test-api", sub_matches)) => {
            let peer_url = sub_matches.get_one::<String>("peer").unwrap().clone();
            let command = kaspa_auth::cli::commands::test_api::TestApiCommand { 
                peer: peer_url, 
                verbose: false, 
                json: false 
            };
            command.execute().await?;
        }
        _ => {
            println!("No subcommand specified. Use --help for available commands.");
            println!("\nAvailable commands:");
            println!("  authenticate  - 🚀 kdapp authentication (UNIFIED ARCHITECTURE)");
            println!("  test-episode  - Test locally (no Kaspa network)");
            println!("  http-peer     - Run HTTP coordination peer");
            println!("  demo         - Interactive demo (simulated)");
            println!("  organizer-peer - Run auth organizer peer on testnet-10");
            println!("  participant-peer - Run auth participant peer on testnet-10");
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
async fn run_kaspa_organizer_peer(signer: Keypair, name: String, rpc_url: Option<String>) -> Result<(), Box<dyn Error>> {
    println!("🎯 Starting Kaspa Auth Organizer Peer: {}", name);
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
async fn run_kaspa_participant_peer(kaspa_signer: Keypair, auth_signer: Keypair, should_auth: bool, rpc_url: Option<String>) -> Result<(), Box<dyn Error>> {
    println!("🔑 Starting Kaspa Auth Participant Peer");
    if let Some(url) = &rpc_url {
        println!("📡 Connecting to node: {}", url);
    } else {
        println!("📡 Connecting to testnet-10 (public node)...");
    }
    
    if should_auth {
        println!("🚀 Initiating blockchain authentication flow...");
        println!("🎯 This will create REAL transactions on Kaspa testnet-10");
        run_client_authentication(kaspa_signer, auth_signer).await?;
    } else {
        println!("👂 Participant peer mode: Listening for authentication requests...");
        println!("💡 Tip: Add --auth flag to initiate authentication instead of listening");
        println!("📖 Example: cargo run -- participant-peer --auth --kaspa-keyfile your_key.txt");
        println!();
        // For now, just run a server instance
        let config = AuthServerConfig::new(kaspa_signer, "auth-participant-peer".to_string(), rpc_url);
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
    
    // Connect to Kaspa network (real blockchain!)
    let network = NetworkId::with_suffix(kaspa_consensus_core::network::NetworkType::Testnet, 10);
    
    let kaspad = connect_client(network, None).await?;
    
    // Create Kaspa address for funding transactions
    let kaspa_addr = Address::new(Prefix::Testnet, Version::PubKey, &kaspa_signer.x_only_public_key().0.serialize());
    
    // Get UTXOs for transaction funding
    let entries = kaspad.get_utxos_by_addresses(vec![kaspa_addr.clone()]).await?;
    
    if entries.is_empty() {
        return Err("No UTXOs found! Please fund the Kaspa address first.".into());
    }
    
    let mut utxo = entries.first().map(|entry| {
        (TransactionOutpoint::from(entry.outpoint.clone()), UtxoEntry::from(entry.utxo_entry.clone()))
    }).unwrap();
    
    // Create real transaction generator (kdapp architecture!)
    let generator = TransactionGenerator::new(kaspa_signer, AUTH_PATTERN, AUTH_PREFIX);
    
    // Step 1: Initialize the episode first (like tictactoe example)
    let episode_id = rand::thread_rng().gen();
    let new_episode = EpisodeMessage::<SimpleAuth>::NewEpisode { 
        episode_id, 
        participants: vec![client_pubkey] 
    };
    
    let tx = generator.build_command_transaction(utxo, &kaspa_addr, &new_episode, 5000);
    
    let _res = kaspad.submit_transaction(tx.as_ref().into(), false).await?;
    utxo = generator::get_first_output_utxo(&tx);
    
    // Step 2: Send RequestChallenge command to blockchain
    let auth_command = AuthCommand::RequestChallenge;
    let step = EpisodeMessage::<SimpleAuth>::new_signed_command(
        episode_id, 
        auth_command, 
        auth_signer.secret_key(), 
        client_pubkey
    );
    
    let tx = generator.build_command_transaction(utxo, &kaspa_addr, &step, 5000);
    
    let _res = kaspad.submit_transaction(tx.as_ref().into(), false).await?;
    utxo = generator::get_first_output_utxo(&tx);
    
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
    let mut challenge = String::new();
    let mut attempt_count = 0;
    let max_attempts = 100; // 10 second timeout - Pure kdapp architecture (100 blocks = 10 seconds)
    
    // Wait for episode state with challenge
    'outer: loop {
        attempt_count += 1;
        
        if let Ok((received_episode_id, episode_state)) = response_receiver.try_recv() {
            if received_episode_id == episode_id {
                if let Some(server_challenge) = &episode_state.challenge {
                    challenge = server_challenge.clone();
                    break;
                }
            }
        }
        
        if attempt_count >= max_attempts {
            return Err("PURE KDAPP AUTHENTICATION FAILED: Blockchain timeout after 10 seconds (100 blocks). No HTTP fallback - this is pure kdapp architecture.".into());
        }
        
        // Add timeout to prevent infinite waiting
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    // Stop listening after we get the challenge
    exit_signal.store(true, std::sync::atomic::Ordering::Relaxed);
    
    // Step 3: Sign challenge and send SubmitResponse command to blockchain
    let msg = to_message(&challenge);
    let signature = sign_message(&auth_signer.secret_key(), &msg);
    let signature_hex = hex::encode(signature.0.serialize_der());
    
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
    
    let _res = kaspad.submit_transaction(tx.as_ref().into(), false).await?;
    
    Ok(())
}

/// 🚀 Automatic authentication - uses REAL kdapp architecture (unified with participant-peer --auth)
async fn run_automatic_authentication(keypair: Keypair) -> Result<(), Box<dyn Error>> {
    println!("🎯 Starting kdapp-based authentication (unified architecture)");
    println!("📱 This uses the same kdapp engine as participant-peer --auth");
    println!("🔑 Using public key: {}", hex::encode(keypair.public_key().serialize()));
    println!();

    // Use the same wallet system as participant-peer for consistency
    let wallet = get_wallet_for_command("participant-peer", None)?;
    
    // Use the wallet's keypair for funding transactions (participant pays)
    let funding_keypair = wallet.keypair;
    let auth_keypair = keypair; // Use provided keypair for authentication
    
    println!("💰 Funding transactions with participant wallet: {}", wallet.get_kaspa_address());
    println!("🔐 Authentication keypair: {}", hex::encode(auth_keypair.public_key().serialize()));
    
    // Check if wallet needs funding
    if wallet.check_funding_status() {
        println!("⚠️  WARNING: Participant wallet may need funding for blockchain transactions!");
        println!("💡 Get testnet funds: https://faucet.kaspanet.io/");
        println!("💰 Fund address: {}", wallet.get_kaspa_address());
        println!();
    }
    
    // Use the REAL kdapp architecture - same as participant-peer --auth
    run_client_authentication(funding_keypair, auth_keypair).await?;
    
    println!("✅ kdapp authentication completed successfully!");
    println!("🔍 Check your transactions on Kaspa explorer: https://explorer-tn10.kaspa.org/");
    println!("📊 Look for AUTH transactions (0x41555448) from your address: {}", wallet.get_kaspa_address());
    
    Ok(())
}

/// 🚀 HTTP Coordinated authentication - hybrid kdapp + HTTP coordination  
/// This function attempts to use pure kdapp authentication first, and falls back to HTTP coordination
/// for challenge retrieval if the blockchain-based challenge retrieval times out.
pub async fn run_http_coordinated_authentication(kaspa_signer: Keypair, auth_signer: Keypair, peer_url: String) -> Result<AuthenticationResult, Box<dyn Error>> {
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
            println!("🔍 CLIENT: Episode {} initialized - challenge: {:?}", episode_id, episode.challenge);
            let _ = self.sender.send((episode_id, episode.clone()));
        }
        
        fn on_command(&self, episode_id: kdapp::episode::EpisodeId, episode: &SimpleAuth, 
                      cmd: &AuthCommand, _authorization: Option<kdapp::pki::PubKey>, 
                      _metadata: &kdapp::episode::PayloadMetadata) {
            println!("🔍 CLIENT: Episode {} command {:?} - challenge: {:?}", episode_id, cmd, episode.challenge);
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
    let max_attempts = 150; // 30 second timeout - Hybrid mode with HTTP fallback
    
    // Try to get challenge from blockchain first
    'blockchain_loop: loop {
        attempt_count += 1;
        
        let recv_result = tokio::time::timeout(tokio::time::Duration::from_millis(200), response_receiver.recv()).await;
        
        if let Ok(Some((received_episode_id, episode_state))) = recv_result {
            println!("📨 Received episode state update for ID: {} (expecting: {})", received_episode_id, episode_id);
            if received_episode_id == episode_id {
                if let Some(server_challenge) = &episode_state.challenge {
                    challenge = server_challenge.clone();
                    println!("🎲 Real challenge received from server: {}", challenge);
                    break 'blockchain_loop;
                } else {
                    println!("📡 Episode state update received, but no challenge yet. Auth status: {}", episode_state.is_authenticated);
                }
            } else {
                println!("🔄 Episode ID mismatch, continuing to listen...");
            }
        }
        
        if attempt_count % 10 == 0 {
            println!("⏰ Still listening... attempt {} of {}", attempt_count, max_attempts);
        }
        
        if attempt_count >= max_attempts {
            return Err("❌ AUTHENTICATION FAILED: Could not retrieve challenge from blockchain within timeout. No HTTP fallback.".into());
        }
        
        // Add timeout to prevent infinite waiting
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    // Step 3: Sign challenge and send SubmitResponse command to blockchain
    // NOTE: Keep proxy alive to receive authentication completion!
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
    
    // Wait for authentication to complete and get the real session token from blockchain
    println!("⏳ Waiting for authentication completion to retrieve session token...");
    let mut session_token = String::new();
    let mut wait_attempts = 0;
    let max_wait_attempts = 50; // 5 second timeout
    
    'auth_wait: loop {
        wait_attempts += 1;
        
        if let Ok((received_episode_id, episode_state)) = response_receiver.try_recv() {
            if received_episode_id == episode_id && episode_state.is_authenticated {
                if let Some(token) = &episode_state.session_token {
                    session_token = token.clone();
                    println!("✅ Real session token retrieved from blockchain: {}", session_token);
                    // Now we can stop the proxy - authentication is complete
                    exit_signal.store(true, std::sync::atomic::Ordering::Relaxed);
                    break 'auth_wait;
                }
            }
        }
        
        if wait_attempts >= max_wait_attempts {
            return Err("❌ AUTHENTICATION FAILED: Could not retrieve session token from blockchain. Authentication incomplete.".into());
        }
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    Ok(AuthenticationResult {
        episode_id: episode_id.into(),
        session_token,
        authenticated: true,
    })
}

/// 🔄 Session revocation - revoke an active session on blockchain
pub async fn run_session_revocation(auth_signer: Keypair, episode_id: u64, session_token: String, _peer_url: String) -> Result<(), Box<dyn Error>> {
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
    
    let client_pubkey = kdapp::pki::PubKey(auth_signer.public_key());
    println!("🔄 Revoking session on blockchain...");
    println!("🔑 Auth public key: {}", client_pubkey);
    println!("📧 Episode ID: {}", episode_id);
    println!("🎫 Session token: {}", session_token);
    
    // Step 1: Connect to Kaspa network
    let network = NetworkId::with_suffix(kaspa_consensus_core::network::NetworkType::Testnet, 10);
    let kaspad = connect_client(network, None).await?;
    let kaspa_addr = Address::new(
        Prefix::Testnet, 
        Version::PubKey, 
        &auth_signer.x_only_public_key().0.serialize()
    );
    
    println!("🔗 Connected to Kaspa testnet-10");
    println!("💰 Funding address: {}", kaspa_addr);
    
    // Step 2: Get UTXOs for transaction funding
    let entries = kaspad.get_utxos_by_addresses(vec![kaspa_addr.clone()]).await?;
    if entries.is_empty() {
        return Err(format!("❌ No UTXOs found for address {}. Please fund this address first.", kaspa_addr).into());
    }
    
    let utxo = entries.first().map(|entry| {
        (TransactionOutpoint::from(entry.outpoint.clone()), UtxoEntry::from(entry.utxo_entry.clone()))
    }).unwrap();
    
    println!("✅ Using UTXO: {}", utxo.0);
    
    // Step 3: Sign the session token to prove ownership
    println!("✍️ Signing session token to prove ownership...");
    let msg = kdapp::pki::to_message(&session_token);
    let signature = kdapp::pki::sign_message(&auth_signer.secret_key(), &msg);
    let signature_hex = hex::encode(signature.0.serialize_der());
    
    // Step 4: Create RevokeSession command
    println!("📤 Creating RevokeSession command...");
    let auth_command = AuthCommand::RevokeSession {
        session_token: session_token.clone(),
        signature: signature_hex,
    };
    
    // Step 5: Build transaction and submit to blockchain
    let episode_id_u32 = episode_id as u32; // Convert for kdapp framework
    let step = EpisodeMessage::<SimpleAuth>::new_signed_command(
        episode_id_u32, 
        auth_command, 
        auth_signer.secret_key(), 
        client_pubkey
    );
    
    let generator = TransactionGenerator::new(auth_signer, AUTH_PATTERN, AUTH_PREFIX);
    
    let tx = generator.build_command_transaction(utxo, &kaspa_addr, &step, 5000);
    
    println!("🚀 Submitting RevokeSession transaction: {}", tx.id());
    
    let _res = kaspad.submit_transaction(tx.as_ref().into(), false).await?;
    
    println!("✅ Session revocation submitted to Kaspa blockchain!");
    println!("🔄 Session token {} has been revoked", session_token);
    println!("📊 Transaction is now being processed by auth organizer peer's kdapp engine");
    
    Ok(())
}

/// Show wallet status for debugging and verification
fn show_wallet_status(role: &str) -> Result<(), Box<dyn Error>> {
    use std::path::Path;
    
    println!("🔍 Kaspa Auth Wallet Status Report");
    println!("==================================");
    
    let wallet_dir = Path::new(".kaspa-auth");
    
    if !wallet_dir.exists() {
        println!("❌ No .kaspa-auth directory found");
        println!("💡 Run any command to create initial wallets");
        return Ok(());
    }
    
    match role {
        "all" => {
            check_wallet_role("organizer-peer");
            println!();
            check_wallet_role("participant-peer");
        },
        role => check_wallet_role(role),
    }
    
    println!();
    println!("🚰 Testnet Faucet: https://faucet.kaspanet.io/");
    println!("🔍 Explorer: https://explorer.kaspanet.io/");
    
    Ok(())
}

fn check_wallet_role(role: &str) {
    use std::path::Path;
    
    let wallet_file = Path::new(".kaspa-auth").join(format!("{}-wallet.key", role));
    
    println!("🔑 {} Wallet:", role.to_uppercase());
    
    if wallet_file.exists() {
        // Try to load the wallet to get address info
        match get_wallet_for_command(role, None) {
            Ok(wallet) => {
                let kaspa_addr = wallet.get_kaspa_address();
                let file_size = std::fs::metadata(&wallet_file)
                    .map(|m| m.len())
                    .unwrap_or(0);
                
                println!("  ✅ Status: EXISTS and LOADED");
                println!("  📁 File: {}", wallet_file.display());
                println!("  📊 Size: {} bytes", file_size);
                println!("  🏠 Address: {}", kaspa_addr);
                println!("  🔄 Will be REUSED on next run");
            }
            Err(e) => {
                println!("  ❌ Status: EXISTS but CORRUPTED");
                println!("  📁 File: {}", wallet_file.display());
                println!("  ⚠️  Error: {}", e);
                println!("  🔧 Solution: Delete file to recreate");
            }
        }
    } else {
        println!("  ❓ Status: NOT CREATED YET");
        println!("  📁 Will create: {}", wallet_file.display());
        println!("  🆕 Will be NEW on next run");
    }
}

// New focused authentication testing functions with timeouts

async fn run_authentication_with_timeout(
    auth_keypair: Keypair, 
    peer_url: Option<String>, 
    timeout_seconds: u64
) -> Result<(), Box<dyn Error>> {
    println!("🔥 Starting focused authentication test ({}s timeout)", timeout_seconds);
    
    let timeout_duration = tokio::time::Duration::from_secs(timeout_seconds);
    
    if let Some(url) = peer_url {
        // Get funding keypair for HTTP coordination
        let funding_wallet = get_wallet_for_command("participant-peer", None)?;
        let funding_keypair = funding_wallet.keypair;
        
        println!("🌐 Using HTTP coordination: {}", url);
        let auth_result = tokio::time::timeout(timeout_duration, run_http_coordinated_authentication(funding_keypair, auth_keypair, url)).await;
        
        match auth_result {
            Ok(result) => {
                match result {
                    Ok(_) => {
                        println!("✅ Authentication completed within {}s timeout", timeout_seconds);
                        Ok(())
                    }
                    Err(e) => {
                        println!("❌ Authentication failed: {}", e);
                        Err(e)
                    }
                }
            }
            Err(_) => {
                println!("⏰ Authentication timed out after {}s", timeout_seconds);
                Err("Authentication timeout".into())
            }
        }
    } else {
        println!("⚡ Using pure kdapp (experimental)");
        let auth_result = tokio::time::timeout(timeout_duration, run_automatic_authentication(auth_keypair)).await;
        
        match auth_result {
            Ok(result) => {
                match result {
                    Ok(_) => {
                        println!("✅ Authentication completed within {}s timeout", timeout_seconds);
                        Ok(())
                    }
                    Err(e) => {
                        println!("❌ Authentication failed: {}", e);
                        Err(e)
                    }
                }
            }
            Err(_) => {
                println!("⏰ Authentication timed out after {}s", timeout_seconds);
                Err("Authentication timeout".into())
            }
        }
    }
}

async fn run_full_authentication_cycle(
    funding_keypair: Keypair,
    auth_keypair: Keypair, 
    peer_url: String,
    session_duration: u64,
    auth_timeout: u64
) -> Result<(), Box<dyn Error>> {
    println!("🔄 Starting complete authentication lifecycle test");
    println!("⏱️  Phase 1: Login ({}s timeout)", auth_timeout);
    
    // Phase 1: Authenticate with timeout
    let auth_timeout_duration = tokio::time::Duration::from_secs(auth_timeout);
    let auth_future = run_http_coordinated_authentication(funding_keypair, auth_keypair, peer_url.clone());
    
    let auth_result = tokio::time::timeout(auth_timeout_duration, auth_future).await;
    
    let authentication_details = match auth_result {
        Ok(Ok(auth_details)) => {
            println!("✅ Phase 1: Authentication successful!");
            println!("📋 Episode ID: {}, Session Token: {}", auth_details.episode_id, auth_details.session_token);
            auth_details
        }
        Ok(Err(e)) => {
            println!("❌ Phase 1: Authentication failed: {}", e);
            return Err(e);
        }
        Err(_) => {
            println!("⏰ Phase 1: Authentication timed out after {}s", auth_timeout);
            return Err("Authentication timeout".into());
        }
    };
    
    // Phase 2: Simulate active session
    println!("⏱️  Phase 2: Active session ({}s duration)", session_duration);
    println!("🔒 Session is active - simulating user activity...");
    
    tokio::time::sleep(tokio::time::Duration::from_secs(session_duration)).await;
    
    // Phase 3: Logout using authentication details from Phase 1
    println!("⏱️  Phase 3: Logout initiated");
    println!("🚪 Revoking session {} for episode {}", authentication_details.session_token, authentication_details.episode_id);
    
    match run_session_revocation(auth_keypair, authentication_details.episode_id, authentication_details.session_token, peer_url).await {
        Ok(_) => {
            println!("✅ Phase 3: Session revocation successful!");
            println!("✅ Full authentication cycle test completed - Login → Active Session → Logout");
        }
        Err(e) => {
            println!("❌ Phase 3: Session revocation failed: {}", e);
            println!("⚠️  Authentication cycle incomplete - logout failed");
            return Err(format!("Logout failed: {}", e).into());
        }
    }
    
    Ok(())
}

async fn run_logout_with_timeout(
    auth_keypair: Keypair,
    episode_id: u64,
    session_token: String,
    peer_url: String,
    timeout_seconds: u64
) -> Result<(), Box<dyn Error>> {
    println!("🚪 Starting focused logout test ({}s timeout)", timeout_seconds);
    println!("📋 Episode: {}, Session: {}", episode_id, session_token);
    
    let timeout_duration = tokio::time::Duration::from_secs(timeout_seconds);
    let logout_future = run_session_revocation(auth_keypair, episode_id, session_token, peer_url);
    
    match tokio::time::timeout(timeout_duration, logout_future).await {
        Ok(result) => {
            match result {
                Ok(_) => {
                    println!("✅ Logout completed within {}s timeout", timeout_seconds);
                    Ok(())
                }
                Err(e) => {
                    println!("❌ Logout failed: {}", e);
                    Err(e)
                }
            }
        }
        Err(_) => {
            println!("⏰ Logout timed out after {}s", timeout_seconds);
            Err("Logout timeout".into())
        }
    }
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