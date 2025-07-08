use borsh::{BorshDeserialize, BorshSerialize};
use kaspa_hashes::Hash;

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Commitment {
    pub hash: Hash,
    pub reveal_after: u64,
    pub revealed_value: Option<Vec<u8>>,
}
