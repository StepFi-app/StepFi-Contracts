use soroban_sdk::{symbol_short, Address, Env, Symbol};

use crate::errors::LiquidityPoolError;

// Instance storage keys
pub const ADMIN_KEY: Symbol = symbol_short!("ADMIN");
pub const TOKEN_KEY: Symbol = symbol_short!("TOKEN");
pub const TOTAL_SHARES_KEY: Symbol = symbol_short!("TOTSHRS");
pub const TOTAL_LIQUIDITY_KEY: Symbol = symbol_short!("TOTLIQ");
pub const LOCKED_LIQUIDITY_KEY: Symbol = symbol_short!("LCKDLIQ");
pub const CREDITLINE_KEY: Symbol = symbol_short!("CRDTLIN");
pub const TREASURY_KEY: Symbol = symbol_short!("TREASURY");
pub const MERCHANT_FUND_KEY: Symbol = symbol_short!("MRCHFND");
pub const REENTRANCY_LOCK_KEY: Symbol = symbol_short!("LOCKED");

// Persistent storage key prefix for LP shares
pub const LP_SHARES_PREFIX: Symbol = symbol_short!("LPSHRS");
pub const PERSISTENT_TTL_THRESHOLD: u32 = 1_036_800;
pub const PERSISTENT_TTL_EXTEND_TO: u32 = 2_073_600;
// Version key (instance storage)
pub const VERSION_KEY: Symbol = symbol_short!("VERSION");

// --- Admin ---

pub fn get_admin(env: &Env) -> Result<Address, LiquidityPoolError> {
    env.storage()
        .instance()
        .get(&ADMIN_KEY)
        .ok_or(LiquidityPoolError::NotInitialized)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&ADMIN_KEY, admin);
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&ADMIN_KEY)
}

// --- Token ---

pub fn get_token(env: &Env) -> Result<Address, LiquidityPoolError> {
    env.storage()
        .instance()
        .get(&TOKEN_KEY)
        .ok_or(LiquidityPoolError::NotInitialized)
}

pub fn set_token(env: &Env, token: &Address) {
    env.storage().instance().set(&TOKEN_KEY, token);
}

// --- CreditLine ---

pub fn get_creditline(env: &Env) -> Result<Option<Address>, LiquidityPoolError> {
    Ok(env.storage().instance().get(&CREDITLINE_KEY))
}

pub fn set_creditline(env: &Env, creditline: &Address) {
    env.storage().instance().set(&CREDITLINE_KEY, creditline);
}

// --- Protocol Treasury ---

pub fn get_treasury(env: &Env) -> Result<Option<Address>, LiquidityPoolError> {
    Ok(env.storage().instance().get(&TREASURY_KEY))
}

pub fn set_treasury(env: &Env, treasury: &Address) {
    env.storage().instance().set(&TREASURY_KEY, treasury);
}

// --- Merchant Incentive Fund ---

pub fn get_merchant_fund(env: &Env) -> Result<Option<Address>, LiquidityPoolError> {
    Ok(env.storage().instance().get(&MERCHANT_FUND_KEY))
}

pub fn set_merchant_fund(env: &Env, merchant_fund: &Address) {
    env.storage()
        .instance()
        .set(&MERCHANT_FUND_KEY, merchant_fund);
}

// --- Total Shares ---

pub fn get_total_shares(env: &Env) -> Result<i128, LiquidityPoolError> {
    Ok(env.storage().instance().get(&TOTAL_SHARES_KEY).unwrap_or(0))
}

pub fn set_total_shares(env: &Env, total: i128) {
    env.storage().instance().set(&TOTAL_SHARES_KEY, &total);
}

// --- Total Liquidity ---

pub fn get_total_liquidity(env: &Env) -> Result<i128, LiquidityPoolError> {
    Ok(env
        .storage()
        .instance()
        .get(&TOTAL_LIQUIDITY_KEY)
        .unwrap_or(0))
}

