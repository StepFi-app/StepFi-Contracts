use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events},
    Address, Env, IntoVal, Symbol, Val, Vec,
};

use crate::ReputationContractClient;
use crate::{ReputationContract, ReputationError};

/// Test: Sets the contract admin
#[test]
fn it_sets_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let retrieved_admin = client.get_admin();
    assert_eq!(retrieved_admin, admin);
}

#[test]
fn it_returns_typed_error_when_getting_admin_before_initialization() {
    let env = Env::default();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    assert_eq!(
        client.try_get_admin(),
        Err(Ok(ReputationError::NotInitialized))
    );
}

/// Test: Gets the contract admin
#[test]
fn it_gets_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let retrieved = client.get_admin();
    assert_eq!(retrieved, admin);
}

/// Test: Assigns updater permissions
#[test]
fn it_sets_updater() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    assert!(client.is_updater(&updater));
}

/// Test: Checks updater permissions
#[test]
fn it_checks_updater() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    let non_updater = Address::generate(&env);

    client.set_updater(&admin, &updater, &true);

    assert!(client.is_updater(&updater));
    assert!(!client.is_updater(&non_updater));
}

/// Test: Gets the reputation score
#[test]
fn it_gets_score() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    assert_eq!(client.get_score(&user), 0);

    client.set_score(&updater, &user, &50);
    assert_eq!(client.get_score(&user), 50);
}

/// Test: Increases the reputation score
#[test]
fn it_increases_score() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    client.set_score(&updater, &user, &50);
    client.increase_score(&updater, &user, &20);

    assert_eq!(client.get_score(&user), 70);
}

/// Test: Decreases the reputation score
#[test]
fn it_decreases_score() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    client.set_score(&updater, &user, &50);
    client.decrease_score(&updater, &user, &20);

    assert_eq!(client.get_score(&user), 30);
}

/// Test: Sets the score to a specific value
#[test]
fn it_sets_score() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    client.set_score(&updater, &user, &75);
    assert_eq!(client.get_score(&user), 75);

    client.set_score(&updater, &user, &25);
    assert_eq!(client.get_score(&user), 25);
}

#[test]
fn it_adds_boost() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    client.set_score(&updater, &user, &40);
    client.add_boost(&updater, &user, &10);

    assert_eq!(client.get_score(&user), 50);
}

#[test]
fn it_removes_boost() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    client.set_score(&updater, &user, &40);
    client.remove_boost(&updater, &user, &10);

    assert_eq!(client.get_score(&user), 30);
}

/// Test: Prevents unauthorized updates
#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn it_prevents_unauthorized_updates() {
    let env = Env::default();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    env.mock_all_auths();
    client.set_admin(&admin);

    let user = Address::generate(&env);
    let unauthorized = Address::generate(&env);

    client.mock_all_auths().set_score(&unauthorized, &user, &50);
}

/// Test: Validates score bounds (0-100)
#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn it_enforces_score_bounds() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    client.set_score(&updater, &user, &101);
}

/// Test: Gets the contract version
#[test]
fn it_gets_version() {
    let env = Env::default();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let version = client.get_version();
    assert_eq!(version, symbol_short!("v1_0_0"));
}

/// Test: Revokes updater access after removal
#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn it_revokes_updater_access_after_removal() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    client.set_score(&updater, &user, &20);
    assert_eq!(client.get_score(&user), 20);

    client.set_updater(&admin, &updater, &false);
    assert!(!client.is_updater(&updater));

    client.increase_score(&updater, &user, &5);
}

/// Test: Emitted event on updater removal
#[test]
fn it_emits_event_on_updater_removal() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);
    assert!(client.is_updater(&updater));

    client.set_updater(&admin, &updater, &false);
    assert!(!client.is_updater(&updater));
}

/// Test: Removing a non-existent updater should not panic
#[test]
fn it_handles_removing_non_existent_updater() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let never_added = Address::generate(&env);

    client.set_updater(&admin, &never_added, &false);
    assert!(!client.is_updater(&never_added));
}

