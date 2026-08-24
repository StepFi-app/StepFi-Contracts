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
    /// Learner reputation score at vouch time, before this vouch's boost was
    /// applied. Used to clamp boost removal so a learner's score never falls
    /// below what they had without the vouch.
    pub baseline: u32,
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
}
