#![no_std]

mod access;
mod errors;
mod events;
mod storage;
mod types;

pub use errors::ParametersError;
pub use types::{default_parameters, ProtocolParameters};

use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Env};

#[contract]
pub struct ParametersContract;

#[contractimpl]
impl ParametersContract {
    pub fn initialize(env: Env, admin: Address, params: ProtocolParameters) {
        if storage::has_admin(&env) {
            panic_with_error!(&env, ParametersError::AlreadyInitialized);
        }

        Self::validate_parameters(&env, &params);
        admin.require_auth();

        storage::set_admin(&env, &admin);
        storage::set_parameters(&env, &params);
        events::emit_parameters_updated(&env, &admin, &params);
    }

    pub fn initialize_defaults(env: Env, admin: Address) {
        Self::initialize(env, admin, default_parameters());
    }

    /// Upgrade the contract WASM — admin only
    pub fn upgrade(env: Env, new_wasm_hash: soroban_sdk::BytesN<32>) {
        let admin = storage::get_admin(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        admin.require_auth();
        Self::enter_non_reentrant(&env);
        env.deployer().update_current_contract_wasm(new_wasm_hash);
        Self::exit_non_reentrant(&env);
    }
    pub fn get_admin(env: Env) -> Result<Address, ParametersError> {
        storage::get_admin(&env)
    }

    pub fn set_admin(env: Env, new_admin: Address) {
        let old_admin = storage::get_admin(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        old_admin.require_auth();
        access::require_admin(&env, &old_admin);

        Self::enter_non_reentrant(&env);
        storage::set_admin(&env, &new_admin);
        events::emit_admin_updated(&env, &old_admin, &new_admin);
        Self::exit_non_reentrant(&env);
    }

    pub fn get_parameters(env: Env) -> Result<ProtocolParameters, ParametersError> {
        storage::get_parameters(&env)
    }

    pub fn update_parameters(env: Env, admin: Address, params: ProtocolParameters) {
        admin.require_auth();
        access::require_admin(&env, &admin);
        Self::validate_parameters(&env, &params);

        Self::enter_non_reentrant(&env);
        storage::set_parameters(&env, &params);
        events::emit_parameters_updated(&env, &admin, &params);
        Self::exit_non_reentrant(&env);
    }

    fn validate_parameters(env: &Env, params: &ProtocolParameters) {
        if params.min_guarantee_percent <= 0
            || params.min_guarantee_percent > 100
            || params.large_loan_threshold <= 0
        {
            panic_with_error!(env, ParametersError::InvalidParameters);
        }
    }

    fn enter_non_reentrant(env: &Env) {
        if storage::is_reentrancy_locked(env)
            .unwrap_or_else(|err| panic_with_error!(env, err))
        {
            panic_with_error!(env, ParametersError::ReentrancyDetected);
        }
        storage::set_reentrancy_locked(env, true);
    }

    fn exit_non_reentrant(env: &Env) {
        storage::set_reentrancy_locked(env, false);
    }
}

#[cfg(test)]
mod tests;
