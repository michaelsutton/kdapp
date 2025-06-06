//! Contains methods for creating a Kaspa wrpc client as well as listener logic for following
//! accepted txs by id pattern and prefix and sending them to corresponding engines.

use kaspa_consensus_core::{network::NetworkId, Hash};
use kaspa_rpc_core::api::rpc::RpcApi;
use kaspa_rpc_core::RpcNetworkType;
use kaspa_wrpc_client::client::ConnectOptions;
use kaspa_wrpc_client::error::Error;
use kaspa_wrpc_client::prelude::*;
use kaspa_wrpc_client::{KaspaRpcClient, WrpcEncoding};

use log::{debug, info, warn};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
    Arc,
};
use std::time::Duration;
use tokio::time::{sleep_until, Instant};

use crate::generator::{PatternType, PrefixType};
use crate::{
    engine::EngineMsg as Msg,
    generator::{check_pattern, Payload},
};

fn connect_options() -> ConnectOptions {
    ConnectOptions {
        block_async_connect: true,
        strategy: ConnectStrategy::Fallback,
        url: None,
        connect_timeout: Some(Duration::from_secs(5)),
        retry_interval: None,
    }
}

// Copied from https://github.com/supertypo/simply-kaspa-indexer/blob/main/kaspad/src/pool/manager.rs
pub async fn connect_client(network_id: NetworkId, rpc_url: Option<String>) -> Result<KaspaRpcClient, Error> {
    let url = if let Some(url) = &rpc_url { url } else { &Resolver::default().get_url(WrpcEncoding::Borsh, network_id).await? };

    debug!("Connecting to Kaspad {}", url);
    let client = KaspaRpcClient::new_with_args(WrpcEncoding::Borsh, Some(url), None, Some(network_id), None)?;
    client.connect(Some(connect_options())).await.map_err(|e| {
        warn!("Kaspad connection failed: {e}");
        e
    })?;

    let server_info = client.get_server_info().await?;
    let connected_network = format!(
        "{}{}",
        server_info.network_id.network_type,
        server_info.network_id.suffix.map(|s| format!("-{}", s)).unwrap_or_default()
    );
    info!("Connected to Kaspad {}, version: {}, network: {}", url, server_info.server_version, connected_network);

    if network_id != server_info.network_id {
        panic!("Network mismatch, expected '{}', actual '{}'", network_id, connected_network);
    } else if !server_info.is_synced
        || server_info.network_id.network_type == RpcNetworkType::Mainnet && server_info.virtual_daa_score < 107107107
    {
        let err_msg = format!("Kaspad {} is NOT synced", server_info.server_version);
        warn!("{err_msg}");
        Err(Error::Custom(err_msg))
    } else {
        Ok(client)
    }
}

pub type EngineMap = HashMap<PrefixType, (PatternType, Sender<Msg>)>;

pub async fn run_listener(kaspad: KaspaRpcClient, engines: EngineMap, exit_signal: Arc<AtomicBool>) {
    let info = kaspad.get_block_dag_info().await.unwrap();
    let mut sink = info.sink;
    let mut now = Instant::now();
    info!("Sink: {}", sink);
    loop {
        if exit_signal.load(Ordering::Relaxed) {
            info!("Exiting...");
            break;
        }
        sleep_until(now + Duration::from_secs(1)).await;
        now = Instant::now();

        let vcb = kaspad.get_virtual_chain_from_block(sink, true).await.unwrap();

        debug!("vspc: {}, {}", vcb.removed_chain_block_hashes.len(), vcb.accepted_transaction_ids.len());

        if let Some(new_sink) = vcb.accepted_transaction_ids.last().map(|ncb| ncb.accepting_block_hash) {
            sink = new_sink;
        } else {
            // No new added chain blocks. This means no removed chain blocks as well so we can continue
            continue;
        }

        for rcb in vcb.removed_chain_block_hashes {
            for (_, sender) in engines.values() {
                let msg = Msg::BlkReverted { accepting_hash: rcb };
                sender.send(msg).unwrap();
            }
        }

        // Iterate new chain blocks
        for ncb in vcb.accepted_transaction_ids {
            let accepting_hash = ncb.accepting_block_hash;

            // Required txs kept in original acceptance order. Skip the first which is always a coinbase tx
            let required_txs: Vec<Hash> = ncb
                .accepted_transaction_ids
                .iter()
                .copied()
                .skip(1)
                .filter(|&id| engines.values().any(|(pattern, _)| check_pattern(id, pattern)))
                .collect();

            // Track the required payloads
            let mut required_payloads: HashMap<Hash, Option<Vec<u8>>> = required_txs.iter().map(|&id| (id, None)).collect();
            let mut required_num = required_payloads.len();

            if required_num == 0 {
                continue;
            }

            let accepting_block = kaspad.get_block(accepting_hash, false).await.unwrap(); // no need for txs of this block itself
            let verbose = accepting_block.verbose_data.unwrap();
            assert_eq!(verbose.selected_parent_hash, verbose.merge_set_blues_hashes[0]);
            debug!(
                "accepting block: {}, selected parent: {}, mergeset len: {}",
                accepting_hash,
                verbose.selected_parent_hash,
                verbose.merge_set_blues_hashes.len() + verbose.merge_set_reds_hashes.len()
            );

            // Iterate over merged blocks until finding all accepted and required txs (the mergeset is guaranteed to contain these txs)
            'outer: for merged_hash in verbose.merge_set_blues_hashes.into_iter().chain(verbose.merge_set_reds_hashes) {
                let merged_block = kaspad.get_block(merged_hash, true).await.unwrap();
                for tx in merged_block.transactions.into_iter().skip(1) {
                    if let Some(required_payload) = required_payloads.get_mut(&tx.verbose_data.unwrap().transaction_id) {
                        if required_payload.is_none() {
                            required_payload.replace(tx.payload);
                            required_num -= 1;
                            if required_num == 0 {
                                break 'outer;
                            }
                        }
                    }
                }
            }
            assert_eq!(0, required_num, "kaspad is misbehaving");
            // info!("Tx payloads: {:?}", required_payloads);

            let mut consumed_txs = 0;
            // Iterate over all engines and look for id pattern + prefix
            for (&prefix, (pattern, sender)) in engines.iter() {
                // Collect and strip payloads in the correct order (as maintained by required_txs)
                let associated_txs: Vec<_> = required_txs
                    .iter()
                    .filter_map(|&id| {
                        // First, check the pattern
                        if !check_pattern(id, pattern) {
                            return None;
                        }
                        match required_payloads.entry(id) {
                            Entry::Occupied(entry) => {
                                // The prefix is unique per engine, so once we find a match we can consume the entry
                                if Payload::check_header(entry.get().as_ref().unwrap(), prefix) {
                                    let payload = entry.remove().unwrap();
                                    consumed_txs += 1;
                                    return Some((id, Payload::strip_header(payload)));
                                }
                            }
                            Entry::Vacant(_) => {}
                        }
                        None
                    })
                    .collect();
                for (tx_id, _payload) in associated_txs.iter() {
                    info!("received episode tx: {}", tx_id);
                }
                if !associated_txs.is_empty() {
                    let msg = Msg::BlkAccepted {
                        accepting_hash,
                        accepting_daa: accepting_block.header.daa_score,
                        accepting_time: accepting_block.header.timestamp,
                        associated_txs,
                    };
                    sender.send(msg).unwrap();
                }
                if consumed_txs == required_txs.len() {
                    // No need to check additional engines
                    break;
                }
            }
        }
    }

    for (_, sender) in engines.values() {
        sender.send(Msg::Exit).unwrap();
    }
}
