extern crate std;

use soroban_sdk::{
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Events},
    Address, Env, IntoVal, Symbol, Val, Vec,
};

use crate::{VouchingContract, VouchingContractClient, DEFAULT_VOUCH_BOOST};

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
}

struct TestCtx {
    env: Env,
    client: VouchingContractClient<'static>,
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
