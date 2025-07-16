use std::error::Error;
use secp256k1::Keypair;
use crate::core::{commands::AuthCommand, episode::SimpleAuth};
use hex;

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
    use crate::episode_runner::{AUTH_PATTERN, AUTH_PREFIX};
    
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
    println!("🔗 [ VERIFY ON KASPA EXPLORER → ] https://explorer-tn10.kaspa.org/txs/{}", tx.id());
    println!("🔗 [ VIEW WALLET ON EXPLORER → ] https://explorer-tn10.kaspa.org/addresses/{}", kaspa_addr);
    println!("🔄 Session token {} has been revoked", session_token);
    println!("📊 Transaction submitted to Kaspa blockchain - organizer peer will detect and respond");
    
    Ok(())
}