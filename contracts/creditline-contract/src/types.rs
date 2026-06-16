use soroban_sdk::{contracttype, Address};

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
}

// Loan status enum
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoanStatus {
    Pending,
    Active,
    Paid,
    Defaulted,
    Cancelled,
}

// Loan classification enum
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoanType {
    Standard,
    LearnerInstallment,
}

// Repayment installment structure
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepaymentInstallment {
    pub due_date: u64, // Unix timestamp
    pub amount: i128,  // Amount due for this installment
    pub paid: bool,    // Whether this installment has been paid
    pub paid_at: u64,  // Unix timestamp of payment (0 = unpaid)
}

// Loan data structure
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Loan {
    pub loan_id: u64,
    pub borrower: Address,
    pub vendor: Address,
    pub total_amount: i128,
    pub guarantee_amount: i128,
    pub interest_rate_bps: u32,
    pub interest_amount: i128,
    pub service_fee_amount: i128,
    pub principal_outstanding: i128,
    pub interest_outstanding: i128,
    pub service_fee_outstanding: i128,
    pub remaining_balance: i128,
    pub repayment_schedule: soroban_sdk::Vec<RepaymentInstallment>,
    pub status: LoanStatus,
    pub loan_type: LoanType,
    pub created_at: u64,                 // Unix timestamp
    pub funded_at: u64,                  // 0 means not funded yet
    pub late_fees_outstanding: i128,     // accumulated unpaid late fees
    pub late_fee_accrual_timestamp: u64, // last accrual timestamp (0 = never accrued)
}

pub fn default_protocol_parameters() -> ProtocolParameters {
    ProtocolParameters {
        min_guarantee_percent: MIN_GUARANTEE_PERCENT,
        min_reputation_threshold: MIN_REPUTATION_THRESHOLD,
        full_repayment_reward: 10,
        default_penalty: 20,
        large_loan_threshold: 5_000,
        large_loan_default_penalty: 30,
        base_interest_bps: 0,
        grace_period_seconds: 0,
    }
}

// Constants
pub const MIN_GUARANTEE_PERCENT: i128 = 20; // 20% minimum guarantee
pub const MIN_REPUTATION_THRESHOLD: u32 = 50; // Minimum reputation score required
pub const SERVICE_FEE_BPS: i128 = 100; // 1% flat service fee
pub const BPS_DENOMINATOR: i128 = 10_000;
pub const LATE_FEE_BPS_PER_DAY: i128 = 50; // 0.5% of remaining balance per overdue day
pub const SECONDS_PER_DAY: u64 = 86_400;
