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
