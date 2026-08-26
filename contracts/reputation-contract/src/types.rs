use soroban_sdk::{Address, Symbol};

// Score change event data structure
#[allow(dead_code)]
pub struct ScoreChanged {
    pub user: Address,
    pub old: u32,
    pub new: u32,
    pub reason: Symbol,
}

// Updater change event data structure
#[allow(dead_code)]
pub struct UpdaterChanged {
    pub updater: Address,
    pub allowed: bool,
}

// Admin change event data structure
#[allow(dead_code)]
pub struct AdminChanged {
    pub old_admin: Address,
    pub new_admin: Address,
}

// Constants for score bounds
#[allow(dead_code)]
pub const MIN_SCORE: u32 = 0;
pub const MAX_SCORE: u32 = 100;

/// Pending timelocked contract upgrade proposal
#[soroban_sdk::contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingUpgrade {
    pub wasm_hash: soroban_sdk::BytesN<32>,
    pub proposed_at: u64,
    pub unlock_at: u64,
}

/// Protocol parameters structure for cross-contract governance fetching
#[soroban_sdk::contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolParameters {
    pub min_guarantee_percent: i128,
    pub min_reputation_threshold: u32,
    pub full_repayment_reward: u32,
    pub default_penalty: u32,
    pub large_loan_threshold: i128,
    pub large_loan_default_penalty: u32,
    pub base_interest_bps: u32,
    pub grace_period_seconds: u64,
    pub upgrade_delay_seconds: u64,
}
