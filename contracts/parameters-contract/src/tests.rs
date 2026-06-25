use crate::{
    default_parameters, MultisigConfig, ParametersContract, ParametersContractClient,
    ParametersError, ProposalAction, ProtocolParameters,
};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    vec, Address, Env, IntoVal, Vec,
};

const SEVEN_DAYS: u64 = 604_800;

fn setup() -> (Env, ParametersContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ParametersContract, ());
    let client = ParametersContractClient::new(&env, &contract_id);
    let client: ParametersContractClient<'static> = unsafe { core::mem::transmute(client) };
    let admin = Address::generate(&env);

    (env, client, admin)
}

fn setup_multisig() -> (
    Env,
    ParametersContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
) {
    let (env, client, admin) = setup();
    client.initialize_defaults(&admin);

    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    let s3 = Address::generate(&env);
    let signers: Vec<Address> = vec![&env, s1.clone(), s2.clone(), s3.clone()];
    client.configure_multisig(&signers, &2u32);

    (env, client, admin, s1, s2, s3)
}

#[test]
fn test_initialize_defaults() {
    let (_env, client, admin) = setup();
    client.initialize_defaults(&admin);

    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_parameters(), default_parameters());
}

#[test]
fn test_get_admin_before_initialize_returns_typed_error() {
    let (_env, client, _admin) = setup();

    assert_eq!(
        client.try_get_admin(),
        Err(Ok(ParametersError::NotInitialized))
    );
}

