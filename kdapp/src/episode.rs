//! Defines the external injection points an Episode developer would need to implement

use crate::pki::PubKey;
use borsh::{BorshDeserialize, BorshSerialize};
use kaspa_consensus_core::Hash;
use std::error::Error;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum EpisodeError<E: Error + 'static> {
    #[error("participant is not authorized in this episode.")]
    Unauthorized,

    #[error("signature verification failed.")]
    InvalidSignature,

    #[error("invalid command: {0}")]
    InvalidCommand(E),

    #[error("episode no longer valid.")]
    DeleteEpisode,
}

#[derive(Clone, PartialEq, Debug, BorshSerialize, BorshDeserialize)]
pub struct PayloadMetadata {
    pub accepting_hash: Hash,
    pub accepting_daa: u64,
    pub accepting_time: u64,
    pub tx_id: Hash,
}

pub type EpisodeId = u32;

pub trait Episode {
    type Command: BorshSerialize + BorshDeserialize + Debug + Clone;
    type CommandRollback: BorshSerialize + BorshDeserialize;
    type CommandError: Error + 'static;

    /// Initialize the episode, possibly providing a set of authorized pubkey participants
    fn initialize(participants: Vec<PubKey>, metadata: &PayloadMetadata) -> Self;

    /// Execute a command advancing the state of the episode, possibly attaching the already verified
    /// authorized pubkey requesting this execution. Returns a rollback object which can be used later
    /// to rollback from the currently obtained state back to the state prior to this call.
    fn execute(
        &mut self,
        cmd: &Self::Command,
        authorization: Option<PubKey>,
        metadata: &PayloadMetadata,
    ) -> Result<Self::CommandRollback, EpisodeError<Self::CommandError>>;

    /// Rollback a previous execute op
    fn rollback(&mut self, rollback: Self::CommandRollback) -> bool;
}

pub trait EpisodeEventHandler<G: Episode> {
    /// Called by the engine on episode initialization
    fn on_initialize(&self, episode_id: EpisodeId, episode: &G);

    /// Called by the engine following a successful command execution
    fn on_command(
        &self,
        episode_id: EpisodeId,
        episode: &G,
        cmd: &G::Command,
        authorization: Option<PubKey>,
        metadata: &PayloadMetadata,
    );

    /// Called by the engine following a command rollback
    fn on_rollback(&self, episode_id: EpisodeId, episode: &G);
}
