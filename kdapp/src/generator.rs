//! Contains methods and helper structures for generating Kaspa transactions for carrying commands as payloads.
//! The generation process increments an internal payload nonce until the tx id matches a predefined pattern.
//! This significantly reduces the overhead of tracking txs through the node, since only txs following the pattern
//! need to be obtained from the Kaspa node.

use itertools::Itertools;
use kaspa_addresses::Address;
use kaspa_consensus_core::{
    constants::TX_VERSION,
    sign::sign,
    subnets::SUBNETWORK_ID_NATIVE,
    tx::{MutableTransaction, Transaction, TransactionInput, TransactionOutpoint, TransactionOutput, UtxoEntry},
    Hash,
};
use kaspa_txscript::pay_to_address_script;
use log::debug;
use secp256k1::Keypair;

use crate::{engine::EpisodeMessage, episode::Episode};

pub type PatternType = [(u8, u8); 10];
pub type PrefixType = u32;

pub fn check_pattern(tx_id: Hash, pattern: &PatternType) -> bool {
    let words = tx_id.as_bytes();
    for (pos, val) in pattern.iter().copied() {
        let word = words[pos as usize / 8];
        if ((word >> (pos % 8)) & 1) != val {
            return false;
        }
    }
    true
}

pub struct Payload;

impl Payload {
    pub fn pack_header(inner_data: Vec<u8>, prefix: PrefixType) -> Vec<u8> {
        // 4 byte prefix | 4 byte nonce | inner data
        prefix.to_le_bytes().into_iter().chain(0u32.to_le_bytes()).chain(inner_data).collect()
    }

    pub fn check_header(payload: &[u8], prefix: PrefixType) -> bool {
        if payload.len() < 8 {
            return false;
        }
        payload[0..4] == prefix.to_le_bytes()
    }

    pub fn set_nonce(data: &mut [u8], nonce: u32) {
        data[4..8].copy_from_slice(&nonce.to_le_bytes());
    }

    /// Strips the payload header. Assumes check_header was called and returned true
    pub fn strip_header(mut payload: Vec<u8>) -> Vec<u8> {
        payload.drain(0..8);
        payload
    }
}

pub struct TransactionGenerator {
    signer: Keypair,
    pattern: PatternType,
    prefix: PrefixType,
}

impl TransactionGenerator {
    pub fn new(signer: Keypair, pattern: PatternType, prefix: PrefixType) -> Self {
        Self { signer, pattern, prefix }
    }

    pub fn build_transaction(
        &self,
        utxos: &[(TransactionOutpoint, UtxoEntry)],
        send_amount: u64,
        num_outs: u64,
        recipient: &Address,
        payload: Vec<u8>,
    ) -> Transaction {
        let script_public_key = pay_to_address_script(recipient);
        let inputs = utxos
            .iter()
            .map(|(op, _)| TransactionInput { previous_outpoint: *op, signature_script: vec![], sequence: 0, sig_op_count: 1 })
            .collect_vec();

        let outputs = (0..num_outs)
            .map(|_| TransactionOutput { value: send_amount / num_outs, script_public_key: script_public_key.clone() })
            .collect_vec();
        let payload = Payload::pack_header(payload, self.prefix);
        let mut nonce = 0u32;
        let mut unsigned_tx = Transaction::new_non_finalized(TX_VERSION, inputs, outputs, 0, SUBNETWORK_ID_NATIVE, 0, payload);
        unsigned_tx.finalize();
        while !check_pattern(unsigned_tx.id(), &self.pattern) {
            nonce = nonce.checked_add(1).unwrap(); // We expect this to never overflow for a 10-bit pattern
            Payload::set_nonce(&mut unsigned_tx.payload, nonce);
            unsigned_tx.finalize();
            debug!("nonce: {}, id: {}", nonce, unsigned_tx.id());
        }
        let signed_tx = sign(
            MutableTransaction::with_entries(unsigned_tx, utxos.iter().map(|(_, entry)| entry.clone()).collect_vec()),
            self.signer,
        );
        signed_tx.tx
    }

    pub fn build_command_transaction<G: Episode>(
        &self,
        utxo: (TransactionOutpoint, UtxoEntry),
        recipient: &Address,
        cmd: &EpisodeMessage<G>,
        fee: u64,
    ) -> Transaction {
        let payload = borsh::to_vec(&cmd).unwrap();
        let send = utxo.1.amount - fee;
        self.build_transaction(&[utxo], send, 1, recipient, payload)
    }
}

pub fn get_first_output_utxo(tx: &Transaction) -> (TransactionOutpoint, UtxoEntry) {
    (TransactionOutpoint::new(tx.id(), 0), UtxoEntry::new(tx.outputs[0].value, tx.outputs[0].script_public_key.clone(), 0, false))
}
