use soroban_sdk::{symbol_short, Address, Env, Symbol};

use crate::types::ProtocolParameters;

const PARAMS_UPDATED: Symbol = symbol_short!("PARMUPDT");
const ADMIN_UPDATED: Symbol = symbol_short!("PARMADMN");
const MS_CONFIGURED: Symbol = symbol_short!("MSCONFIG");
const PROP_CREATED: Symbol = symbol_short!("PROPNEW");
const PROP_APPROVED: Symbol = symbol_short!("PROPAPPR");
const PROP_EXECUTED: Symbol = symbol_short!("PROPEXEC");

pub fn emit_parameters_updated(env: &Env, admin: &Address, params: &ProtocolParameters) {
    env.events().publish(
        (PARAMS_UPDATED, admin),
        (
            params.min_guarantee_percent,
            params.min_reputation_threshold,
            params.full_repayment_reward,
            params.default_penalty,
            params.large_loan_threshold,
            params.large_loan_default_penalty,
            params.base_interest_bps,
        ),
    );
}

pub fn emit_admin_updated(env: &Env, old_admin: &Address, new_admin: &Address) {
    env.events()
        .publish((ADMIN_UPDATED, old_admin), new_admin.clone());
}

pub fn emit_contract_upgraded(env: &Env, old_version: u32, new_version: u32) {
    env.events().publish(
        (soroban_sdk::Symbol::new(env, "CONTRACTUPGRADED"),),
        (old_version, new_version, env.ledger().timestamp()),
    );
}

pub fn emit_multisig_configured(env: &Env, threshold: u32, num_signers: u32) {
    env.events()
        .publish((MS_CONFIGURED,), (threshold, num_signers));
}

pub fn emit_proposal_created(env: &Env, id: u64, proposer: &Address) {
    env.events().publish((PROP_CREATED, proposer), id);
}

pub fn emit_proposal_approved(env: &Env, id: u64, signer: &Address, approvals: u32) {
    env.events().publish((PROP_APPROVED, signer), (id, approvals));
}

pub fn emit_proposal_executed(env: &Env, id: u64) {
    env.events().publish((PROP_EXECUTED,), id);
}
