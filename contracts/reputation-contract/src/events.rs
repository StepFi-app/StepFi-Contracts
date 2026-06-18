use soroban_sdk::{symbol_short, Address, Env, Symbol};

// Event topics
const SCORE_CHANGED: Symbol = symbol_short!("SCORECHGD");
const UPDATER_CHANGED: Symbol = symbol_short!("UPDCHGD");
const ADMIN_CHANGED: Symbol = symbol_short!("ADMINCHGD");

/// Emit a score changed event
pub fn emit_score_changed(
    env: &Env,
    user: &Address,
    old_score: u32,
    new_score: u32,
    reason: &Symbol,
) {
    env.events()
        .publish((SCORE_CHANGED, user), (old_score, new_score, reason));
}

/// Emit an updater changed event
pub fn emit_updater_changed(env: &Env, updater: &Address, allowed: bool) {
    env.events().publish((UPDATER_CHANGED, updater), allowed);
}

/// Emit an admin changed event
pub fn emit_admin_changed(env: &Env, old_admin: &Address, new_admin: &Address) {
    env.events()
        .publish((ADMIN_CHANGED,), (old_admin, new_admin));
}

pub fn emit_contract_upgraded(env: &Env, old_version: u32, new_version: u32) {
    env.events().publish(
        (Symbol::new(env, "CONTRACTUPGRADED"),),
        (old_version, new_version, env.ledger().timestamp()),
    );
}
