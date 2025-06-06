//! This module handles the logic of running and maintaining several episodes of the same type
//! including keeping a stack of rollback objects per episode in order to support DAG reorg handling

use borsh::{BorshDeserialize, BorshSerialize};
use kaspa_consensus_core::Hash;
use log::*;
use secp256k1::SecretKey;

use crate::episode::{Episode, EpisodeError, EpisodeEventHandler, EpisodeId, PayloadMetadata};
use crate::pki::{sign_message, to_message, verify_signature, PubKey, Sig};
use std::any::type_name;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::mpsc::Receiver;

const EPISODE_LIFETIME: u64 = 2592000; // Three days
const SAMPLE_REMOVAL_TIME: u64 = 432000; // Half a day

pub(crate) struct EpisodeWrapper<G: Episode> {
    pub episode: G,
    pub rollback_stack: Vec<G::CommandRollback>,
}

#[derive(Default)]
pub struct DefaultEventHandler;

impl<G: Episode> EpisodeEventHandler<G> for DefaultEventHandler {
    fn on_initialize(&self, _episode_id: EpisodeId, _episode: &G) {}

    fn on_command(
        &self,
        _episode_id: EpisodeId,
        _episode: &G,
        _cmd: &<G as Episode>::Command,
        _authorization: Option<PubKey>,
        _metadata: &PayloadMetadata,
    ) {
    }

    fn on_rollback(&self, _episode_id: EpisodeId, _episode: &G) {}
}

/// The main entry point for running episodes of a given Episode type.
pub struct Engine<G: Episode, P: EpisodeEventHandler<G> = DefaultEventHandler> {
    pub(crate) episodes: HashMap<EpisodeId, EpisodeWrapper<G>>,
    pub(crate) revert_map: HashMap<Hash, Vec<(EpisodeId, PayloadMetadata)>>,
    pub(crate) receiver: Receiver<EngineMsg>,
    pub(crate) next_filtering: u64,
    pub(crate) episode_creation_times: HashMap<EpisodeId, u64>,

    _phantom: PhantomData<P>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub enum EpisodeMessage<G: Episode> {
    NewEpisode { episode_id: EpisodeId, participants: Vec<PubKey> },
    SignedCommand { episode_id: EpisodeId, cmd: G::Command, pubkey: PubKey, sig: Sig },
    UnsignedCommand { episode_id: EpisodeId, cmd: G::Command },
    Revert { episode_id: EpisodeId },
}

impl<G: Episode> EpisodeMessage<G> {
    pub fn new_signed_command(episode_id: EpisodeId, cmd: G::Command, sk: SecretKey, pk: PubKey) -> Self {
        let msg = to_message(&cmd);
        let sig = sign_message(&sk, &msg);
        Self::SignedCommand { episode_id, cmd, pubkey: pk, sig }
    }

