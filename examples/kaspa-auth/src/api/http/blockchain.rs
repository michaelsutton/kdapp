// src/api/http/blockchain.rs
use kaspa_consensus_core::{network::{NetworkId, NetworkType}, tx::{TransactionOutpoint, UtxoEntry}};
use kaspa_addresses::{Address, Prefix, Version};
use kaspa_rpc_core::api::rpc::RpcApi;
use kdapp::engine::EpisodeMessage;
use crate::core::{episode::SimpleAuth, commands::AuthCommand};

pub struct TxSubmitter<'a> {
    pub server_keypair: &'a secp256k1::Keypair,
    pub transaction_generator: &'a kdapp::generator::TransactionGenerator,
}

impl<'a> TxSubmitter<'a> {
    pub async fn submit_auth(
        &self,
        episode_id: u64,
        signature: String,
        nonce: String,
        client_pubkey: kdapp::pki::PubKey,
    ) -> Result<String, String> {
        // Create command
        let cmd = AuthCommand::SubmitResponse { signature, nonce };
        let msg = EpisodeMessage::<SimpleAuth>::new_signed_command(
            episode_id as u32, cmd, self.server_keypair.secret_key(), client_pubkey
        );
        
        // Connect to Kaspa
        let network = NetworkId::with_suffix(NetworkType::Testnet, 10);
        let kaspad = kdapp::proxy::connect_client(network, None).await
            .map_err(|e| format!("Connect failed: {}", e))?;
        
        // Get server address and UTXOs
        let addr = Address::new(Prefix::Testnet, Version::PubKey, 
            &self.server_keypair.public_key().serialize()[1..]);
        
        let entries = kaspad.get_utxos_by_addresses(vec![addr.clone()]).await
            .map_err(|e| format!("UTXO fetch failed: {}", e))?;
        
        if entries.is_empty() {
            return Err(format!("No UTXOs! Fund: {}", addr));
        }
        
        // Build and submit transaction
        let utxo = (TransactionOutpoint::from(entries[0].outpoint.clone()),
                    UtxoEntry::from(entries[0].utxo_entry.clone()));
        
        let tx = self.transaction_generator.build_command_transaction(
            utxo, &addr, &msg, 5000
        );
        
        kaspad.submit_transaction(tx.as_ref().into(), false).await
            .map_err(|e| format!("Submit failed: {}", e))?;
        
        Ok(tx.id().to_string())
    }
}