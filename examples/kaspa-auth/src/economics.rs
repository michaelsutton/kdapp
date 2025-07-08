use borsh::{BorshDeserialize, BorshSerialize};
use std::collections::HashMap;
use kdapp::pki::PubKey;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct EpisodeEconomics {
    pub creation_fee: u64,
    pub action_fees: HashMap<String, u64>,
    pub collected_fees: u64,
    pub fee_recipient: Option<PubKey>,
}

impl Default for EpisodeEconomics {
    fn default() -> Self {
        Self {
            creation_fee: 0,
            action_fees: HashMap::new(),
            collected_fees: 0,
            fee_recipient: None,
        }
    }
}

impl EpisodeEconomics {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_creation_fee(mut self, fee: u64) -> Self {
        self.creation_fee = fee;
        self
    }
    
    pub fn with_action_fee(mut self, action: &str, fee: u64) -> Self {
        self.action_fees.insert(action.to_string(), fee);
        self
    }
    
    pub fn collect_fee(&mut self, amount: u64) {
        self.collected_fees += amount;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EconomicParams {
    pub entry_fee: u64,           // Cost to create episode
    pub challenge_fee: u64,       // Cost per challenge request
    pub success_reward: u64,      // Reward for successful auth
    pub oracle_fee: u64,          // Fee for oracle data submission
    pub tournament_buy_in: u64,   // Tournament entry cost
}

impl Default for EconomicParams {
    fn default() -> Self {
        Self {
            entry_fee: 1000,      // 0.001 KAS
            challenge_fee: 500,   // 0.0005 KAS
            success_reward: 2000, // 0.002 KAS
            oracle_fee: 100,      // 0.0001 KAS
            tournament_buy_in: 10000, // 0.01 KAS
        }
    }
}

#[derive(Clone, Debug)]
pub struct EconomicManager {
    params: EconomicParams,
    balances: HashMap<String, u64>,
    escrow: HashMap<u64, u64>, // episode_id -> escrowed amount
}

impl EconomicManager {
    pub fn new(params: EconomicParams) -> Self {
        Self {
            params,
            balances: HashMap::new(),
            escrow: HashMap::new(),
        }
    }
    
    pub fn charge_entry_fee(&mut self, episode_id: u64, participant: &str) -> Result<(), String> {
        let balance = self.balances.get(participant).unwrap_or(&0);
        if *balance < self.params.entry_fee {
            return Err("Insufficient balance".to_string());
        }
        
        self.balances.insert(participant.to_string(), balance - self.params.entry_fee);
        self.escrow.insert(episode_id, self.params.entry_fee);
        Ok(())
    }
    
    pub fn distribute_success_reward(&mut self, episode_id: u64, participant: &str) {
        if let Some(escrowed) = self.escrow.remove(&episode_id) {
            let reward = escrowed + self.params.success_reward;
            let balance = self.balances.get(participant).unwrap_or(&0);
            self.balances.insert(participant.to_string(), balance + reward);
        }
    }
    
    pub fn get_balance(&self, participant: &str) -> u64 {
        *self.balances.get(participant).unwrap_or(&0)
    }
    
    pub fn add_balance(&mut self, participant: &str, amount: u64) {
        let balance = self.balances.get(participant).unwrap_or(&0);
        self.balances.insert(participant.to_string(), balance + amount);
    }
}