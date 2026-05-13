use soroban_sdk::{symbol_short, Address, Env, Symbol, Vec};

use crate::types::RepaymentInstallment;

// Event topics
const LOAN_CREATED: Symbol = symbol_short!("LOANCRTD");
const LOAN_REQUESTED: Symbol = symbol_short!("LOANRQST");
const LOAN_DEFAULTED: Symbol = symbol_short!("LOANDFLT");
const LOAN_REPAID: Symbol = symbol_short!("LOANRPD");
const LOAN_CANCELLED: Symbol = symbol_short!("LOANCNCL");
const LOAN_LATE_FEE: Symbol = symbol_short!("LOANLTFE");
const LOAN_GRACE_PERIOD: Symbol = symbol_short!("LOANGRC");
const INSTALLMENT_PAID: Symbol = symbol_short!("INSTPAID");

pub const LOANAPPROVED: &str = "LOANAPPROVED";

pub fn emit_loan_approved(env: &Env, loan_id: u64) {
    let topics = (Symbol::new(env, LOANAPPROVED), loan_id);
    env.events().publish(topics, ());
}

/// Emit a loan created event
pub fn emit_loan_created(
    env: &Env,
    user: &Address,
    vendor: &Address,
    loan_id: u64,
    total_amount: i128,
    guarantee_amount: i128,
    repayment_schedule: &Vec<RepaymentInstallment>,
) {
    env.events().publish(
        (LOAN_CREATED, user, vendor),
        (
            loan_id,
            total_amount,
            guarantee_amount,
            repayment_schedule.clone(),
        ),
    );
}

pub fn emit_loan_requested(
    env: &Env,
    user: &Address,
    vendor: &Address,
    loan_id: u64,
    total_amount: i128,
    guarantee_amount: i128,
    repayment_schedule: &Vec<RepaymentInstallment>,
) {
    env.events().publish(
        (LOAN_REQUESTED, user, vendor),
        (
            loan_id,
            total_amount,
            guarantee_amount,
            repayment_schedule.clone(),
        ),
    );
}

pub fn emit_loan_defaulted(
    env: &Env,
    borrower: Address,
    loan_id: u64,
    total_amount: i128,
    unpaid_balance: i128,
    guarantee_forfeited: i128,
) {
    env.events().publish(
        (LOAN_DEFAULTED, borrower, loan_id),
        (
            total_amount,
            unpaid_balance,
            guarantee_forfeited,
            env.ledger().timestamp(),
        ),
    );
}

pub fn emit_loan_repaid(
    env: &Env,
    borrower: &Address,
    loan_id: u64,
    amount_paid: i128,
    remaining_balance: i128,
    is_fully_repaid: bool,
) {
    env.events().publish(
        (LOAN_REPAID, borrower, loan_id),
        (
            amount_paid,
            remaining_balance,
            is_fully_repaid,
            env.ledger().timestamp(),
        ),
    );
}

pub fn emit_loan_cancelled(env: &Env, borrower: &Address, loan_id: u64, refunded_guarantee: i128) {
    env.events().publish(
        (LOAN_CANCELLED, borrower, loan_id),
        (refunded_guarantee, env.ledger().timestamp()),
    );
}

pub fn emit_late_fee_accrued(
    env: &Env,
    borrower: &Address,
    loan_id: u64,
    fee_amount: i128,
    new_remaining_balance: i128,
) {
    env.events().publish(
        (LOAN_LATE_FEE, borrower, loan_id),
        (fee_amount, new_remaining_balance, env.ledger().timestamp()),
    );
}

/// Emitted when a single installment is marked paid via `repay_installment`.
pub fn emit_installment_paid(
    env: &Env,
    loan_id: u64,
    installment_index: u32,
    amount: i128,
    remaining_balance: i128,
) {
    env.events().publish(
        (INSTALLMENT_PAID, loan_id),
        (
            installment_index,
            amount,
            remaining_balance,
            env.ledger().timestamp(),
        ),
    );
}

/// Emitted when a loan is past its due date but still within the grace period.
/// Signals that the borrower can still repay before a hard default is triggered.
pub fn emit_loan_in_grace_period(
    env: &Env,
    borrower: &Address,
    loan_id: u64,
    remaining_balance: i128,
    grace_period_ends_at: u64,
) {
    env.events().publish(
        (LOAN_GRACE_PERIOD, borrower, loan_id),
        (
            remaining_balance,
            grace_period_ends_at,
            env.ledger().timestamp(),
        ),
    );
}