    pub fn episode_id(&self) -> EpisodeId {
        match self {
            EpisodeMessage::NewEpisode { episode_id, .. } => *episode_id,
            EpisodeMessage::SignedCommand { episode_id, .. } => *episode_id,
            EpisodeMessage::UnsignedCommand { episode_id, .. } => *episode_id,
            EpisodeMessage::Revert { episode_id } => *episode_id,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub enum EngineMsg {
    BlkAccepted { accepting_hash: Hash, accepting_daa: u64, accepting_time: u64, associated_txs: Vec<(Hash, Vec<u8>)> },
    BlkReverted { accepting_hash: Hash },
    Exit,
}

impl<G: Episode> EpisodeWrapper<G> {
    pub fn initialize(participants: Vec<PubKey>, metadata: &PayloadMetadata) -> Self {
        let episode = G::initialize(participants, metadata);
        let rollback_stack = vec![];
        EpisodeWrapper { episode, rollback_stack }
    }

    pub fn execute_signed(
        &mut self,
        cmd: &G::Command,
        pubkey: PubKey,
        sig: Sig,
        metadata: &PayloadMetadata,
    ) -> Result<(), EpisodeError<G::CommandError>> {
        if !self::verify_signature(&pubkey, &self::to_message(&cmd), &sig) {
            return Err(EpisodeError::InvalidSignature);
        }
        let rollback = G::execute(&mut self.episode, cmd, Some(pubkey), metadata)?;
        self.rollback_stack.push(rollback);
        Ok(())
    }

    pub fn execute_unsigned(&mut self, cmd: &G::Command, metadata: &PayloadMetadata) -> Result<(), EpisodeError<G::CommandError>> {
        let rollback = G::execute(&mut self.episode, cmd, None, metadata)?;
        self.rollback_stack.push(rollback);
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<(), EpisodeError<G::CommandError>> {
        if let Some(rollback) = self.rollback_stack.pop() {
            let res = self.episode.rollback(rollback);
            if !res {
                error!(
                    "Episode rollback for type {} was unsuccessful (indicates a severe bug in episode impl or engine code)",
                    type_name::<G>()
                );
            }
            Ok(())
        } else {
            // Stack is empty, hint for episode deletion
            Err(EpisodeError::DeleteEpisode)
        }
    }
}

impl<G: Episode, H: EpisodeEventHandler<G>> Engine<G, H> {
    pub fn new(receiver: Receiver<EngineMsg>) -> Self {
        let episodes: HashMap<EpisodeId, EpisodeWrapper<G>> = HashMap::new();
        let episode_creation_times: HashMap<EpisodeId, u64> = HashMap::new();
        let revert_map: HashMap<Hash, Vec<(EpisodeId, PayloadMetadata)>> = HashMap::new();
        let next_filtering: u64 = 0;
        Self { episodes, revert_map, episode_creation_times, receiver, next_filtering, _phantom: Default::default() }
    }

    pub fn start(&mut self, handlers: Vec<H>) {
        while let Ok(msg) = self.receiver.recv() {
            match msg {
                EngineMsg::BlkAccepted { accepting_hash, accepting_daa, accepting_time, associated_txs } => {
                    self.filter_old_episodes(accepting_daa);
                    let mut revert_vec: Vec<(EpisodeId, PayloadMetadata)> = vec![];
                    for (tx_id, payload) in associated_txs {
                        let episode_action: EpisodeMessage<G> = match borsh::from_slice(&payload) {
                            Ok(EpisodeMessage::Revert { episode_id }) => {
                                warn!("Episode: {}. Illegal revert attempted. Ignoring.", episode_id);
                                continue;
                            }
                            Ok(episode_action) => episode_action,
                            Err(err) => {
                                warn!("Payload: {:?} rejected. Parsing error: {}", payload, err);
                                continue;
                            }
                        };
                        let metadata = PayloadMetadata { accepting_hash, accepting_daa, accepting_time, tx_id };
                        if let Some(revert_id) = self.handle_message(episode_action, &metadata, &handlers) {
                            revert_vec.push(revert_id);
                        }
                    }
                    self.revert_map.insert(accepting_hash, revert_vec);
                }
                EngineMsg::BlkReverted { accepting_hash } => match self.revert_map.entry(accepting_hash) {
                    Entry::Occupied(entry) => {
                        for reversion in entry.remove().into_iter().rev() {
                            let episode_action: EpisodeMessage<G> = EpisodeMessage::Revert { episode_id: reversion.0 };
                            let metadata = PayloadMetadata {
                                accepting_hash,
                                accepting_daa: reversion.1.accepting_daa,
                                accepting_time: reversion.1.accepting_time,
                                tx_id: reversion.1.tx_id,
                            };
                            assert_eq!(self.handle_message(episode_action, &metadata, &handlers), None);
                        }
                    }
                    Entry::Vacant(_) => {}
                },
                EngineMsg::Exit => break,
            }
        }
    }

    pub fn filter_old_episodes(&mut self, daa_score: u64) {
        if daa_score > self.next_filtering + SAMPLE_REMOVAL_TIME {
            let mut remove_ids = vec![];
            for (episode_id, creation_time) in self.episode_creation_times.iter() {
                if creation_time < &daa_score.saturating_sub(EPISODE_LIFETIME) {
                    remove_ids.push(*episode_id);
                }
            }
            for episode_id in remove_ids {
                self.episodes.remove_entry(&episode_id);
                self.episode_creation_times.remove_entry(&episode_id);
            }
            self.next_filtering = daa_score;
        }
    }

    pub fn handle_message(
        &mut self,
        episode_action: EpisodeMessage<G>,
        metadata: &PayloadMetadata,
        handlers: &[H],
    ) -> Option<(EpisodeId, PayloadMetadata)> {
        match episode_action {
            EpisodeMessage::NewEpisode { episode_id, participants } => {
                if self.episodes.contains_key(&episode_id) {
                    warn!("Episode with id {} already exists", episode_id);
                    return None;
                }
                let ew = EpisodeWrapper::<G>::initialize(participants, metadata);
                for handler in handlers.iter() {
                    handler.on_initialize(episode_id, &ew.episode);
                }
                self.episodes.insert(episode_id, ew);
                debug!("Episode {} created.", episode_id);
                self.episode_creation_times.insert(episode_id, metadata.accepting_daa);

                return Some((episode_id, metadata.clone()));
            }

            EpisodeMessage::SignedCommand { episode_id, cmd, pubkey, sig } => {
                if let Some(wrapper) = self.episodes.get_mut(&episode_id) {
                    match wrapper.execute_signed(&cmd, pubkey, sig, metadata) {
                        Ok(()) => {
                            for handler in handlers.iter() {
                                handler.on_command(episode_id, &wrapper.episode, &cmd, Some(pubkey), metadata);
                            }
                            return Some((episode_id, metadata.clone()));
                        }
                        Err(e) => {
                            warn!("Episode {}: Command {:?} rejected: {}", episode_id, cmd, e)
                        }
                    }
                } else {
                    warn!("Episode {} not found.", episode_id);
                }
            }

            EpisodeMessage::UnsignedCommand { episode_id, cmd } => {
                if let Some(wrapper) = self.episodes.get_mut(&episode_id) {
                    match wrapper.execute_unsigned(&cmd, metadata) {
                        Ok(()) => {
                            for handler in handlers.iter() {
                                handler.on_command(episode_id, &wrapper.episode, &cmd, None, metadata);
                            }
                            return Some((episode_id, metadata.clone()));
                        }
                        Err(e) => {
                            warn!("Episode {}: Command {:?} rejected: {}", episode_id, cmd, e)
                        }
                    }
                } else {
                    warn!("Episode {} not found.", episode_id);
                }
            }

            EpisodeMessage::Revert { episode_id } => {
                if let Some(wrapper) = self.episodes.get_mut(&episode_id) {
                    info!("Episode {}: Reverting command: {:?}", episode_id, metadata.tx_id);
                    let rollback_result = wrapper.rollback();
                    for handler in handlers.iter() {
                        handler.on_rollback(episode_id, &wrapper.episode);
                    }
                    if let Err(EpisodeError::DeleteEpisode) = rollback_result {
                        // A revert of the creation
                        self.episodes.remove_entry(&episode_id);
                        self.episode_creation_times.remove_entry(&episode_id);
                    }
                } else {
                    warn!("Episode {} not found.", episode_id);
                }
                return None;
            }
        }
        None
    }
}