/// Overflow/Underflow tests
#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn it_prevents_overflow_on_increase() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    client.set_score(&updater, &user, &80);
    client.increase_score(&updater, &user, &50);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn it_prevents_overflow_at_max() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    client.set_score(&updater, &user, &100);
    client.increase_score(&updater, &user, &1);
}

#[test]
fn it_allows_increase_up_to_max() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    client.set_score(&updater, &user, &80);
    client.increase_score(&updater, &user, &20);
    assert_eq!(client.get_score(&user), 100);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn it_prevents_underflow_on_decrease() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    client.set_score(&updater, &user, &30);
    client.decrease_score(&updater, &user, &50);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn it_prevents_underflow_at_min() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    client.set_score(&updater, &user, &0);
    client.decrease_score(&updater, &user, &1);
}

#[test]
fn it_allows_decrease_down_to_min() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    client.set_score(&updater, &user, &30);
    client.decrease_score(&updater, &user, &30);
    assert_eq!(client.get_score(&user), 0);
}

/// Test: Removing one updater doesn't affect other updaters
#[test]
fn it_removing_one_updater_does_not_affect_others() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater1 = Address::generate(&env);
    let updater2 = Address::generate(&env);
    client.set_updater(&admin, &updater1, &true);
    client.set_updater(&admin, &updater2, &true);

    let user = Address::generate(&env);

    client.set_score(&updater1, &user, &10);
    client.increase_score(&updater1, &user, &5);
    assert_eq!(client.get_score(&user), 15);

    client.set_updater(&admin, &updater1, &false);
    assert!(!client.is_updater(&updater1));
    assert!(client.is_updater(&updater2));

    client.increase_score(&updater2, &user, &5);
    assert_eq!(client.get_score(&user), 20);
}

/// Test: Emits SCORECHGD event on score increase
#[test]
fn it_emits_score_changed_event_on_increase() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);
    client.set_score(&updater, &user, &50);

    client.increase_score(&updater, &user, &20);

    let events: Vec<(Address, Vec<Val>, Val)> = env.events().all();

    let mut found_event = false;
    for event in events.iter() {
        let topics = event.1.clone();
        let event_type: Symbol = topics.get(0).unwrap().into_val(&env);

        if event_type == symbol_short!("SCORECHGD") {
            found_event = true;

            let event_user: Address = topics.get(1).unwrap().into_val(&env);
            assert_eq!(event_user, user);

            let data_tuple: (u32, u32, Symbol) = event.2.into_val(&env);
            let (old_score, new_score, reason) = data_tuple;

            assert_eq!(old_score, 50);
            assert_eq!(new_score, 70);
            assert_eq!(reason, symbol_short!("increase"));
            break;
        }
    }

    assert!(found_event, "SCORECHGD event not found");
}

/// Test: Emits SCORECHGD event on score decrease
#[test]
fn it_emits_score_changed_event_on_decrease() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);
    client.set_score(&updater, &user, &50);

    client.decrease_score(&updater, &user, &20);

    let events: Vec<(Address, Vec<Val>, Val)> = env.events().all();

    let mut found_event = false;
    for event in events.iter() {
        let topics = event.1.clone();
        let event_type: Symbol = topics.get(0).unwrap().into_val(&env);

        if event_type == symbol_short!("SCORECHGD") {
            let data_tuple: (u32, u32, Symbol) = event.2.into_val(&env);
            let (_, _new_score, reason) = data_tuple;

            if reason == symbol_short!("decrease") {
                found_event = true;

                let event_user: Address = topics.get(1).unwrap().into_val(&env);
                assert_eq!(event_user, user);

                let (old_score, new_score, _) = data_tuple;
                assert_eq!(old_score, 50);
                assert_eq!(new_score, 30);
                break;
            }
        }
    }

    assert!(
        found_event,
        "SCORECHGD event with 'decrease' reason not found"
    );
}

