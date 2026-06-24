use soroban_sdk::{panic_with_error, symbol_short, Address, Env, Symbol};

use crate::errors::ParametersError;
use crate::types::{DataKey, MultisigConfig, Proposal, ProtocolParameters};

pub const ADMIN_KEY: Symbol = symbol_short!("ADMIN");
pub const PARAMS_KEY: Symbol = symbol_short!("PARAMS");
pub const REENTRANCY_LOCK: Symbol = symbol_short!("LOCKED");
pub const VERSION_KEY: Symbol = symbol_short!("VERSION");
pub const MULTISIG_KEY: Symbol = symbol_short!("MSIG");
pub const PROP_CNT_KEY: Symbol = symbol_short!("PROPCNT");

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&ADMIN_KEY)
}

pub fn get_admin(env: &Env) -> Result<Address, ParametersError> {
    env.storage()
        .instance()
        .get(&ADMIN_KEY)
        .ok_or(ParametersError::NotInitialized)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&ADMIN_KEY, admin);
}

pub fn get_parameters(env: &Env) -> Result<ProtocolParameters, ParametersError> {
    env.storage()
        .instance()
        .get(&PARAMS_KEY)
        .ok_or(ParametersError::NotInitialized)
}

pub fn set_parameters(env: &Env, params: &ProtocolParameters) {
    env.storage().instance().set(&PARAMS_KEY, params);
}

pub fn is_reentrancy_locked(env: &Env) -> Result<bool, ParametersError> {
    Ok(env
        .storage()
        .instance()
        .get(&REENTRANCY_LOCK)
        .unwrap_or(false))
}

pub fn set_reentrancy_locked(env: &Env, locked: bool) {
    env.storage().instance().set(&REENTRANCY_LOCK, &locked);
}

pub fn get_version(env: &Env) -> Result<u32, ParametersError> {
    Ok(env.storage().instance().get(&VERSION_KEY).unwrap_or(1u32))
}

pub fn set_version(env: &Env, v: u32) {
    env.storage().instance().set(&VERSION_KEY, &v);
}

pub fn has_multisig(env: &Env) -> bool {
    env.storage().instance().has(&MULTISIG_KEY)
}

pub fn get_multisig(env: &Env) -> Result<MultisigConfig, ParametersError> {
    env.storage()
        .instance()
        .get(&MULTISIG_KEY)
        .ok_or(ParametersError::MultisigNotConfigured)
}

pub fn set_multisig(env: &Env, config: &MultisigConfig) {
    env.storage().instance().set(&MULTISIG_KEY, config);
}

pub fn next_proposal_id(env: &Env) -> u64 {
    let id: u64 = env.storage().instance().get(&PROP_CNT_KEY).unwrap_or(0u64);
    let next = id
        .checked_add(1)
        .unwrap_or_else(|| panic_with_error!(env, ParametersError::Overflow));
    env.storage().instance().set(&PROP_CNT_KEY, &next);
    id
}

pub fn get_proposal(env: &Env, id: u64) -> Result<Proposal, ParametersError> {
    env.storage()
        .persistent()
        .get(&DataKey::Proposal(id))
        .ok_or(ParametersError::ProposalNotFound)
}

pub fn set_proposal(env: &Env, proposal: &Proposal) {
    env.storage()
        .persistent()
        .set(&DataKey::Proposal(proposal.id), proposal);
}
