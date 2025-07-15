// src/cli/commands/submit_comment.rs
use std::error::Error;
use kaspa_wrpc_client::prelude::*;
use kaspa_consensus_core::tx::UtxoEntry;
use kaspa_consensus_core::tx::TransactionOutpoint;
use kaspa_consensus_core::network::{NetworkId, NetworkType};
use kdapp::engine::EpisodeMessage;
use kdapp::generator::TransactionGenerator;
use kdapp::proxy;
use kdapp::pki::PubKey;
use log::info;

use crate::wallet::get_wallet_for_command;
use crate::comment::{CommentCommand, CommentEpisode};
use crate::episode_runner::{COMMENT_PATTERN, COMMENT_PREFIX};

pub async fn submit_comment_to_episode(
    episode_id: u64,
    comment_text: String,
    session_token: String,
    _kaspa_address: Option<&str>,
    private_key: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    // Load participant wallet
    let wallet = get_wallet_for_command("submit-comment", private_key)?;
    let addr = wallet.get_kaspa_address();
    info!("🔑 Using wallet address: {}", addr);
    
    // Validate comment
    if comment_text.trim().is_empty() {
        return Err("Comment cannot be empty".into());
    }
    
    if comment_text.len() > 2000 {
        return Err("Comment too long (max 2000 characters)".into());
    }
    
    // Connect to Kaspa network
    let network = NetworkId::with_suffix(NetworkType::Testnet, 10);
    let kaspad = proxy::connect_client(network, None).await?;
    let generator = TransactionGenerator::new(wallet.keypair, COMMENT_PATTERN, COMMENT_PREFIX);
    
    // Get UTXO for transaction
    let entries = kaspad.get_utxos_by_addresses(vec![addr.clone()]).await?;
    if entries.is_empty() {
        return Err("No UTXOs found. Wallet needs funding.".into());
    }
    
    let utxo = (
        TransactionOutpoint::from(entries[0].outpoint.clone()),
        UtxoEntry::from(entries[0].utxo_entry.clone())
    );
    
    // Create comment command
    let comment_cmd = CommentCommand::SubmitComment {
        text: comment_text.clone(),
        author: format!("{}", wallet.keypair.public_key()),
        session_token: session_token.clone(),
        signature: "TODO_SIGN_COMMENT".to_string(), // TODO: Implement proper signature
    };
    
    // Create episode message (like other authentication commands)
    let step = EpisodeMessage::<CommentEpisode>::new_signed_command(
        episode_id as u32,
        comment_cmd,
        wallet.keypair.secret_key(),
        PubKey(wallet.keypair.public_key()),
    );
    
    // Build transaction (like other authentication commands)
    let tx = generator.build_command_transaction(utxo, &addr, &step, 5000);
    
    // Submit to blockchain
    let _res = kaspad.submit_transaction(tx.as_ref().into(), false).await?;
    
    // Get transaction ID for explorer link
    let tx_id = tx.id();
    
    println!("✅ Comment submitted to episode {} on blockchain!", episode_id);
    println!("💬 Comment: \"{}\"", comment_text);
    println!("🎯 Real kdapp architecture: P2P comment via blockchain transaction");
    println!("📋 Transaction ID: {}", tx_id);
    println!("🔗 [ VERIFY ON KASPA EXPLORER → ] https://explorer-tn10.kaspa.org/txs/{}", tx_id);
    println!("🔗 [ VIEW WALLET ON EXPLORER → ] https://explorer-tn10.kaspa.org/addresses/{}", addr);
    
    Ok(())
}

// CLI command for submitting comments
pub async fn run_submit_comment_command(
    episode_id: u64,
    comment_text: String,
    session_token: String,
    kaspa_address: Option<&str>,
    private_key: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    println!("💬 COMMENT SUBMISSION TO EPISODE {}", episode_id);
    println!("📝 Comment: \"{}\"", comment_text);
    println!("🎫 Session: {}", session_token);
    println!("🔐 Using kdapp P2P architecture (not HTTP server)");
    println!("");
    
    submit_comment_to_episode(
        episode_id,
        comment_text,
        session_token,
        kaspa_address,
        private_key,
    ).await?;
    
    Ok(())
}