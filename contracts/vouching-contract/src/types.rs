use soroban_sdk::{contracttype, Address};

pub const DEFAULT_VOUCH_BOOST: u32 = 10;

/// How long a vouch remains effective before it expires on-chain. 30 days, in
/// seconds. Expiry is enforced permissionlessly via `expire_vouch`.
pub const VOUCH_DURATION: u64 = 2_592_000;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VouchRecord {
    pub mentor: Address,
    pub learner: Address,
    pub ts: u64,
    pub boost_amount: u32,
    pub active: bool,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    ReputationContract,
    VouchBoost,
    Locked,
    Mentor(Address),
    Vouch(Address, Address),
    LearnerVouches(Address),
    /// Learner reputation score before ANY active vouch's boost was applied.
    /// Captured on the first vouch and shared by every overlapping vouch so
    /// boost removal can be clamped to a single, stable floor.
    LearnerBaseline(Address),
    /// Aggregate boost currently contributed by all active vouches for a learner.
    /// Lets removal stay exact and order-independent across overlapping vouches
    /// and mid-life boost-config changes.
    LearnerTotalBoost(Address),
}
