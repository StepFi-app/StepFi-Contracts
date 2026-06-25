#![no_std]

mod access;
mod errors;
mod events;
mod safe_math;
mod storage;
mod types;

pub use errors::ParametersError;
pub use types::{
    default_parameters, MultisigConfig, Proposal, ProposalAction, ProtocolParameters,
};

use soroban_sdk::{contract, contractimpl, panic_with_error, Address, BytesN, Env, Vec};

const PROPOSAL_TTL_SECONDS: u64 = 604_800;

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

    pub fn configure_multisig(env: Env, signers: Vec<Address>, threshold: u32) {
        let admin = storage::get_admin(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        admin.require_auth();
        access::require_admin(&env, &admin);

        if storage::has_multisig(&env) {
            panic_with_error!(&env, ParametersError::MultisigAlreadyConfigured);
        }

        let config = MultisigConfig { signers, threshold };
        Self::validate_multisig_config(&env, &config);

        Self::enter_non_reentrant(&env);
        storage::set_multisig(&env, &config);
        events::emit_multisig_configured(&env, config.threshold, config.signers.len());
        Self::exit_non_reentrant(&env);
    }

    pub fn get_multisig(env: Env) -> Result<MultisigConfig, ParametersError> {
        storage::get_multisig(&env)
    }

    pub fn propose(env: Env, proposer: Address, action: ProposalAction) -> u64 {
        proposer.require_auth();
        access::require_signer(&env, &proposer);

        match &action {
            ProposalAction::UpdateParameters(p) => Self::validate_parameters(&env, p),
            ProposalAction::UpdateSigners(c) => Self::validate_multisig_config(&env, c),
            _ => {}
        }

        let now = env.ledger().timestamp();
        let expires_at = now
            .checked_add(PROPOSAL_TTL_SECONDS)
            .unwrap_or_else(|| panic_with_error!(&env, ParametersError::Overflow));

        let id = storage::next_proposal_id(&env);
        let mut approvals = Vec::new(&env);
        approvals.push_back(proposer.clone());

        let proposal = Proposal {
            id,
            action,
            proposer: proposer.clone(),
            approvals,
            created_at: now,
            expires_at,
            executed: false,
        };
        storage::set_proposal(&env, &proposal);
        events::emit_proposal_created(&env, id, &proposer);
        id
    }

    pub fn approve(env: Env, signer: Address, proposal_id: u64) {
        signer.require_auth();
        access::require_signer(&env, &signer);

        let mut proposal =
            storage::get_proposal(&env, proposal_id).unwrap_or_else(|err| panic_with_error!(&env, err));

        if proposal.executed {
            panic_with_error!(&env, ParametersError::ProposalAlreadyExecuted);
        }
        if env.ledger().timestamp() > proposal.expires_at {
            panic_with_error!(&env, ParametersError::ProposalExpired);
        }
        if proposal.approvals.contains(&signer) {
            panic_with_error!(&env, ParametersError::DuplicateSignature);
        }

        proposal.approvals.push_back(signer.clone());
        storage::set_proposal(&env, &proposal);
        events::emit_proposal_approved(&env, proposal_id, &signer, proposal.approvals.len());
    }

    /// Execute a proposal once it has collected at least `threshold` approvals.
    /// Permissionless — the collected approvals are the authorization.
    pub fn execute(env: Env, proposal_id: u64) {
        let mut proposal =
            storage::get_proposal(&env, proposal_id).unwrap_or_else(|err| panic_with_error!(&env, err));

        if proposal.executed {
            panic_with_error!(&env, ParametersError::ProposalAlreadyExecuted);
        }
        if env.ledger().timestamp() > proposal.expires_at {
            panic_with_error!(&env, ParametersError::ProposalExpired);
        }

        let config = storage::get_multisig(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        if proposal.approvals.len() < config.threshold {
            panic_with_error!(&env, ParametersError::ThresholdNotMet);
        }

        Self::enter_non_reentrant(&env);
        match proposal.action.clone() {
            ProposalAction::UpdateParameters(p) => Self::do_update_parameters(&env, &p),
            ProposalAction::SetAdmin(a) => Self::do_set_admin(&env, &a),
            ProposalAction::Upgrade(h) => Self::do_upgrade(&env, h),
            ProposalAction::UpdateSigners(c) => Self::do_update_signers(&env, &c),
        }

        proposal.executed = true;
        storage::set_proposal(&env, &proposal);
        events::emit_proposal_executed(&env, proposal_id);
        Self::exit_non_reentrant(&env);
    }

    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<Proposal, ParametersError> {
        storage::get_proposal(&env, proposal_id)
    }


    pub fn get_admin(env: Env) -> Result<Address, ParametersError> {
        storage::get_admin(&env)
    }

    pub fn get_version(env: Env) -> u32 {
        storage::get_version(&env).unwrap_or_else(|err| panic_with_error!(&env, err))
    }

    pub fn get_parameters(env: Env) -> Result<ProtocolParameters, ParametersError> {
        storage::get_parameters(&env)
    }


    fn do_update_parameters(env: &Env, params: &ProtocolParameters) {
        Self::validate_parameters(env, params);
        let admin = storage::get_admin(env).unwrap_or_else(|err| panic_with_error!(env, err));
        storage::set_parameters(env, params);
        events::emit_parameters_updated(env, &admin, params);
    }

    fn do_set_admin(env: &Env, new_admin: &Address) {
        let old_admin = storage::get_admin(env).unwrap_or_else(|err| panic_with_error!(env, err));
        storage::set_admin(env, new_admin);
        events::emit_admin_updated(env, &old_admin, new_admin);
    }

    fn do_upgrade(env: &Env, new_wasm_hash: BytesN<32>) {
        let old = storage::get_version(env).unwrap_or(1u32);
        let new = old.checked_add(1).unwrap_or(old);
        storage::set_version(env, new);
        env.deployer().update_current_contract_wasm(new_wasm_hash);
        events::emit_contract_upgraded(env, old, new);
    }

    fn do_update_signers(env: &Env, config: &MultisigConfig) {
        Self::validate_multisig_config(env, config);
        storage::set_multisig(env, config);
        events::emit_multisig_configured(env, config.threshold, config.signers.len());
    }

    fn validate_parameters(env: &Env, params: &ProtocolParameters) {
        if params.min_guarantee_percent <= 0
            || params.min_guarantee_percent > 100
            || params.large_loan_threshold <= 0
        {
            panic_with_error!(env, ParametersError::InvalidParameters);
        }
    }

    fn validate_multisig_config(env: &Env, config: &MultisigConfig) {
        let n = config.signers.len();
        if config.threshold < 2 || config.threshold > n {
            panic_with_error!(env, ParametersError::InvalidThreshold);
        }
        for i in 0..n {
            let a = config.signers.get_unchecked(i);
            for j in (i + 1)..n {
                if a == config.signers.get_unchecked(j) {
                    panic_with_error!(env, ParametersError::DuplicateSigner);
                }
            }
        }
    }

    fn enter_non_reentrant(env: &Env) {
        if storage::is_reentrancy_locked(env).unwrap_or_else(|err| panic_with_error!(env, err)) {
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