/// Test: Emits SCORECHGD event on score set
#[test]
fn it_emits_score_changed_event_on_set() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);
    client.set_score(&updater, &user, &50);

    client.set_score(&updater, &user, &75);

    let events: Vec<(Address, Vec<Val>, Val)> = env.events().all();

    let mut found_event = false;
    for event in events.iter() {
        let topics = event.1.clone();
        let event_type: Symbol = topics.get(0).unwrap().into_val(&env);

        if event_type == symbol_short!("SCORECHGD") {
            let data_tuple: (u32, u32, Symbol) = event.2.into_val(&env);
            let (_, _, reason) = data_tuple;

            if reason == symbol_short!("set") {
                found_event = true;

                let event_user: Address = topics.get(1).unwrap().into_val(&env);
                assert_eq!(event_user, user);

                let (old_score, new_score, _) = data_tuple;
                assert_eq!(old_score, 50);
                assert_eq!(new_score, 75);
                break;
            }
        }
    }

    assert!(found_event, "SCORECHGD event with 'set' reason not found");
}

/// Test: Emits UPDCHGD event on updater grant
#[test]
fn it_emits_updater_changed_event_on_grant() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);

    client.set_updater(&admin, &updater, &true);

    let events: Vec<(Address, Vec<Val>, Val)> = env.events().all();

    let mut found_event = false;
    for event in events.iter() {
        let topics = event.1.clone();
        let event_type: Symbol = topics.get(0).unwrap().into_val(&env);

        if event_type == symbol_short!("UPDCHGD") {
            let allowed: bool = event.2.into_val(&env);

            if allowed {
                found_event = true;

                let event_updater: Address = topics.get(1).unwrap().into_val(&env);
                assert_eq!(event_updater, updater);
                break;
            }
        }
    }

    assert!(found_event, "UPDCHGD event with allowed=true not found");
}

/// Test: Emits UPDCHGD event on updater revoke
#[test]
fn it_emits_updater_changed_event_on_revoke() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    client.set_updater(&admin, &updater, &false);

    let events: Vec<(Address, Vec<Val>, Val)> = env.events().all();

    let mut found_event = false;
    for event in events.iter() {
        let topics = event.1.clone();
        let event_type: Symbol = topics.get(0).unwrap().into_val(&env);

        if event_type == symbol_short!("UPDCHGD") {
            let allowed: bool = event.2.into_val(&env);

            if !allowed {
                found_event = true;

                let event_updater: Address = topics.get(1).unwrap().into_val(&env);
                assert_eq!(event_updater, updater);
                break;
            }
        }
    }

    assert!(found_event, "UPDCHGD event with allowed=false not found");
}

/// Test: Emits ADMINCHGD event on admin change
#[test]
fn it_emits_admin_changed_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let new_admin = Address::generate(&env);

    client.set_admin(&new_admin);

    let events: Vec<(Address, Vec<Val>, Val)> = env.events().all();

    let mut found_event = false;
    for event in events.iter() {
        let topics = event.1.clone();
        let event_type: Symbol = topics.get(0).unwrap().into_val(&env);

        if event_type == symbol_short!("ADMINCHGD") {
            let data_tuple: (Address, Address) = event.2.into_val(&env);
            let (old_admin, new_admin_event) = data_tuple;

            if old_admin != new_admin_event {
                found_event = true;
                assert_eq!(old_admin, admin);
                assert_eq!(new_admin_event, new_admin);
                break;
            }
        }
    }

    assert!(found_event, "ADMINCHGD event for admin change not found");
}

/// Test: Emits ADMINCHGD event on initial admin setup
#[test]
fn it_emits_admin_changed_event_on_initial_setup() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    client.set_admin(&admin);

    let events: Vec<(Address, Vec<Val>, Val)> = env.events().all();

    let mut found_event = false;
    for event in events.iter() {
        let topics = event.1.clone();
        let event_type: Symbol = topics.get(0).unwrap().into_val(&env);

        if event_type == symbol_short!("ADMINCHGD") {
            found_event = true;

            let data_tuple: (Address, Address) = event.2.into_val(&env);
            let (old_admin, new_admin_event) = data_tuple;

            assert_eq!(old_admin, admin);
            assert_eq!(new_admin_event, admin);
            break;
        }
    }

    assert!(found_event, "ADMINCHGD event not found");
}