pub fn set_total_liquidity(env: &Env, total: i128) {
    env.storage().instance().set(&TOTAL_LIQUIDITY_KEY, &total);
}

// --- Locked Liquidity ---

pub fn get_locked_liquidity(env: &Env) -> Result<i128, LiquidityPoolError> {
    Ok(env
        .storage()
        .instance()
        .get(&LOCKED_LIQUIDITY_KEY)
        .unwrap_or(0))
}

pub fn set_locked_liquidity(env: &Env, locked: i128) {
    env.storage().instance().set(&LOCKED_LIQUIDITY_KEY, &locked);
}

// --- LP Shares (persistent per-provider) ---

pub fn get_lp_shares(env: &Env, provider: &Address) -> Result<i128, LiquidityPoolError> {
    Ok(env
        .storage()
        .persistent()
        .get(&(LP_SHARES_PREFIX, provider.clone()))
        .unwrap_or(0))
}

pub fn set_lp_shares(env: &Env, provider: &Address, shares: i128) {
    let key = (LP_SHARES_PREFIX, provider.clone());
    env.storage().persistent().set(&key, &shares);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

pub fn is_reentrancy_locked(env: &Env) -> Result<bool, LiquidityPoolError> {
    Ok(env
        .storage()
        .instance()
        .get(&REENTRANCY_LOCK_KEY)
        .unwrap_or(false))
}

pub fn set_reentrancy_locked(env: &Env, locked: bool) {
    env.storage().instance().set(&REENTRANCY_LOCK_KEY, &locked);
}

pub fn get_version(env: &Env) -> Result<u32, LiquidityPoolError> {
    Ok(env.storage().instance().get(&VERSION_KEY).unwrap_or(1u32))
}

pub fn set_version(env: &Env, v: u32) {
    env.storage().instance().set(&VERSION_KEY, &v);
}

// --- Pending Upgrade Timelock ---
pub const PENDING_UPGRADE_KEY: Symbol = symbol_short!("PNDUPGD");
pub const DEFAULT_UPGRADE_DELAY_SECONDS: u64 = 86_400; // 1 day

pub fn get_pending_upgrade(
    env: &Env,
) -> Result<Option<crate::types::PendingUpgrade>, LiquidityPoolError> {
    Ok(env.storage().instance().get(&PENDING_UPGRADE_KEY))
}

pub fn set_pending_upgrade(env: &Env, upgrade: &crate::types::PendingUpgrade) {
    env.storage().instance().set(&PENDING_UPGRADE_KEY, upgrade);
}

pub fn clear_pending_upgrade(env: &Env) {
    env.storage().instance().remove(&PENDING_UPGRADE_KEY);
}

// --- Parameters Contract ---
pub const PARAMETERS_CONTRACT_KEY: Symbol = symbol_short!("PARAMS");

pub fn get_parameters_contract(env: &Env) -> Result<Option<Address>, LiquidityPoolError> {
    Ok(env.storage().instance().get(&PARAMETERS_CONTRACT_KEY))
}

pub fn set_parameters_contract(env: &Env, address: &Address) {
    env.storage().instance().set(&PARAMETERS_CONTRACT_KEY, address);
}

// --- Paused State ---
pub const PAUSED_KEY: Symbol = symbol_short!("PAUSED");

pub fn is_paused(env: &Env) -> bool {
    env.storage().instance().get(&PAUSED_KEY).unwrap_or(false)
}

pub fn set_paused(env: &Env, paused: bool) {
    env.storage().instance().set(&PAUSED_KEY, &paused);
}

// --- Per-Ledger Outflow Cap (instance) ---
pub const OUTFLOW_CAP_BPS_KEY: Symbol = symbol_short!("OUTFLOW");

/// Get the per-ledger outflow cap expressed in basis points of available
/// liquidity (10000 = 100%). Falls back to `DEFAULT_OUTFLOW_CAP_BPS`.
pub fn get_outflow_cap_bps(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&OUTFLOW_CAP_BPS_KEY)
        .unwrap_or(crate::types::DEFAULT_OUTFLOW_CAP_BPS)
}

