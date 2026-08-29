extern crate std;

use soroban_sdk::{
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Events, Ledger},
    Address, Env, IntoVal, Symbol, Val, Vec,
};

use crate::{
    VouchingContract, VouchingContractClient, DEFAULT_VOUCH_BOOST, VOUCH_DURATION,
};

#[contract]
pub struct MockReputationContract;

#[contractimpl]
impl MockReputationContract {
    pub fn add_boost(env: Env, updater: Address, learner: Address, amount: u32) {
        updater.require_auth();
        let score = Self::get_score(env.clone(), learner.clone());
        let next = score.checked_add(amount).unwrap_or(100).min(100);
        env.storage()
            .instance()
            .set(&(symbol_short!("SCORE"), learner), &next);
    }

    pub fn remove_boost(env: Env, updater: Address, learner: Address, amount: u32) {
        updater.require_auth();
        let score = Self::get_score(env.clone(), learner.clone());
        let next = score.saturating_sub(amount);
        env.storage()
            .instance()
            .set(&(symbol_short!("SCORE"), learner), &next);
    }

    pub fn get_score(env: Env, learner: Address) -> u32 {
        env.storage()
            .instance()
            .get(&(symbol_short!("SCORE"), learner))
            .unwrap_or(0)
    }

    pub fn decrease_score(env: Env, updater: Address, user: Address, amount: u32) {
        updater.require_auth();
        let score = Self::get_score(env.clone(), user.clone());
        let next = score.saturating_sub(amount);
        env.storage()
            .instance()
            .set(&(symbol_short!("SCORE"), user), &next);
    }
}

struct TestCtx {
    env: Env,
    client: VouchingContractClient<'static>,
    contract_id: Address,
    reputation: Address,
    admin: Address,
    mentor: Address,
    learner: Address,
}

impl TestCtx {
    fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let reputation = env.register(MockReputationContract, ());
        let contract_id = env.register(VouchingContract, ());
        let client = VouchingContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let mentor = Address::generate(&env);
        let learner = Address::generate(&env);

        client.initialize(&admin, &reputation, &DEFAULT_VOUCH_BOOST);

        Self {
            env,
            client,
            contract_id,
            reputation,
            admin,
            mentor,
            learner,
        }
    }

    fn reputation_score(&self) -> u32 {
        let reputation_client = MockReputationContractClient::new(&self.env, &self.reputation);
        reputation_client.get_score(&self.learner)
    }

    /// Directly set the configured vouch boost (simulates an admin changing the
    /// boost config between vouches, which the contract does not yet expose as a
    /// public entrypoint but which the accounting must tolerate).
    fn set_configured_boost(&self, boost: u32) {
        self.env.as_contract(&self.contract_id, || {
            self.env
                .storage()
                .instance()
                .set(&crate::types::DataKey::VouchBoost, &boost);
        });
    }
}

