use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol, Vec};

use crate::errors::CreditLineError;
use crate::types::Loan;

// Storage keys
pub const ADMIN_KEY: Symbol = symbol_short!("ADMIN");
pub const LOAN_COUNTER: Symbol = symbol_short!("LOANCNT");
pub const REPUTATION_CONTRACT: Symbol = symbol_short!("REPCONT");
pub const MERCHANT_REGISTRY: Symbol = symbol_short!("MERCHANT");
pub const LIQUIDITY_POOL: Symbol = symbol_short!("LIQPOOL");
pub const TOKEN: Symbol = symbol_short!("TOKEN");
pub const PARAMETERS_CONTRACT: Symbol = symbol_short!("PARAMS");
pub const REENTRANCY_LOCK: Symbol = symbol_short!("LOCKED");

const LOAN_SHARD_COUNT: u32 = 32;

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Loan(u32, u64),
    UserLoanCount(Address),
    UserLoanAt(Address, u64),
    UserActiveDebt(Address),
}

/// Get the admin address from storage
pub fn get_admin(env: &Env) -> Result<Address, CreditLineError> {
    env.storage()
        .instance()
        .get(&ADMIN_KEY)
        .ok_or(CreditLineError::NotInitialized)
}

/// Set the admin address in storage
pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&ADMIN_KEY, admin);
}

/// Get the current loan counter (for generating unique loan IDs)
pub fn get_loan_counter(env: &Env) -> Result<u64, CreditLineError> {
    Ok(env.storage().instance().get(&LOAN_COUNTER).unwrap_or(0))
}

/// Increment and return the next loan ID
pub fn increment_loan_counter(env: &Env) -> Result<u64, CreditLineError> {
    let current = get_loan_counter(env)?;
    let next = current.checked_add(1).ok_or(CreditLineError::Overflow)?;
    env.storage().instance().set(&LOAN_COUNTER, &next);
    Ok(next)
}

/// Read a loan from storage
pub fn read_loan(env: &Env, loan_id: u64) -> Result<Loan, CreditLineError> {
    let shard = loan_shard(loan_id);
    env.storage()
        .persistent()
        .get(&DataKey::Loan(shard, loan_id))
        .ok_or(CreditLineError::LoanNotFound)
}

/// Write a loan to storage
pub fn write_loan(env: &Env, loan: &Loan) {
    let shard = loan_shard(loan.loan_id);
    let key = DataKey::Loan(shard, loan.loan_id);
    let is_new = !env.storage().persistent().has(&key);
    env.storage().persistent().set(&key, loan);
    extend_persistent_ttl_loan(env, &key);

    if is_new {
        append_user_loan_index(env, &loan.borrower, loan.loan_id);
    }
}

pub fn get_user_loan_count(env: &Env, borrower: &Address) -> Result<u64, CreditLineError> {
    Ok(env
        .storage()
        .persistent()
        .get(&DataKey::UserLoanCount(borrower.clone()))
        .unwrap_or(0))
}

pub fn get_user_loan_ids_paginated(
    env: &Env,
    borrower: &Address,
    start: u64,
    limit: u32,
) -> Result<Vec<u64>, CreditLineError> {
    let total = get_user_loan_count(env, borrower)?;
    let mut result = Vec::new(env);

    if limit == 0 || start >= total {
        return Ok(result);
    }

    let end = start.saturating_add(limit as u64).min(total);
    let mut idx = start;
    while idx < end {
        let key = DataKey::UserLoanAt(borrower.clone(), idx);
        if let Some(loan_id) = env.storage().persistent().get::<DataKey, u64>(&key) {
            result.push_back(loan_id);
        }
        idx += 1;
    }

    Ok(result)
}

pub fn get_user_loans_paginated(
    env: &Env,
    borrower: &Address,
    start: u64,
    limit: u32,
) -> Result<Vec<Loan>, CreditLineError> {
    let loan_ids = get_user_loan_ids_paginated(env, borrower, start, limit)?;
    let mut loans = Vec::new(env);

    for loan_id in loan_ids.iter() {
        loans.push_back(read_loan(env, loan_id)?);
    }

    Ok(loans)
}

pub fn get_user_active_debt(env: &Env, borrower: &Address) -> Result<i128, CreditLineError> {
    Ok(env
        .storage()
        .persistent()
        .get(&DataKey::UserActiveDebt(borrower.clone()))
        .unwrap_or(0))
}

pub fn increase_user_active_debt(
    env: &Env,
    borrower: &Address,
    amount: i128,
) -> Result<(), CreditLineError> {
    let current = get_user_active_debt(env, borrower)?;
    let next = current
        .checked_add(amount)
        .ok_or(CreditLineError::Overflow)?;
    let key = DataKey::UserActiveDebt(borrower.clone());
    env.storage().persistent().set(&key, &next);
    extend_persistent_ttl(env, &key);
    Ok(())
}

