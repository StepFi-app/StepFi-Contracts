use soroban_sdk::{contracttype, Address, BytesN, Vec};

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
    /// Seconds after the last installment due date before a hard default can be triggered.
    /// During this window the borrower can still repay (with late fees) and no reputation
    /// penalty is applied yet.  Set to 0 to disable the grace period.
    pub grace_period_seconds: u64,
}

pub const DEFAULT_MIN_GUARANTEE_PERCENT: i128 = 20;
pub const DEFAULT_MIN_REPUTATION_THRESHOLD: u32 = 50;
pub const DEFAULT_FULL_REPAYMENT_REWARD: u32 = 10;
pub const DEFAULT_DEFAULT_PENALTY: u32 = 20;
pub const DEFAULT_LARGE_LOAN_THRESHOLD: i128 = 5_000;
pub const DEFAULT_LARGE_LOAN_DEFAULT_PENALTY: u32 = 30;
pub const DEFAULT_BASE_INTEREST_BPS: u32 = 0;
/// Default grace period: disabled (0).  Set via governance to enable, e.g.
/// 259_200 for a 3-day window.
pub const DEFAULT_GRACE_PERIOD_SECONDS: u64 = 0;

pub fn default_parameters() -> ProtocolParameters {
    ProtocolParameters {
        min_guarantee_percent: DEFAULT_MIN_GUARANTEE_PERCENT,
        min_reputation_threshold: DEFAULT_MIN_REPUTATION_THRESHOLD,
        full_repayment_reward: DEFAULT_FULL_REPAYMENT_REWARD,
        default_penalty: DEFAULT_DEFAULT_PENALTY,
        large_loan_threshold: DEFAULT_LARGE_LOAN_THRESHOLD,
        large_loan_default_penalty: DEFAULT_LARGE_LOAN_DEFAULT_PENALTY,
        base_interest_bps: DEFAULT_BASE_INTEREST_BPS,
        grace_period_seconds: DEFAULT_GRACE_PERIOD_SECONDS,
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultisigConfig {
    pub signers: Vec<Address>,
    pub threshold: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalAction {
    UpdateParameters(ProtocolParameters),
    SetAdmin(Address),
    Upgrade(BytesN<32>),
    UpdateSigners(MultisigConfig),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub id: u64,
    pub action: ProposalAction,
    pub proposer: Address,
    pub approvals: Vec<Address>,
    pub created_at: u64,
    pub expires_at: u64,
    pub executed: bool,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Proposal(u64),
}
