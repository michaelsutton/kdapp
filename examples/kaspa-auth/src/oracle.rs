use borsh::{BorshDeserialize, BorshSerialize};
use kdapp::pki::PubKey;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Oracle {
    pub pubkey: PubKey,
    pub reputation: u64,
    pub specialization: OracleType,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum OracleType {
    RandomnessProvider,
    PriceOracle,
    DisputeResolver,
}