#[test]
fn test_get_parameters_before_initialize_returns_typed_error() {
    let (_env, client, _admin) = setup();

    assert_eq!(
        client.try_get_parameters(),
        Err(Ok(ParametersError::NotInitialized))
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_invalid_parameters_rejected() {
    let (_env, client, admin) = setup();

    let params = ProtocolParameters {
        min_guarantee_percent: 0,
        ..default_parameters()
    };

    client.initialize(&admin, &params);
}

#[test]
fn test_configure_multisig_stores_committee() {
    let (env, client, admin, s1, s2, s3) = setup_multisig();
    let _ = (admin, env);

    let config = client.get_multisig();
    assert_eq!(config.threshold, 2);
    assert_eq!(config.signers.len(), 3);
    assert!(config.signers.contains(&s1));
    assert!(config.signers.contains(&s2));
    assert!(config.signers.contains(&s3));
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")] // InvalidThreshold
fn test_configure_multisig_rejects_threshold_below_two() {
    let (env, client, admin) = setup();
    client.initialize_defaults(&admin);

    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    let signers: Vec<Address> = vec![&env, s1, s2];
    client.configure_multisig(&signers, &1u32);
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")] // InvalidThreshold
fn test_configure_multisig_rejects_threshold_above_signer_count() {
    let (env, client, admin) = setup();
    client.initialize_defaults(&admin);

    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    let signers: Vec<Address> = vec![&env, s1, s2];
    client.configure_multisig(&signers, &3u32);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")] // DuplicateSigner
fn test_configure_multisig_rejects_duplicate_signers() {
    let (env, client, admin) = setup();
    client.initialize_defaults(&admin);

    let s1 = Address::generate(&env);
    let signers: Vec<Address> = vec![&env, s1.clone(), s1];
    client.configure_multisig(&signers, &2u32);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")] // MultisigAlreadyConfigured
fn test_configure_multisig_only_once() {
    let (env, client, _admin, s1, s2, s3) = setup_multisig();
    let signers: Vec<Address> = vec![&env, s1, s2, s3];
    client.configure_multisig(&signers, &2u32);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")] // NotSigner
fn test_propose_rejects_non_signer() {
    let (env, client, _admin, _s1, _s2, _s3) = setup_multisig();
    let intruder = Address::generate(&env);
    client.propose(&intruder, &ProposalAction::SetAdmin(intruder.clone()));
}

#[test]
fn test_update_parameters_two_of_three_workflow() {
    let (_env, client, _admin, s1, s2, _s3) = setup_multisig();

    let params = ProtocolParameters {
        min_guarantee_percent: 30,
        min_reputation_threshold: 70,
        full_repayment_reward: 12,
        default_penalty: 25,
        large_loan_threshold: 7_500,
        large_loan_default_penalty: 40,
        base_interest_bps: 900,
        grace_period_seconds: 86_400,
    };

    let id = client.propose(&s1, &ProposalAction::UpdateParameters(params.clone()));
    // Proposer counts as first approval; one more reaches the 2-of-3 threshold.
    client.approve(&s2, &id);
    client.execute(&id);

    assert_eq!(client.get_parameters(), params);
    assert!(client.get_proposal(&id).executed);
}

#[test]
#[should_panic(expected = "Error(Contract, #17)")] // ThresholdNotMet
fn test_execute_before_threshold_met_is_rejected() {
    let (_env, client, _admin, s1, _s2, _s3) = setup_multisig();

    let id = client.propose(&s1, &ProposalAction::UpdateParameters(default_parameters()));
    client.execute(&id);
}

#[test]
#[should_panic(expected = "Error(Contract, #16)")] // DuplicateSignature
fn test_duplicate_signature_rejected() {
    let (_env, client, _admin, s1, _s2, _s3) = setup_multisig();

    let id = client.propose(&s1, &ProposalAction::UpdateParameters(default_parameters()));
    client.approve(&s1, &id);
}

#[test]
#[should_panic(expected = "Error(Contract, #15)")] // ProposalAlreadyExecuted
fn test_cannot_execute_twice() {
    let (_env, client, _admin, s1, s2, _s3) = setup_multisig();

    let id = client.propose(&s1, &ProposalAction::UpdateParameters(default_parameters()));
    client.approve(&s2, &id);
    client.execute(&id);
    client.execute(&id);
}

#[test]
#[should_panic(expected = "Error(Contract, #14)")]
fn test_proposal_expires_after_seven_days() {
    let (env, client, _admin, s1, s2, _s3) = setup_multisig();

    let id = client.propose(&s1, &ProposalAction::UpdateParameters(default_parameters()));

    env.ledger().set_timestamp(SEVEN_DAYS + 1);
    client.approve(&s2, &id);
}

#[test]
#[should_panic(expected = "Error(Contract, #14)")]
fn test_expired_proposal_cannot_execute() {
    let (env, client, _admin, s1, s2, _s3) = setup_multisig();

    let id = client.propose(&s1, &ProposalAction::UpdateParameters(default_parameters()));
    client.approve(&s2, &id);

    env.ledger().set_timestamp(SEVEN_DAYS + 1);
    client.execute(&id);
}

#[test]
fn test_set_admin_via_proposal() {
    let (env, client, _admin, s1, s2, _s3) = setup_multisig();
    let new_admin = Address::generate(&env);

    let id = client.propose(&s1, &ProposalAction::SetAdmin(new_admin.clone()));
    client.approve(&s2, &id);
    client.execute(&id);

    assert_eq!(client.get_admin(), new_admin);
}

#[test]
fn test_update_signers_via_proposal() {
    let (env, client, _admin, s1, s2, _s3) = setup_multisig();

    let n1 = Address::generate(&env);
    let n2 = Address::generate(&env);
    let new_config = MultisigConfig {
        signers: vec![&env, n1.clone(), n2.clone()],
        threshold: 2,
    };

    let id = client.propose(&s1, &ProposalAction::UpdateSigners(new_config));
    client.approve(&s2, &id);
    client.execute(&id);

    let config = client.get_multisig();
    assert_eq!(config.signers.len(), 2);
    assert!(config.signers.contains(&n1));
    assert!(config.signers.contains(&n2));
    // Old signers are no longer part of the committee.
    assert!(!config.signers.contains(&s1));
}

#[test]
fn test_upgrade_via_proposal_increments_version() {
    let (env, client, _admin, s1, s2, _s3) = setup_multisig();
    assert_eq!(client.get_version(), 1u32);

    let wasm_hash = env.deployer().upload_contract_wasm(soroban_sdk::Bytes::from_slice(
        &env,
        include_bytes!("../../../contracts/test-fixtures/contract.wasm"),
    ));

    let id = client.propose(&s1, &ProposalAction::Upgrade(wasm_hash));
    client.approve(&s2, &id);
    client.execute(&id);

    let events: soroban_sdk::Vec<(Address, soroban_sdk::Vec<soroban_sdk::Val>, soroban_sdk::Val)> =
        env.events().all();
    let mut found = false;
    for e in events.iter() {
        let topic: soroban_sdk::Symbol = e.1.get_unchecked(0).into_val(&env);
        if topic == soroban_sdk::Symbol::new(&env, "CONTRACTUPGRADED") {
            found = true;
            break;
        }
    }
    assert!(found, "CONTRACTUPGRADED event not found");
}

#[test]
fn test_three_of_three_with_full_committee_approval() {
    let (env, client, admin) = setup();
    client.initialize_defaults(&admin);

    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    let s3 = Address::generate(&env);
    client.configure_multisig(&vec![&env, s1.clone(), s2.clone(), s3.clone()], &3u32);

    let id = client.propose(&s1, &ProposalAction::UpdateParameters(default_parameters()));
    client.approve(&s2, &id);
    assert!(client.try_execute(&id).is_err());

    client.approve(&s3, &id);
    client.execute(&id);
    assert!(client.get_proposal(&id).executed);
}