#[test]
fn test_initialize_sets_admin() {
    let ctx = TestCtx::setup();

    assert_eq!(ctx.client.get_admin(), ctx.admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_initialize_rejects_zero_boost() {
    let env = Env::default();
    env.mock_all_auths();

    let reputation = env.register(MockReputationContract, ());
    let contract_id = env.register(VouchingContract, ());
    let client = VouchingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin, &reputation, &0);
}

#[test]
fn test_set_mentor_verifies_mentor_and_emits_event() {
    let ctx = TestCtx::setup();

    ctx.client.set_mentor(&ctx.admin, &ctx.mentor, &true);
    assert_event(&ctx.env, Symbol::new(&ctx.env, "MENTORVERIFIED"));

    assert!(ctx.client.is_mentor(&ctx.mentor));
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_set_mentor_requires_admin() {
    let ctx = TestCtx::setup();
    let not_admin = Address::generate(&ctx.env);

    ctx.client.set_mentor(&not_admin, &ctx.mentor, &true);
}

#[test]
fn test_vouch_writes_record_and_adds_reputation_boost() {
    let ctx = TestCtx::setup();
    ctx.client.set_mentor(&ctx.admin, &ctx.mentor, &true);

    ctx.client.vouch(&ctx.mentor, &ctx.learner);

    let vouches = ctx.client.get_vouches(&ctx.learner);
    assert_eq!(vouches.len(), 1);
    let record = vouches.get_unchecked(0);
    assert_eq!(record.mentor, ctx.mentor);
    assert_eq!(record.learner, ctx.learner);
    assert_eq!(record.boost_amount, DEFAULT_VOUCH_BOOST);
    assert!(record.active);
    assert_eq!(ctx.reputation_score(), DEFAULT_VOUCH_BOOST);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_vouch_rejects_unverified_mentor() {
    let ctx = TestCtx::setup();

    ctx.client.vouch(&ctx.mentor, &ctx.learner);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_vouch_rejects_duplicate_active_vouch() {
    let ctx = TestCtx::setup();
    ctx.client.set_mentor(&ctx.admin, &ctx.mentor, &true);
    ctx.client.vouch(&ctx.mentor, &ctx.learner);

    ctx.client.vouch(&ctx.mentor, &ctx.learner);
}

#[test]
fn test_revoke_vouch_marks_inactive_and_removes_reputation_boost() {
    let ctx = TestCtx::setup();
    ctx.client.set_mentor(&ctx.admin, &ctx.mentor, &true);
    ctx.client.vouch(&ctx.mentor, &ctx.learner);

    ctx.client.revoke_vouch(&ctx.mentor, &ctx.learner);

    let record = ctx.client.get_vouches(&ctx.learner).get_unchecked(0);
    assert!(!record.active);
    assert_eq!(ctx.reputation_score(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_revoke_vouch_rejects_missing_record() {
    let ctx = TestCtx::setup();

    ctx.client.revoke_vouch(&ctx.mentor, &ctx.learner);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_revoke_vouch_rejects_already_inactive_record() {
    let ctx = TestCtx::setup();
    ctx.client.set_mentor(&ctx.admin, &ctx.mentor, &true);
    ctx.client.vouch(&ctx.mentor, &ctx.learner);
    ctx.client.revoke_vouch(&ctx.mentor, &ctx.learner);

    ctx.client.revoke_vouch(&ctx.mentor, &ctx.learner);
}

#[test]
fn test_events_emitted_for_vouch_and_revoke() {
    let ctx = TestCtx::setup();
    ctx.client.set_mentor(&ctx.admin, &ctx.mentor, &true);
    ctx.client.vouch(&ctx.mentor, &ctx.learner);
    assert_event(&ctx.env, Symbol::new(&ctx.env, "MENTORVOUCHED"));
    ctx.client.revoke_vouch(&ctx.mentor, &ctx.learner);
    assert_event(&ctx.env, Symbol::new(&ctx.env, "VOUCHREVOKED"));

    let record = ctx.client.get_vouches(&ctx.learner).get_unchecked(0);
    assert!(!record.active);
    assert_eq!(ctx.reputation_score(), 0);
}

fn assert_event(env: &Env, expected: Symbol) {
    let events: Vec<(Address, Vec<Val>, Val)> = env.events().all();
    for event in events.iter() {
        let topics = event.1.clone();
        let topic: Symbol = topics.get_unchecked(0).into_val(env);
        if topic == expected {
            return;
        }
    }

    panic!("expected event was not emitted");
}

// ============================================================================
// Vouch Expiry & Boost-Accounting Clamp Tests (Issue #87)
// ============================================================================

#[test]
fn test_expire_vouch_removes_boost_permissionless() {
    let ctx = TestCtx::setup();
    ctx.client.set_mentor(&ctx.admin, &ctx.mentor, &true);
    ctx.env.ledger().with_mut(|l| l.timestamp = 1_000_000);
    ctx.client.vouch(&ctx.mentor, &ctx.learner);
    assert_eq!(ctx.reputation_score(), DEFAULT_VOUCH_BOOST);

    // Past the TTL — anyone may trigger expiry without special auth.
    ctx.env
        .ledger()
        .with_mut(|l| l.timestamp = 1_000_000 + VOUCH_DURATION + 1);
    ctx.client.expire_vouch(&ctx.mentor, &ctx.learner);

    let record = ctx.client.get_vouches(&ctx.learner).get_unchecked(0);
    assert!(!record.active);
    assert_eq!(ctx.reputation_score(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_expire_vouch_before_ttl_fails() {
    let ctx = TestCtx::setup();
    ctx.client.set_mentor(&ctx.admin, &ctx.mentor, &true);
    ctx.env.ledger().with_mut(|l| l.timestamp = 1_000_000);
    ctx.client.vouch(&ctx.mentor, &ctx.learner);

    // Still within the TTL — expiry must be rejected.
    ctx.env
        .ledger()
        .with_mut(|l| l.timestamp = 1_000_000 + VOUCH_DURATION - 10);
    ctx.client.expire_vouch(&ctx.mentor, &ctx.learner);
}

#[test]
fn test_expire_vouch_idempotent() {
    let ctx = TestCtx::setup();
    ctx.client.set_mentor(&ctx.admin, &ctx.mentor, &true);
    ctx.env.ledger().with_mut(|l| l.timestamp = 1_000_000);
    ctx.client.vouch(&ctx.mentor, &ctx.learner);

    ctx.env
        .ledger()
        .with_mut(|l| l.timestamp = 1_000_000 + VOUCH_DURATION + 1);
    ctx.client.expire_vouch(&ctx.mentor, &ctx.learner);
    // Second expiry is a no-op rather than an error.
    ctx.client.expire_vouch(&ctx.mentor, &ctx.learner);

    assert_eq!(ctx.reputation_score(), 0);
    let record = ctx.client.get_vouches(&ctx.learner).get_unchecked(0);
    assert!(!record.active);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_revoke_after_expiry_fails() {
    let ctx = TestCtx::setup();
    ctx.client.set_mentor(&ctx.admin, &ctx.mentor, &true);
    ctx.env.ledger().with_mut(|l| l.timestamp = 1_000_000);
    ctx.client.vouch(&ctx.mentor, &ctx.learner);

    ctx.env
        .ledger()
        .with_mut(|l| l.timestamp = 1_000_000 + VOUCH_DURATION + 1);
    ctx.client.expire_vouch(&ctx.mentor, &ctx.learner);
    // Revoking an already-expired (inactive) vouch must fail cleanly.
    ctx.client.revoke_vouch(&ctx.mentor, &ctx.learner);
}

#[test]
fn test_get_vouches_marks_expired_inactive_without_expire() {
    let ctx = TestCtx::setup();
    ctx.client.set_mentor(&ctx.admin, &ctx.mentor, &true);
    ctx.env.ledger().with_mut(|l| l.timestamp = 1_000_000);
    ctx.client.vouch(&ctx.mentor, &ctx.learner);

    // Advance past TTL but never call expire_vouch.
    ctx.env
        .ledger()
        .with_mut(|l| l.timestamp = 1_000_000 + VOUCH_DURATION + 1);
    let record = ctx.client.get_vouches(&ctx.learner).get_unchecked(0);
    assert!(!record.active);
}

#[test]
fn test_boost_removal_clamped_to_baseline() {
    let ctx = TestCtx::setup();
    ctx.client.set_mentor(&ctx.admin, &ctx.mentor, &true);

    // Seed pre-existing reputation so the vouch baseline is non-zero.
    ctx.env.as_contract(&ctx.reputation, || {
        ctx.env.storage().instance().set(
            &(symbol_short!("SCORE"), ctx.learner.clone()),
            &20u32,
        );
    });

    ctx.env.ledger().with_mut(|l| l.timestamp = 1_000_000);
    ctx.client.vouch(&ctx.mentor, &ctx.learner); // score 20 -> 30, baseline 20
    assert_eq!(ctx.reputation_score(), 30);

    // External penalty drops the score below the baseline.
    let rep = MockReputationContractClient::new(&ctx.env, &ctx.reputation);
    rep.decrease_score(&ctx.learner, &ctx.learner, &25);
    assert_eq!(ctx.reputation_score(), 5);

    // Expiring the vouch must not push the score further below the recorded
    // baseline — the removable amount is clamped to zero here.
    ctx.env
        .ledger()
        .with_mut(|l| l.timestamp = 1_000_000 + VOUCH_DURATION + 1);
    ctx.client.expire_vouch(&ctx.mentor, &ctx.learner);

    assert_eq!(ctx.reputation_score(), 5);
    let record = ctx.client.get_vouches(&ctx.learner).get_unchecked(0);
    assert!(!record.active);
}

// ============================================================================
// Audit follow-up: multi-vouch exactness & expire idempotency (Issue #87)
// ============================================================================

/// Issue acceptance criterion 2: a boost-config change between vouch and expiry
/// must keep score accounting exact. Two overlapping vouches with *different*
/// boosts (config changed from 10 to 5 between them). Expiring the older/larger
/// vouch first must remove exactly its 10 boost, not clamp its successor's
/// removal to zero (the drift the original per-record-baseline design had).
#[test]
fn test_boost_config_change_exact_older_larger_expired_first() {
    let ctx = TestCtx::setup();
    ctx.client.set_mentor(&ctx.admin, &ctx.mentor, &true);

    ctx.env.ledger().with_mut(|l| l.timestamp = 1_000_000);
    ctx.client.vouch(&ctx.mentor, &ctx.learner); // boost 10
    assert_eq!(ctx.reputation_score(), 10);

    // Admin changes the global boost config between vouches (now 5).
    ctx.set_configured_boost(5);
    let mentor2 = Address::generate(&ctx.env);
    ctx.client.set_mentor(&ctx.admin, &mentor2, &true);
    ctx.client.vouch(&mentor2, &ctx.learner); // boost 5
    assert_eq!(ctx.reputation_score(), 15);

    // Expire the OLDER/LARGER vouch first — the exact scenario that drifted
    // under the original clamp.
    ctx.env
        .ledger()
        .with_mut(|l| l.timestamp = 1_000_000 + VOUCH_DURATION + 1);
    ctx.client.expire_vouch(&ctx.mentor, &ctx.learner);
    assert_eq!(ctx.reputation_score(), 5);

    // Expire the remaining vouch — must remove exactly its 5 boost.
    ctx.client.expire_vouch(&mentor2, &ctx.learner);
    assert_eq!(ctx.reputation_score(), 0);
}

#[test]
fn test_boost_config_change_exact_newer_smaller_expired_first() {
    let ctx = TestCtx::setup();
    ctx.client.set_mentor(&ctx.admin, &ctx.mentor, &true);

    ctx.env.ledger().with_mut(|l| l.timestamp = 1_000_000);
    ctx.client.vouch(&ctx.mentor, &ctx.learner); // boost 10
    assert_eq!(ctx.reputation_score(), 10);

    ctx.set_configured_boost(5);
    let mentor2 = Address::generate(&ctx.env);
    ctx.client.set_mentor(&ctx.admin, &mentor2, &true);
    ctx.client.vouch(&mentor2, &ctx.learner); // boost 5
    assert_eq!(ctx.reputation_score(), 15);

    // Expire the NEWER/SMALLER vouch first.
    ctx.env
        .ledger()
        .with_mut(|l| l.timestamp = 1_000_000 + VOUCH_DURATION + 1);
    ctx.client.expire_vouch(&mentor2, &ctx.learner);
    assert_eq!(ctx.reputation_score(), 10);

    // Expire the older/larger vouch.
    ctx.client.expire_vouch(&ctx.mentor, &ctx.learner);
    assert_eq!(ctx.reputation_score(), 0);
}

/// `expire_vouch` must be idempotent for already-inactive records even within
/// the TTL: calling it on a revoked (inactive) record is a no-op, not a
/// `VouchNotExpired` panic.
#[test]
fn test_expire_vouch_on_revoked_record_is_noop_within_ttl() {
    let ctx = TestCtx::setup();
    ctx.client.set_mentor(&ctx.admin, &ctx.mentor, &true);
    ctx.env.ledger().with_mut(|l| l.timestamp = 1_000_000);
    ctx.client.vouch(&ctx.mentor, &ctx.learner);
    ctx.client.revoke_vouch(&ctx.mentor, &ctx.learner);
    assert_eq!(ctx.reputation_score(), 0);

    // Still within TTL, but the record is already inactive (revoked) — no-op.
    ctx.env
        .ledger()
        .with_mut(|l| l.timestamp = 1_000_000 + VOUCH_DURATION - 10);
    ctx.client.expire_vouch(&ctx.mentor, &ctx.learner);

    assert_eq!(ctx.reputation_score(), 0);
    let record = ctx.client.get_vouches(&ctx.learner).get_unchecked(0);
    assert!(!record.active);
}

// ============================================================================
// Reentrancy Guard Tests
// ============================================================================

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_reentrancy_guard_on_vouch() {
    let env = Env::default();
    env.mock_all_auths();

    let reputation = env.register(MockReputationContract, ());
    let contract_id = env.register(VouchingContract, ());
    let client = VouchingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let mentor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin, &reputation, &DEFAULT_VOUCH_BOOST);
    client.set_mentor(&admin, &mentor, &true);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&crate::types::DataKey::Locked, &true);
    });

    client.vouch(&mentor, &learner);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_reentrancy_guard_on_revoke_vouch() {
    let env = Env::default();
    env.mock_all_auths();

    let reputation = env.register(MockReputationContract, ());
    let contract_id = env.register(VouchingContract, ());
    let client = VouchingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let mentor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin, &reputation, &DEFAULT_VOUCH_BOOST);
    client.set_mentor(&admin, &mentor, &true);
    client.vouch(&mentor, &learner);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&crate::types::DataKey::Locked, &true);
    });

    client.revoke_vouch(&mentor, &learner);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_reentrancy_guard_on_set_mentor() {
    let env = Env::default();
    env.mock_all_auths();

    let reputation = env.register(MockReputationContract, ());
    let contract_id = env.register(VouchingContract, ());
    let client = VouchingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let mentor = Address::generate(&env);

    client.initialize(&admin, &reputation, &DEFAULT_VOUCH_BOOST);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&crate::types::DataKey::Locked, &true);
    });

    client.set_mentor(&admin, &mentor, &true);
}

#[test]
fn test_reentrancy_guard_allows_normal_operations() {
    let env = Env::default();
    env.mock_all_auths();

    let reputation = env.register(MockReputationContract, ());
    let contract_id = env.register(VouchingContract, ());
    let client = VouchingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let mentor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin, &reputation, &DEFAULT_VOUCH_BOOST);
    client.set_mentor(&admin, &mentor, &true);
    client.vouch(&mentor, &learner);

    let record = client.get_vouches(&learner).get_unchecked(0);
    assert!(record.active);

    // Normal operations after unlocking should work
    let mentor2 = Address::generate(&env);
    client.set_mentor(&admin, &mentor2, &true);
    assert!(client.is_mentor(&mentor2));

    client.revoke_vouch(&mentor, &learner);
    let record = client.get_vouches(&learner).get_unchecked(0);
    assert!(!record.active);
}

#[test]
fn test_reentrancy_guard_is_released_after_call() {
    let env = Env::default();
    env.mock_all_auths();

    let reputation = env.register(MockReputationContract, ());
    let contract_id = env.register(VouchingContract, ());
    let client = VouchingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let mentor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin, &reputation, &DEFAULT_VOUCH_BOOST);
    client.set_mentor(&admin, &mentor, &true);

    // First call should succeed
    client.vouch(&mentor, &learner);

    // Lock should be released, second call should also succeed
    let learner2 = Address::generate(&env);
    // Can't vouch same pair, so create a new learner
    let mentor2 = Address::generate(&env);
    client.set_mentor(&admin, &mentor2, &true);
    client.vouch(&mentor2, &learner2);
    assert_eq!(client.get_vouches(&learner2).len(), 1);
}

// ============================================================================
// Version & Upgrade Tests
// ============================================================================

#[test]
fn test_get_version_returns_default() {
    let ctx = TestCtx::setup();
    assert_eq!(ctx.client.get_version(), 1u32);
}

#[test]
fn test_upgrade_bumps_version_and_emits_event() {
    let ctx = TestCtx::setup();

    assert_eq!(ctx.client.get_version(), 1u32);

    let wasm_hash = ctx.env.deployer().upload_contract_wasm(soroban_sdk::Bytes::from_slice(
        &ctx.env,
        include_bytes!("../../../contracts/test-fixtures/contract.wasm"),
    ));
    ctx.client.upgrade(&wasm_hash);

    let events: soroban_sdk::Vec<(soroban_sdk::Address, soroban_sdk::Vec<soroban_sdk::Val>, soroban_sdk::Val)> = ctx.env.events().all();
    let mut found = false;
    for e in events.iter() {
        let topic: soroban_sdk::Symbol = e.1.get_unchecked(0).into_val(&ctx.env);
        if topic == soroban_sdk::Symbol::new(&ctx.env, "CONUPGRADED") {
            found = true;
            break;
        }
    }
    assert!(found, "CONUPGRADED event not found");
}