// ============================================================================
// Admin Succession Tests (Feature #14)
// ============================================================================

/// Test: Supports multiple consecutive admin changes
#[test]
fn it_supports_admin_succession() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);

    client.set_admin(&admin1);
    assert_eq!(client.get_admin(), admin1);

    client.set_admin(&admin2);
    assert_eq!(client.get_admin(), admin2);

    client.set_admin(&admin3);
    assert_eq!(client.get_admin(), admin3);

    let final_admin = client.get_admin();
    assert_eq!(final_admin, admin3);
}

/// Test: Allows admin to set same admin (no-op case)
#[test]
fn it_allows_admin_to_set_same_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    client.set_admin(&admin);
    assert_eq!(client.get_admin(), admin);

    client.set_admin(&admin);
    assert_eq!(client.get_admin(), admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);
    assert!(client.is_updater(&updater));
}

/// Test: Preserves user scores during admin changes
#[test]
fn it_preserves_user_scores_during_admin_changes() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    client.set_admin(&admin1);

    let updater = Address::generate(&env);
    client.set_updater(&admin1, &updater, &true);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);

    client.set_score(&updater, &user1, &75);
    client.set_score(&updater, &user2, &50);
    client.set_score(&updater, &user3, &90);

    assert_eq!(client.get_score(&user1), 75);
    assert_eq!(client.get_score(&user2), 50);
    assert_eq!(client.get_score(&user3), 90);

    client.set_admin(&admin2);
    assert_eq!(client.get_admin(), admin2);

    assert_eq!(client.get_score(&user1), 75);
    assert_eq!(client.get_score(&user2), 50);
    assert_eq!(client.get_score(&user3), 90);
}

/// Test: Preserves updater permissions during admin changes
#[test]
fn it_preserves_updater_permissions_during_admin_changes() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    client.set_admin(&admin1);

    let updater1 = Address::generate(&env);
    let updater2 = Address::generate(&env);

    client.set_updater(&admin1, &updater1, &true);
    client.set_updater(&admin1, &updater2, &true);

    assert!(client.is_updater(&updater1));
    assert!(client.is_updater(&updater2));

    client.set_admin(&admin2);

    assert!(client.is_updater(&updater1));
    assert!(client.is_updater(&updater2));

    client.set_updater(&admin2, &updater1, &false);
    assert!(!client.is_updater(&updater1));
    assert!(client.is_updater(&updater2));

    let updater3 = Address::generate(&env);
    client.set_updater(&admin2, &updater3, &true);
    assert!(client.is_updater(&updater3));
}

/// Test: Revokes old admin permissions completely
#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn it_revokes_old_admin_permissions_completely() {
    let env = Env::default();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    env.mock_all_auths();
    client.set_admin(&admin1);
    client.set_admin(&admin2);

    assert_eq!(client.get_admin(), admin2);

    let updater = Address::generate(&env);
    client
        .mock_all_auths()
        .set_updater(&admin1, &updater, &true);
}

/// Test: New admin can perform all admin operations after transfer
#[test]
fn it_grants_new_admin_full_permissions() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    client.set_admin(&admin1);
    client.set_admin(&admin2);

    let updater = Address::generate(&env);
    client.set_updater(&admin2, &updater, &true);
    assert!(client.is_updater(&updater));

    client.set_updater(&admin2, &updater, &false);
    assert!(!client.is_updater(&updater));

    let admin3 = Address::generate(&env);
    client.set_admin(&admin3);
    assert_eq!(client.get_admin(), admin3);
}

// ============================================================================
// Zero Amount Tests (Feature #12)
// ============================================================================

