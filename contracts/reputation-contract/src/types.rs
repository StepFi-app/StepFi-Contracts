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
