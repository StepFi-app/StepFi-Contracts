use soroban_sdk::{contracttype, Address};

pub const DEFAULT_VOUCH_BOOST: u32 = 10;

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
}
