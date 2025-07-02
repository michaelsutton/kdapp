use clap::{Arg, Command};
use env_logger;
use std::error::Error;

mod simple_auth_episode;

use kdapp::pki::{generate_keypair, sign_message, to_message};
use kdapp::episode::{PayloadMetadata, Episode};
use simple_auth_episode::{SimpleAuth, AuthCommand};

fn main() -> Result<(), Box<dyn Error>> {
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
        _ => {
            println!("No subcommand specified. Use --help for available commands.");
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
            signature: signature.0.serialize_der().to_vec(),
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
            signature: signature.0.serialize_der().to_vec(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episode_creation() {
        let result = test_episode_logic(2);
        assert!(result.is_ok());
    }
}