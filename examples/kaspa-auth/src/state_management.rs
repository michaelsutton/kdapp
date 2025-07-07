use borsh::{BorshDeserialize, BorshSerialize};
use kdapp::episode::{Episode, PayloadMetadata};
use kaspa_hashes::Hash;

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct StateSnapshot<T: Episode> {
    pub episode_state: T,
    pub timestamp: u64,
    pub block_height: u64,
    pub merkle_root: Hash,
}

pub trait SnapshotCapable: Episode {
    fn create_snapshot(&self, metadata: &PayloadMetadata) -> StateSnapshot<Self>
    where
        Self: Sized + Clone;
    
    fn verify_snapshot(&self, snapshot: &StateSnapshot<Self>) -> bool
    where
        Self: Sized;
}