pub fn decrease_user_active_debt(
    env: &Env,
    borrower: &Address,
    amount: i128,
) -> Result<(), CreditLineError> {
    let current = get_user_active_debt(env, borrower)?;
    let next = current
        .checked_sub(amount)
        .ok_or(CreditLineError::Underflow)?;
    let key = DataKey::UserActiveDebt(borrower.clone());
    env.storage().persistent().set(&key, &next);
    extend_persistent_ttl(env, &key);
    Ok(())
}

fn append_user_loan_index(env: &Env, borrower: &Address, loan_id: u64) {
    let count = get_user_loan_count(env, borrower)
        .unwrap_or_else(|err| soroban_sdk::panic_with_error!(env, err));
    let loan_at_key = DataKey::UserLoanAt(borrower.clone(), count);
    env.storage().persistent().set(&loan_at_key, &loan_id);
    extend_persistent_ttl(env, &loan_at_key);

    let count_key = DataKey::UserLoanCount(borrower.clone());
    let next_count = count
        .checked_add(1)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, CreditLineError::Overflow));
    env.storage().persistent().set(&count_key, &next_count);
    extend_persistent_ttl(env, &count_key);
}

fn loan_shard(loan_id: u64) -> u32 {
    (loan_id % (LOAN_SHARD_COUNT as u64)) as u32
}

/// Get the Reputation Contract address
pub fn get_reputation_contract(env: &Env) -> Result<Option<Address>, CreditLineError> {
    Ok(env.storage().instance().get(&REPUTATION_CONTRACT))
}

/// Set the Reputation Contract address
pub fn set_reputation_contract(env: &Env, address: &Address) {
    env.storage().instance().set(&REPUTATION_CONTRACT, address);
}

/// Get the Vendor Registry Contract address
pub fn get_vendor_registry(env: &Env) -> Result<Option<Address>, CreditLineError> {
    Ok(env.storage().instance().get(&MERCHANT_REGISTRY))
}

/// Set the Vendor Registry Contract address
pub fn set_vendor_registry(env: &Env, address: &Address) {
    env.storage().instance().set(&MERCHANT_REGISTRY, address);
}

/// Get the Liquidity Pool Contract address
pub fn get_liquidity_pool(env: &Env) -> Result<Option<Address>, CreditLineError> {
    Ok(env.storage().instance().get(&LIQUIDITY_POOL))
}

/// Set the Liquidity Pool Contract address
pub fn set_liquidity_pool(env: &Env, address: &Address) {
    env.storage().instance().set(&LIQUIDITY_POOL, address);
}

/// Get the Token Contract address
pub fn get_token(env: &Env) -> Result<Option<Address>, CreditLineError> {
    Ok(env.storage().instance().get(&TOKEN))
}

/// Set the Token Contract address
pub fn set_token(env: &Env, address: &Address) {
    env.storage().instance().set(&TOKEN, address);
}

/// Get the Parameters Contract address
pub fn get_parameters_contract(env: &Env) -> Result<Option<Address>, CreditLineError> {
    Ok(env.storage().instance().get(&PARAMETERS_CONTRACT))
}

/// Set the Parameters Contract address
pub fn set_parameters_contract(env: &Env, address: &Address) {
    env.storage().instance().set(&PARAMETERS_CONTRACT, address);
}

pub fn is_reentrancy_locked(env: &Env) -> Result<bool, CreditLineError> {
    Ok(env
        .storage()
        .instance()
        .get(&REENTRANCY_LOCK)
        .unwrap_or(false))
}

pub fn set_reentrancy_locked(env: &Env, locked: bool) {
    env.storage().instance().set(&REENTRANCY_LOCK, &locked);
}

// TTL constants (in ledgers — 1 ledger ≈ 5 seconds on mainnet)
// 120 days in ledgers
pub const PERSISTENT_TTL_THRESHOLD: u32 = 1_036_800;
pub const PERSISTENT_TTL_EXTEND_TO: u32 = 2_073_600;
// Version key (instance storage) — defaults to 1 when missing
pub const VERSION_KEY: Symbol = symbol_short!("VERSION");

/// Extend TTL for a persistent storage entry
fn extend_persistent_ttl(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

/// Extend TTL for a loan storage entry
fn extend_persistent_ttl_loan(env: &Env, key: &DataKey) {
    extend_persistent_ttl(env, key);
}

/// Get the contract version (instance storage). Defaults to 1 when not set.
pub fn get_version(env: &Env) -> Result<u32, CreditLineError> {
    Ok(env.storage().instance().get(&VERSION_KEY).unwrap_or(1u32))
}

/// Set the contract version in instance storage.
pub fn set_version(env: &Env, version: u32) {
    env.storage().instance().set(&VERSION_KEY, &version);
}
