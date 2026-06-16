use soroban_sdk::{symbol_short, Address, Env, Map, Symbol};

use crate::errors::ReputationError;

// Storage keys for the reputation contract
pub const ADMIN_KEY: Symbol = symbol_short!("ADMIN");
pub const UPDATERS_MAP: Symbol = symbol_short!("UPDATERS");
pub const SCORES_MAP: Symbol = symbol_short!("SCORES");

/// Get the admin address from storage
pub fn get_admin(env: &Env) -> Result<Address, ReputationError> {
    env.storage()
        .instance()
        .get(&ADMIN_KEY)
        .ok_or(ReputationError::NotInitialized)
}

/// Set the admin address in storage
pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&ADMIN_KEY, admin);
}

/// Read a user's reputation score from storage
pub fn read_score(env: &Env, user: &Address) -> Result<u32, ReputationError> {
    let scores: Map<Address, u32> = env
        .storage()
        .instance()
        .get(&SCORES_MAP)
        .unwrap_or_else(|| Map::new(env));

    Ok(scores.get(user.clone()).unwrap_or(0))
}

/// Write a user's reputation score to storage
pub fn write_score(env: &Env, user: &Address, score: u32) {
    let mut scores: Map<Address, u32> = env
        .storage()
        .instance()
        .get(&SCORES_MAP)
        .unwrap_or_else(|| Map::new(env));

    scores.set(user.clone(), score);
    env.storage().instance().set(&SCORES_MAP, &scores);
}

/// Check if an address is an authorized updater
pub fn is_updater(env: &Env, addr: &Address) -> Result<bool, ReputationError> {
    let updaters: Map<Address, bool> = env
        .storage()
        .instance()
        .get(&UPDATERS_MAP)
        .unwrap_or_else(|| Map::new(env));

    Ok(updaters.get(addr.clone()).unwrap_or(false))
}

/// Set an address as an authorized updater
pub fn set_updater(env: &Env, updater: &Address, allowed: bool) {
    let mut updaters: Map<Address, bool> = env
        .storage()
        .instance()
        .get(&UPDATERS_MAP)
        .unwrap_or_else(|| Map::new(env));

    if allowed {
        updaters.set(updater.clone(), true);
    } else {
        updaters.remove(updater.clone());
    }

    env.storage().instance().set(&UPDATERS_MAP, &updaters);
}
