#![no_std]

use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contract, contractimpl, panic_with_error, symbol_short, Address, Env, IntoVal, Symbol, Val,
    Vec,
};

mod errors;
mod events;
mod storage;
mod types;

pub use errors::VouchingError;
pub use types::{VouchRecord, DEFAULT_VOUCH_BOOST};

#[contract]
pub struct VouchingContract;

#[contractimpl]
impl VouchingContract {
    pub fn initialize(env: Env, admin: Address, reputation_contract: Address, vouch_boost: u32) {
        admin.require_auth();

        if storage::has_admin(&env) {
            panic_with_error!(&env, VouchingError::AlreadyInitialized);
        }
        if vouch_boost == 0 {
            panic_with_error!(&env, VouchingError::InvalidBoost);
        }

        storage::set_admin(&env, &admin);
        storage::set_reputation_contract(&env, &reputation_contract);
        storage::set_vouch_boost(&env, vouch_boost);
    }

    pub fn set_mentor(env: Env, admin: Address, mentor: Address, verified: bool) {
        admin.require_auth();
        Self::require_admin(&env, &admin);

        Self::enter_non_reentrant(&env);

        storage::set_mentor(&env, &mentor, verified);
        events::emit_mentor_verified(&env, &mentor, verified);

        Self::exit_non_reentrant(&env);
    }

    pub fn vouch(env: Env, mentor: Address, learner: Address) {
        mentor.require_auth();

        if !storage::is_mentor(&env, &mentor) {
            panic_with_error!(&env, VouchingError::MentorNotVerified);
        }

        if let Ok(existing) = storage::get_vouch(&env, &mentor, &learner) {
            storage::extend_vouch_ttl(&env, &mentor, &learner);
            if existing.active {
                panic_with_error!(&env, VouchingError::VouchAlreadyActive);
            }
        }

        Self::enter_non_reentrant(&env);

        let boost_amount =
            storage::get_vouch_boost(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let record = VouchRecord {
            mentor: mentor.clone(),
            learner: learner.clone(),
            ts: env.ledger().timestamp(),
            boost_amount,
            active: true,
        };

        storage::set_vouch(&env, &record);
        storage::add_learner_mentor(&env, &learner, &mentor);
        Self::add_reputation_boost(&env, &learner, boost_amount);
        events::emit_mentor_vouched(&env, &mentor, &learner, boost_amount);

        Self::exit_non_reentrant(&env);
    }

    pub fn revoke_vouch(env: Env, mentor: Address, learner: Address) {
        mentor.require_auth();

        let mut record = storage::get_vouch(&env, &mentor, &learner)
            .unwrap_or_else(|err| panic_with_error!(&env, err));
        if !record.active {
            panic_with_error!(&env, VouchingError::VouchNotActive);
        }

        Self::enter_non_reentrant(&env);

        Self::remove_reputation_boost(&env, &learner, record.boost_amount);
        record.active = false;
        storage::set_vouch(&env, &record);
        events::emit_vouch_revoked(&env, &mentor, &learner, record.boost_amount);

        Self::exit_non_reentrant(&env);
    }

    pub fn get_vouches(env: Env, learner: Address) -> Vec<VouchRecord> {
        let mentors = storage::get_learner_mentors(&env, &learner);
        let mut records = Vec::new(&env);

        for mentor in mentors {
            if let Ok(record) = storage::get_vouch(&env, &mentor, &learner) {
                records.push_back(record);
            }
        }

        records
    }

    pub fn get_admin(env: Env) -> Result<Address, VouchingError> {
        storage::get_admin(&env)
    }

    pub fn is_mentor(env: Env, mentor: Address) -> bool {
        storage::is_mentor(&env, &mentor)
    }

    fn add_reputation_boost(env: &Env, learner: &Address, boost_amount: u32) {
        let reputation_contract =
            storage::get_reputation_contract(env).unwrap_or_else(|err| panic_with_error!(env, err));
        Self::authorize_reputation_call(
            env,
            &reputation_contract,
            symbol_short!("add_boost"),
            learner,
            boost_amount,
        );
        env.try_invoke_contract::<(), soroban_sdk::Error>(
            &reputation_contract,
            &symbol_short!("add_boost"),
            (env.current_contract_address(), learner, boost_amount).into_val(env),
        )
        .unwrap_or_else(|_| panic_with_error!(env, VouchingError::ReputationCallFailed))
        .unwrap_or_else(|_| panic_with_error!(env, VouchingError::ReputationCallFailed));
    }

    fn remove_reputation_boost(env: &Env, learner: &Address, boost_amount: u32) {
        let reputation_contract =
            storage::get_reputation_contract(env).unwrap_or_else(|err| panic_with_error!(env, err));
        let function = Symbol::new(env, "remove_boost");
        Self::authorize_reputation_call(
            env,
            &reputation_contract,
            function.clone(),
            learner,
            boost_amount,
        );
        env.try_invoke_contract::<(), soroban_sdk::Error>(
            &reputation_contract,
            &function,
            (env.current_contract_address(), learner, boost_amount).into_val(env),
        )
        .unwrap_or_else(|_| panic_with_error!(env, VouchingError::ReputationCallFailed))
        .unwrap_or_else(|_| panic_with_error!(env, VouchingError::ReputationCallFailed));
    }

    fn authorize_reputation_call(
        env: &Env,
        reputation_contract: &Address,
        function: Symbol,
        learner: &Address,
        boost_amount: u32,
    ) {
        let args: Vec<Val> = (env.current_contract_address(), learner, boost_amount).into_val(env);
        let invocation = SubContractInvocation {
            context: ContractContext {
                contract: reputation_contract.clone(),
                fn_name: function,
                args,
            },
            sub_invocations: Vec::new(env),
        };
        let mut auth_entries = Vec::new(env);
        auth_entries.push_back(InvokerContractAuthEntry::Contract(invocation));
        env.authorize_as_current_contract(auth_entries);
    }

    fn require_admin(env: &Env, caller: &Address) {
        let admin = storage::get_admin(env).unwrap_or_else(|err| panic_with_error!(env, err));
        if admin != *caller {
            panic_with_error!(env, VouchingError::NotAdmin);
        }
    }

    fn enter_non_reentrant(env: &Env) {
        if storage::is_reentrancy_locked(env)
            .unwrap_or_else(|err| panic_with_error!(env, err))
        {
            panic_with_error!(env, VouchingError::ReentrancyDetected);
        }
        storage::set_reentrancy_locked(env, true);
    }

    fn exit_non_reentrant(env: &Env) {
        storage::set_reentrancy_locked(env, false);
    }
}

#[cfg(test)]
mod tests;