/// Test: Allows zero amount increase
#[test]
fn it_allows_zero_increase_score() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    client.set_score(&updater, &user, &50);
    assert_eq!(client.get_score(&user), 50);

    client.increase_score(&updater, &user, &0);
    assert_eq!(client.get_score(&user), 50);
}

/// Test: Allows zero amount decrease
#[test]
fn it_allows_zero_decrease_score() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    client.set_score(&updater, &user, &50);
    assert_eq!(client.get_score(&user), 50);

    client.decrease_score(&updater, &user, &0);
    assert_eq!(client.get_score(&user), 50);
}

// ============================================================================
// Reentrancy Guard Tests
// ============================================================================

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_reentrancy_guard_rejects_reentrant_increase_score() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);
    client.set_score(&updater, &user, &50);

    // Lock the contract to simulate a reentrant call
    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&crate::storage::REENTRANCY_LOCK, &true);
    });

    client.increase_score(&updater, &user, &10);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_reentrancy_guard_rejects_reentrant_decrease_score() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);
    client.set_score(&updater, &user, &50);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&crate::storage::REENTRANCY_LOCK, &true);
    });

    client.decrease_score(&updater, &user, &10);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_reentrancy_guard_rejects_reentrant_set_score() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&crate::storage::REENTRANCY_LOCK, &true);
    });

    client.set_score(&updater, &user, &50);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_reentrancy_guard_rejects_reentrant_add_boost() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);
    client.set_score(&updater, &user, &40);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&crate::storage::REENTRANCY_LOCK, &true);
    });

    client.add_boost(&updater, &user, &10);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_reentrancy_guard_rejects_reentrant_remove_boost() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);
    client.set_score(&updater, &user, &50);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&crate::storage::REENTRANCY_LOCK, &true);
    });

    client.remove_boost(&updater, &user, &10);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_reentrancy_guard_rejects_reentrant_set_updater() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&crate::storage::REENTRANCY_LOCK, &true);
    });

    client.set_updater(&admin, &updater, &true);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_reentrancy_guard_rejects_reentrant_set_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let new_admin = Address::generate(&env);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&crate::storage::REENTRANCY_LOCK, &true);
    });

    client.set_admin(&new_admin);
}

/// Test: Mutating functions succeed when NOT reentrant
#[test]
fn test_reentrancy_guard_allows_normal_operations() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    // Normal operations should succeed
    client.set_score(&updater, &user, &50);
    assert_eq!(client.get_score(&user), 50);

    client.increase_score(&updater, &user, &10);
    assert_eq!(client.get_score(&user), 60);

    client.decrease_score(&updater, &user, &5);
    assert_eq!(client.get_score(&user), 55);

    client.add_boost(&updater, &user, &10);
    assert_eq!(client.get_score(&user), 65);

    client.remove_boost(&updater, &user, &5);
    assert_eq!(client.get_score(&user), 60);

    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);
    assert_eq!(client.get_admin(), new_admin);

    let new_updater = Address::generate(&env);
    client.set_updater(&new_admin, &new_updater, &true);
    assert!(client.is_updater(&new_updater));
}

/// Test: Guard is properly released after function completes
#[test]
fn test_reentrancy_guard_is_released_after_call() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    // First call should succeed
    client.set_score(&updater, &user, &50);

    // Lock should be released, second call should also succeed
    client.increase_score(&updater, &user, &10);
    assert_eq!(client.get_score(&user), 60);
}

/// Test: Allows setting score to current value
#[test]
fn it_allows_setting_score_to_current_value() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    let updater = Address::generate(&env);
    client.set_updater(&admin, &updater, &true);

    let user = Address::generate(&env);

    client.set_score(&updater, &user, &50);
    assert_eq!(client.get_score(&user), 50);

    client.set_score(&updater, &user, &50);
    assert_eq!(client.get_score(&user), 50);

    client.set_score(&updater, &user, &75);
    assert_eq!(client.get_score(&user), 75);

    client.set_score(&updater, &user, &75);
    assert_eq!(client.get_score(&user), 75);
}