pub fn set_outflow_cap_bps(env: &Env, bps: u32) {
    env.storage().instance().set(&OUTFLOW_CAP_BPS_KEY, &bps);
}

// --- Single-Recipient Exposure Cap (instance) ---
pub const MERCHANT_EXPOSURE_CAP_KEY: Symbol = symbol_short!("MCRCAP");

/// Get the cumulative exposure ceiling (token units) for a single recipient.
/// Falls back to `DEFAULT_MERCHANT_EXPOSURE_CAP`.
pub fn get_merchant_exposure_cap(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&MERCHANT_EXPOSURE_CAP_KEY)
        .unwrap_or(crate::types::DEFAULT_MERCHANT_EXPOSURE_CAP)
}

pub fn set_merchant_exposure_cap(env: &Env, amount: i128) {
    env.storage().instance().set(&MERCHANT_EXPOSURE_CAP_KEY, &amount);
}

// --- Optional Vendor Registry (instance) ---
pub const VENDOR_REGISTRY_KEY: Symbol = symbol_short!("VREGD");

/// Get the optional vendor-registry address. When `None`, `fund_loan` performs
/// no vendor cross-check (legacy behavior preserved). A missing key is treated
/// as `None` — the key is only ever present when holding a real address, so
/// reads never hit a stored `void` value.
pub fn get_vendor_registry(env: &Env) -> Result<Option<Address>, LiquidityPoolError> {
    Ok(env.storage().instance().get(&VENDOR_REGISTRY_KEY))
}

pub fn set_vendor_registry(env: &Env, registry: &Option<Address>) {
    match registry {
        Some(addr) => env.storage().instance().set(&VENDOR_REGISTRY_KEY, addr),
        None => env.storage().instance().remove(&VENDOR_REGISTRY_KEY),
    }
}

// --- Rolling Outflow Window (instance) ---
pub const OUTFLOW_SEQ_KEY: Symbol = symbol_short!("LEDSEQ");
pub const OUTFLOW_USED_KEY: Symbol = symbol_short!("LEDOUT");

/// Read the current outflow window as `(ledger_sequence, amount_already_used)`.
/// Persisting the ledger sequence is what makes the window "rolling": the next
/// `fund_loan` on a new ledger sequence resets the used counter to zero.
pub fn get_outflow_window(env: &Env) -> (u32, i128) {
    let seq: u32 = env.storage().instance().get(&OUTFLOW_SEQ_KEY).unwrap_or(0);
    let used: i128 = env
        .storage()
        .instance()
        .get(&OUTFLOW_USED_KEY)
        .unwrap_or(0);
    (seq, used)
}

pub fn set_outflow_window(env: &Env, ledger_seq: u32, used: i128) {
    env.storage().instance().set(&OUTFLOW_SEQ_KEY, &ledger_seq);
    env.storage().instance().set(&OUTFLOW_USED_KEY, &used);
}

// --- Per-Merchant Cumulative Exposure (persistent) ---
pub const MERCHANT_FUNDED_PREFIX: Symbol = symbol_short!("MERCHEX");

/// Get the cumulative amount the pool has funded to a single merchant.
pub fn get_merchant_funded(env: &Env, merchant: &Address) -> Result<i128, LiquidityPoolError> {
    Ok(env
        .storage()
        .persistent()
        .get(&(MERCHANT_FUNDED_PREFIX, merchant.clone()))
        .unwrap_or(0))
}

/// Set the cumulative amount funded to a merchant, extending TTL per the
/// persistent-storage rule.
pub fn set_merchant_funded(env: &Env, merchant: &Address, amount: i128) {
    let key = (MERCHANT_FUNDED_PREFIX, merchant.clone());
    env.storage().persistent().set(&key, &amount);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}
