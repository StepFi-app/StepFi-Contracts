use soroban_sdk::{symbol_short, Address, Env, Symbol};

const DEPOSITED: Symbol = symbol_short!("LQDEPST");
const WITHDRAWN: Symbol = symbol_short!("LQWTHDR");
const LOAN_FUNDED: Symbol = symbol_short!("LQFUND");
const REPAYMENT_RCV: Symbol = symbol_short!("LQREPAY");
const GUARANTEE_RCV: Symbol = symbol_short!("LQGUART");
const INTEREST_DIST: Symbol = symbol_short!("LQINTDST");
const LOSS_ABSORBED: Symbol = symbol_short!("LQLOSS");

/// Emitted when a liquidity provider deposits tokens
pub fn emit_liquidity_deposited(env: &Env, provider: &Address, amount: i128, shares_issued: i128) {
    env.events()
        .publish((DEPOSITED, provider), (amount, shares_issued));
}

/// Emitted when a liquidity provider withdraws tokens
pub fn emit_liquidity_withdrawn(
    env: &Env,
    provider: &Address,
    shares_burned: i128,
    amount_returned: i128,
) {
    env.events()
        .publish((WITHDRAWN, provider), (shares_burned, amount_returned));
}

/// Emitted when the pool funds a loan (CreditLine → merchant).
/// Payload carries the recipient, the funded amount, and the remaining
/// per-ledger outflow and per-merchant exposure caps for indexer monitoring.
pub fn emit_loan_funded(
    env: &Env,
    creditline: &Address,
    merchant: &Address,
    amount: i128,
    outflow_remaining: i128,
    merchant_remaining: i128,
) {
    env.events().publish(
        (LOAN_FUNDED, creditline),
        (
            merchant.clone(),
            amount,
            outflow_remaining,
            merchant_remaining,
        ),
    );
}

/// Emitted when principal + interest repayment is received from CreditLine
pub fn emit_repayment_received(env: &Env, creditline: &Address, principal: i128, interest: i128) {
    env.events()
        .publish((REPAYMENT_RCV, creditline), (principal, interest));
}

/// Emitted when a forfeited guarantee is received on loan default
pub fn emit_guarantee_received(env: &Env, creditline: &Address, amount: i128) {
    env.events().publish((GUARANTEE_RCV, creditline), amount);
}

/// Emitted when interest is distributed to LPs, treasury, and merchant fund
pub fn emit_interest_distributed(
    env: &Env,
    total_interest: i128,
    lp_amount: i128,
    protocol_amount: i128,
    merchant_amount: i128,
) {
    env.events().publish(
        (INTEREST_DIST,),
        (total_interest, lp_amount, protocol_amount, merchant_amount),
    );
}

/// Emitted when a principal shortfall is absorbed from a defaulted loan
pub fn emit_loss_absorbed(env: &Env, creditline: &Address, principal_shortfall: i128) {
    env.events().publish((LOSS_ABSORBED, creditline), principal_shortfall);
}

pub fn emit_contract_upgraded(env: &Env, old_version: u32, new_version: u32) {
    env.events().publish(
        (soroban_sdk::Symbol::new(env, "CONTRACTUPGRADED"),),
        (old_version, new_version, env.ledger().timestamp()),
    );
}

pub fn emit_upgrade_proposed(
    env: &Env,
    wasm_hash: &soroban_sdk::BytesN<32>,
    proposed_at: u64,
    unlock_at: u64,
) {
    env.events().publish(
        (symbol_short!("UPGDPRP"),),
        (wasm_hash.clone(), proposed_at, unlock_at),
    );
}

pub fn emit_paused(env: &Env, admin: &Address) {
    env.events().publish(
        (symbol_short!("PAUSED"), admin),
        env.ledger().timestamp(),
    );
}

pub fn emit_unpaused(env: &Env, admin: &Address) {
    env.events().publish(
        (symbol_short!("UNPAUSED"), admin),
        env.ledger().timestamp(),
    );
}

const CAPS_UPDATED: Symbol = symbol_short!("CAPSUPD");
const VREG_UPDATED: Symbol = symbol_short!("VREGUPD");

/// Emitted whenever the admin changes the outflow or exposure caps.
/// Carries the new `(outflow_cap_bps, exposure_cap)` for indexers.
pub fn emit_caps_updated(env: &Env, admin: &Address, outflow_cap_bps: u32, exposure_cap: i128) {
    env.events()
        .publish((CAPS_UPDATED, admin), (outflow_cap_bps, exposure_cap));
}

/// Emitted whenever the admin sets or clears the vendor-registry address.
pub fn emit_vendor_registry_updated(env: &Env, admin: &Address, registry: &Option<Address>) {
    env.events()
        .publish((VREG_UPDATED, admin), (registry.clone(),));
}
