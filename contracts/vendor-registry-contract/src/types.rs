use soroban_sdk::{contracttype, Address, String};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    // Instance storage
    Admin,
    Locked,

    // Persistent storage
    Vendor(Address),
    VendorCount,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VendorStatus {
    Pending = 0,
    Approved = 1,
    Suspended = 2,
    Rejected = 3,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VendorInfo {
    pub name: String,
    pub registration_date: u64,
    pub status: VendorStatus,
    pub total_sales: u64,
}

/// Pending timelocked contract upgrade proposal
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingUpgrade {
    pub wasm_hash: soroban_sdk::BytesN<32>,
    pub proposed_at: u64,
    pub unlock_at: u64,
}

/// Protocol parameters structure for cross-contract governance fetching
#[contracttype]
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
