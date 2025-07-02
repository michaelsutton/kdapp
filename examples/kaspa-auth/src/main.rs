use clap::{Arg, Command};
use env_logger;
use std::error::Error;
use secp256k1::{Secp256k1, SecretKey, Keypair};
use log::info;

mod simple_auth_episode;
mod auth_commands;
mod episode_runner;

use kdapp::pki::{generate_keypair, sign_message, to_message};
use kdapp::episode::{PayloadMetadata, Episode};
use simple_auth_episode::SimpleAuth;
use auth_commands::AuthCommand;
use episode_runner::{AuthServerConfig, run_auth_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

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
        Some(("demo", _)) => {
            run_interactive_demo()?;
        }
        Some(("server", sub_matches)) => {
            let name = sub_matches.get_one::<String>("name").unwrap().clone();
            let keypair = if let Some(key_hex) = sub_matches.get_one::<String>("key") {
                parse_private_key(key_hex)?
            } else {
                generate_random_keypair()
            };
            
            info!("🔑 Server public key: {}", hex::encode(keypair.public_key().serialize()));
            run_kaspa_server(keypair, name).await?;
        }
        Some(("client", sub_matches)) => {
            let should_auth = sub_matches.get_flag("auth");
            let keypair = if let Some(key_hex) = sub_matches.get_one::<String>("key") {
                parse_private_key(key_hex)?
            } else {
                generate_random_keypair()
            };
            
            info!("🔑 Client public key: {}", hex::encode(keypair.public_key().serialize()));
            run_kaspa_client(keypair, should_auth).await?;
        }
        _ => {
            println!("No subcommand specified. Use --help for available commands.");
            println!("\nAvailable commands:");
            println!("  test-episode  - Test locally (no Kaspa network)");
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

/// Run Kaspa authentication server
async fn run_kaspa_server(signer: Keypair, name: String) -> Result<(), Box<dyn Error>> {
    println!("🎯 Starting Kaspa Auth Server: {}", name);
    println!("📡 Connecting to testnet-10...");
    
    let config = AuthServerConfig::new_testnet10(signer, name);
    run_auth_server(config).await?;
    
    Ok(())
}

/// Run Kaspa authentication client
async fn run_kaspa_client(signer: Keypair, should_auth: bool) -> Result<(), Box<dyn Error>> {
    println!("🔑 Starting Kaspa Auth Client");
    println!("📡 Connecting to testnet-10...");
    
    if should_auth {
        println!("🚀 Initiating authentication flow...");
        // TODO: Implement client authentication flow
        todo!("Client authentication flow not yet implemented");
    } else {
        println!("👂 Listening for authentication requests...");
        // For now, just run a server instance
        let config = AuthServerConfig::new_testnet10(signer, "auth-client".to_string());
        run_auth_server(config).await?;
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