#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

// Module imports
mod access;
mod errors;
mod events;
mod storage;
mod types;

// Re-export types for external use
pub use errors::ReputationError;

/// Reputation contract structure
#[contract]
pub struct ReputationContract;

/// Contract implementation
#[contractimpl]
impl ReputationContract {
    /// Get the version of this contract
    pub fn get_version() -> Symbol {
        symbol_short!("v1_0_0")
    }

    /// Get the reputation score for a user
    pub fn get_score(env: Env, user: Address) -> u32 {
        storage::read_score(&env, &user)
            .unwrap_or_else(|err| soroban_sdk::panic_with_error!(&env, err))
    }

    /// Increase a user's reputation score by a given amount
    /// Requires authorization from an updater
    pub fn increase_score(env: Env, updater: Address, user: Address, amount: u32) {
        updater.require_auth();
        access::require_updater(&env, &updater);

        Self::enter_non_reentrant(&env);

        let old_score = storage::read_score(&env, &user)
            .unwrap_or_else(|err| soroban_sdk::panic_with_error!(&env, err));
        let new_score = old_score
            .checked_add(amount)
            .ok_or(ReputationError::Overflow)
            .unwrap();

        if new_score > types::MAX_SCORE {
            soroban_sdk::panic_with_error!(&env, ReputationError::Overflow);
        }

        storage::write_score(&env, &user, new_score);

        let reason = symbol_short!("increase");
        events::emit_score_changed(&env, &user, old_score, new_score, &reason);

        Self::exit_non_reentrant(&env);
    }

    /// Decrease a user's reputation score by a given amount
    /// Requires authorization from an updater
    pub fn decrease_score(env: Env, updater: Address, user: Address, amount: u32) {
        updater.require_auth();
        access::require_updater(&env, &updater);

        Self::enter_non_reentrant(&env);

        let old_score = storage::read_score(&env, &user)
            .unwrap_or_else(|err| soroban_sdk::panic_with_error!(&env, err));
        let new_score = match old_score.checked_sub(amount) {
            Some(score) => score,
            None => soroban_sdk::panic_with_error!(&env, ReputationError::Underflow),
        };

        storage::write_score(&env, &user, new_score);

        let reason = symbol_short!("decrease");
        events::emit_score_changed(&env, &user, old_score, new_score, &reason);

        Self::exit_non_reentrant(&env);
    }

    /// Set a user's reputation score to a specific value
    /// Requires authorization from an updater
    pub fn set_score(env: Env, updater: Address, user: Address, new_score: u32) {
        updater.require_auth();
        access::require_updater(&env, &updater);

        if new_score > types::MAX_SCORE {
            soroban_sdk::panic_with_error!(&env, ReputationError::OutOfBounds);
        }

        Self::enter_non_reentrant(&env);

        let old_score = storage::read_score(&env, &user)
            .unwrap_or_else(|err| soroban_sdk::panic_with_error!(&env, err));
        storage::write_score(&env, &user, new_score);

        let reason = symbol_short!("set");
        events::emit_score_changed(&env, &user, old_score, new_score, &reason);

        Self::exit_non_reentrant(&env);
    }

    /// Add a mentor vouching boost to a user's reputation score.
    /// Requires authorization from an updater.
    pub fn add_boost(env: Env, updater: Address, user: Address, amount: u32) {
        updater.require_auth();
        access::require_updater(&env, &updater);

        Self::enter_non_reentrant(&env);

        let old_score = storage::read_score(&env, &user)
            .unwrap_or_else(|err| soroban_sdk::panic_with_error!(&env, err));
        let new_score = old_score
            .checked_add(amount)
            .ok_or(ReputationError::Overflow)
            .unwrap();

        if new_score > types::MAX_SCORE {
            soroban_sdk::panic_with_error!(&env, ReputationError::Overflow);
        }

        storage::write_score(&env, &user, new_score);

        let reason = symbol_short!("boost");
        events::emit_score_changed(&env, &user, old_score, new_score, &reason);

        Self::exit_non_reentrant(&env);
    }

    /// Remove a mentor vouching boost from a user's reputation score.
    /// Requires authorization from an updater.
    pub fn remove_boost(env: Env, updater: Address, user: Address, amount: u32) {
        updater.require_auth();
        access::require_updater(&env, &updater);

        Self::enter_non_reentrant(&env);

        let old_score = storage::read_score(&env, &user)
            .unwrap_or_else(|err| soroban_sdk::panic_with_error!(&env, err));
        let new_score = match old_score.checked_sub(amount) {
            Some(score) => score,
            None => soroban_sdk::panic_with_error!(&env, ReputationError::Underflow),
        };

        storage::write_score(&env, &user, new_score);

        let reason = symbol_short!("unboost");
        events::emit_score_changed(&env, &user, old_score, new_score, &reason);

        Self::exit_non_reentrant(&env);
    }

    /// Set or remove an address as an authorized updater
    /// Requires authorization from admin
    pub fn set_updater(env: Env, admin: Address, updater: Address, allowed: bool) {
        admin.require_auth();
        access::require_admin(&env, &admin);

        Self::enter_non_reentrant(&env);

        storage::set_updater(&env, &updater, allowed);
        events::emit_updater_changed(&env, &updater, allowed);

        Self::exit_non_reentrant(&env);
    }

    /// Check if an address is an authorized updater
    pub fn is_updater(env: Env, addr: Address) -> bool {
        storage::is_updater(&env, &addr)
            .unwrap_or_else(|err| soroban_sdk::panic_with_error!(&env, err))
    }

    /// Set the admin address for this contract
    /// Requires authorization from current admin (or allows initial setup)
    pub fn set_admin(env: Env, new_admin: Address) {
        let old_admin_opt: Option<Address> = env.storage().instance().get(&storage::ADMIN_KEY);

        if let Some(old_admin) = old_admin_opt {
            // Admin exists, require current admin authorization
            old_admin.require_auth();
            access::require_admin(&env, &old_admin);

            Self::enter_non_reentrant(&env);

            storage::set_admin(&env, &new_admin);
            events::emit_admin_changed(&env, &old_admin, &new_admin);

            Self::exit_non_reentrant(&env);
        } else {
            // No admin exists, allow setting (initialization)
            storage::set_admin(&env, &new_admin);
            let dummy = new_admin.clone();
            events::emit_admin_changed(&env, &dummy, &new_admin);
        }
    }

    /// Upgrade the contract WASM — admin only
    pub fn upgrade(env: Env, new_wasm_hash: soroban_sdk::BytesN<32>) {
        let admin = storage::get_admin(&env)
            .unwrap_or_else(|err| soroban_sdk::panic_with_error!(&env, err));
        admin.require_auth();

        Self::enter_non_reentrant(&env);
        env.deployer().update_current_contract_wasm(new_wasm_hash);
        Self::exit_non_reentrant(&env);
    }
    pub fn get_admin(env: Env) -> Result<Address, ReputationError> {
        storage::get_admin(&env)
    }

    fn enter_non_reentrant(env: &Env) {
        if storage::is_reentrancy_locked(env)
            .unwrap_or_else(|err| soroban_sdk::panic_with_error!(env, err))
        {
            soroban_sdk::panic_with_error!(env, ReputationError::ReentrancyDetected);
        }
        storage::set_reentrancy_locked(env, true);
    }

    fn exit_non_reentrant(env: &Env) {
        storage::set_reentrancy_locked(env, false);
    }
}

#[cfg(test)]
mod tests;
