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
    client.confirm_multisig();

    (env, client, admin, s1, s2, s3)
}

fn signer_update_action(signers: Vec<Address>, threshold: u32) -> ProposalAction {
    ProposalAction::UpdateSigners(MultisigConfig { signers, threshold })
}

fn has_event(env: &Env, symbol: &str) -> bool {
    let events: soroban_sdk::Vec<(Address, soroban_sdk::Vec<soroban_sdk::Val>, soroban_sdk::Val)> =
        env.events().all();
    for e in events.iter() {
        let topic: soroban_sdk::Symbol = e.1.get_unchecked(0).into_val(env);
        if topic == soroban_sdk::Symbol::new(env, symbol) {
            return true;
        }
    }
    false
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
fn test_configure_multisig_two_step_propose_confirm_emits_prominent_events() {
    let (env, client, admin) = setup();
    client.initialize_defaults(&admin);

    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    let s3 = Address::generate(&env);
    let signers: Vec<Address> = vec![&env, s1.clone(), s2.clone(), s3.clone()];

    client.configure_multisig(&signers, &2u32);
    // Step 1 alone must NOT activate the committee — a single admin call can
    // no longer silently swap the set. (Event checks must run immediately
    // after the emitting call: each contract invocation resets the test event
    // log.)
    assert!(has_event(&env, "MSCONFPR"), "MSCONFPR event not found");
    assert_eq!(
        client.try_get_multisig(),
        Err(Ok(ParametersError::MultisigNotConfigured))
    );

    client.confirm_multisig();
    assert!(has_event(&env, "MSCONFIG"), "MSCONFIG event not found");
    let config = client.get_multisig();
    assert_eq!(config.threshold, 2);
    assert_eq!(config.signers.len(), 3);

    // The confirmed committee governs proposals as expected.
    let id = client.propose(&s1, &ProposalAction::UpdateParameters(default_parameters()));
    client.approve(&s2, &id);
    client.execute(&id);
    assert!(client.get_proposal(&id).executed);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")] // MultisigNotConfigured
fn test_confirm_multisig_without_propose_fails() {
    let (_env, client, admin) = setup();
    client.initialize_defaults(&admin);

    client.confirm_multisig();
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")] // MultisigAlreadyConfigured
fn test_configure_multisig_cannot_repropose_without_confirm() {
    let (env, client, admin) = setup();
    client.initialize_defaults(&admin);

    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    client.configure_multisig(&vec![&env, s1.clone(), s2.clone()], &2u32);
    client.configure_multisig(&vec![&env, s1, s2], &2u32);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")] // MultisigAlreadyConfigured
fn test_confirm_multisig_cannot_be_called_twice() {
    let (env, client, admin) = setup();
    client.initialize_defaults(&admin);

    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    client.configure_multisig(&vec![&env, s1, s2], &2u32);
    client.confirm_multisig();
    client.confirm_multisig();
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
        upgrade_delay_seconds: 86_400,
        late_fee_bps: 500,
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
    let (env, client, _admin, s1, s2, s3) = setup_multisig();

    let n1 = Address::generate(&env);
    let n2 = Address::generate(&env);
    let new_config = MultisigConfig {
        signers: vec![&env, n1.clone(), n2.clone()],
        threshold: 2,
    };

    let id = client.propose(&s1, &ProposalAction::UpdateSigners(new_config));
    // Signer-set changes need elevated quorum (threshold + 1 = 3 for 2-of-3).
    client.approve(&s2, &id);
    client.approve(&s3, &id);
    client.execute(&id);

    let config = client.get_multisig();
    assert_eq!(config.signers.len(), 2);
    assert!(config.signers.contains(&n1));
    assert!(config.signers.contains(&n2));
    // Old signers are no longer part of the committee.
    assert!(!config.signers.contains(&s1));

    // The proposal recorded a snapshot of the original eligible signer set.
    let proposal = client.get_proposal(&id);
    assert!(!proposal.invalidated);
    assert_eq!(proposal.snapshot.len(), 3);
    assert!(proposal.snapshot.contains(&s1));
    assert!(proposal.snapshot.contains(&s2));
    assert!(proposal.snapshot.contains(&s3));
}

#[test]
#[should_panic(expected = "Error(Contract, #17)")] // ThresholdNotMet
fn test_update_signers_requires_elevated_quorum() {
    let (env, client, _admin, s1, s2, _s3) = setup_multisig();

    let n1 = Address::generate(&env);
    let n2 = Address::generate(&env);
    let id = client.propose(
        &s1,
        &signer_update_action(vec![&env, n1, n2], 2),
    );
    // s1 (proposer auto-approves) + s2 = 2 approvals, but signer changes in a
    // 2-of-3 set require threshold + 1 = 3.
    client.approve(&s2, &id);
    client.execute(&id);
}

#[test]
fn test_update_signers_with_elevated_quorum_executes() {
    let (env, client, _admin, s1, s2, s3) = setup_multisig();

    let n1 = Address::generate(&env);
    let n2 = Address::generate(&env);
    let id = client.propose(
        &s1,
        &signer_update_action(vec![&env, n1.clone(), n2.clone()], 2),
    );
    client.approve(&s2, &id);
    client.approve(&s3, &id);
    client.execute(&id);

    let config = client.get_multisig();
    assert!(config.signers.contains(&n1));
    assert!(config.signers.contains(&n2));
    assert!(!config.signers.contains(&s1));
    assert!(!config.signers.contains(&s2));
}

#[test]
fn test_signer_change_quorum_capped_at_full_committee_for_unanimous_set() {
    let (env, client, admin) = setup();
    client.initialize_defaults(&admin);

    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    client.configure_multisig(&vec![&env, s1.clone(), s2.clone()], &2u32);
    client.confirm_multisig();

    let n1 = Address::generate(&env);
    let n2 = Address::generate(&env);
    let id = client.propose(
        &s1,
        &signer_update_action(vec![&env, n1, n2], 2),
    );
    // 2-of-2 committee: elevated quorum min(2 + 1, 2) = 2 = unanimity. One
    // approval (the proposer) is not enough.
    assert!(client.try_execute(&id).is_err());
    client.approve(&s2, &id);
    client.execute(&id);
    assert_eq!(client.get_multisig().signers.len(), 2);
}

#[test]
fn test_stale_approval_from_removed_signer_is_never_counted() {
    let (env, client, _admin, s1, s2, s3) = setup_multisig();

    let params = ProtocolParameters {
        min_guarantee_percent: 30,
        ..default_parameters()
    };

    // s1 proposes a parameter change and s2 approves it — 2-of-3 reached.
    let id = client.propose(&s1, &ProposalAction::UpdateParameters(params.clone()));
    client.approve(&s2, &id);

    // The signer set changes and removes s2. s2 consents to its own removal,
    // so the change proposal reaches the elevated quorum of 3.
    let new_set = MultisigConfig {
        signers: vec![&env, s1.clone(), s3.clone()],
        threshold: 2,
    };
    let change_id = client.propose(&s2, &ProposalAction::UpdateSigners(new_set));
    client.approve(&s1, &change_id);
    client.approve(&s3, &change_id);
    client.execute(&change_id);
    assert!(!client.get_multisig().signers.contains(&s2));

    // s2's stale approval on the parameter proposal must NOT count.
    let proposal = client.get_proposal(&id);
    assert_eq!(proposal.approvals.len(), 2); // s1 + s2
    assert!(proposal.snapshot.contains(&s2)); // it was a signer at propose time
    assert!(client.try_execute(&id).is_err());
    // The exploit would have rewritten parameters; post-fix they are intact.
    assert_eq!(client.get_parameters(), default_parameters());
}

#[test]
#[should_panic(expected = "Error(Contract, #19)")] // ApproverNotEligible
fn test_execute_rejects_proposal_whose_approver_was_removed() {
    let (env, client, _admin, s1, s2, s3) = setup_multisig();

    let id = client.propose(&s1, &ProposalAction::UpdateParameters(default_parameters()));
    client.approve(&s2, &id); // s2's approval will go stale.

    let new_set = MultisigConfig {
        signers: vec![&env, s1.clone(), s3.clone()],
        threshold: 2,
    };
    let change_id = client.propose(&s3, &ProposalAction::UpdateSigners(new_set));
    client.approve(&s1, &change_id);
    client.approve(&s2, &change_id);
    client.execute(&change_id);

    // execute() re-validates every historical approver: s2 is gone.
    client.execute(&id);
}

#[test]
#[should_panic(expected = "Error(Contract, #19)")] // ApproverNotEligible
fn test_newly_added_signer_cannot_approve_old_proposal() {
    let (env, client, _admin, s1, s2, s3) = setup_multisig();

    let id = client.propose(&s1, &ProposalAction::UpdateParameters(default_parameters()));

    // Replace the committee with a brand-new signer s4 (elevated quorum).
    let s4 = Address::generate(&env);
    let new_set = MultisigConfig {
        signers: vec![&env, s3.clone(), s4.clone()],
        threshold: 2,
    };
    let change_id = client.propose(&s2, &ProposalAction::UpdateSigners(new_set));
    client.approve(&s1, &change_id);
    client.approve(&s3, &change_id);
    client.execute(&change_id);

    // s4 is a current signer but was not a member at proposal time.
    client.approve(&s4, &id);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")] // NotSigner
fn test_removed_signer_cannot_approve_after_removal() {
    let (env, client, _admin, s1, s2, s3) = setup_multisig();

    let id = client.propose(&s1, &ProposalAction::UpdateParameters(default_parameters()));

    let new_set = MultisigConfig {
        signers: vec![&env, s1.clone(), s3.clone()],
        threshold: 2,
    };
    let change_id = client.propose(&s2, &ProposalAction::UpdateSigners(new_set));
    client.approve(&s1, &change_id);
    client.approve(&s3, &change_id);
    client.execute(&change_id);

    // s2 is no longer a current signer — approve() rejects it outright.
    client.approve(&s2, &id);
}

#[test]
fn test_in_flight_signer_set_proposals_are_invalidated_on_signer_change() {
    let (env, client, _admin, s1, s2, s3) = setup_multisig();

    // In-flight signer-set proposal A (never executes).
    let a_id = client.propose(
        &s1,
        &signer_update_action(vec![&env, s1.clone(), s2.clone()], 2),
    );

    // Signer-set proposal B executes first.
    let b_config = MultisigConfig {
        signers: vec![&env, s2.clone(), s3.clone()],
        threshold: 2,
    };
    let b_id = client.propose(&s2, &ProposalAction::UpdateSigners(b_config.clone()));
    client.approve(&s1, &b_id);
    client.approve(&s3, &b_id);
    client.execute(&b_id);

    // The signer change voids every other in-flight signer-set proposal.
    // (Event checks must run immediately after the emitting call: each
    // contract invocation resets the test event log.)
    assert!(has_event(&env, "PROPIVLD"), "PROPIVLD event not found");

    // Proposal A is stale now: voided, cannot collect approval or execute.
    assert!(client.get_proposal(&a_id).invalidated);
    assert!(client.try_execute(&a_id).is_err());

    // The executed proposal that drove the change is NOT flagged invalidated.
    let b = client.get_proposal(&b_id);
    assert!(b.executed);
    assert!(!b.invalidated);
    assert_eq!(client.get_multisig(), b_config);
}

#[test]
#[should_panic(expected = "Error(Contract, #18)")] // ProposalInvalidated
fn test_approve_rejects_invalidated_signer_set_proposal() {
    let (env, client, _admin, s1, s2, s3) = setup_multisig();

    let a_id = client.propose(
        &s1,
        &signer_update_action(vec![&env, s1.clone(), s2.clone()], 2),
    );

    let b_id = client.propose(
        &s2,
        &signer_update_action(vec![&env, s2.clone(), s3.clone()], 2),
    );
    client.approve(&s1, &b_id);
    client.approve(&s3, &b_id);
    client.execute(&b_id);

    // a_id was invalidated by the signer change — a fresh approval must fail.
    client.approve(&s2, &a_id);
}

#[test]
fn test_parameter_proposals_survive_signer_change_but_stale_approvals_revalidated() {
    let (env, client, _admin, s1, s2, s3) = setup_multisig();

    // Parameter proposal by s1.
    let p_id = client.propose(&s1, &ProposalAction::UpdateParameters(default_parameters()));

    // Remove s1 via signer change (elevated quorum: all three approve).
    let new_set = MultisigConfig {
        signers: vec![&env, s2.clone(), s3.clone()],
        threshold: 2,
    };
    let change_id = client.propose(&s1, &ProposalAction::UpdateSigners(new_set));
    client.approve(&s2, &change_id);
    client.approve(&s3, &change_id);
    client.execute(&change_id);

    // Parameter proposals are NOT invalidated by a signer change...
    assert!(!client.get_proposal(&p_id).invalidated);
    // ...but the stale proposer approval (s1, now removed) blocks execution.
    assert!(client.try_execute(&p_id).is_err());

    // A still-eligible signer can add an approval, yet the stale s1 approval
    // means the proposal can never execute — signers re-propose to move on.
    client.approve(&s2, &p_id);
    assert!(client.try_execute(&p_id).is_err());
    assert_eq!(client.get_parameters(), default_parameters());
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

    assert!(has_event(&env, "CONTRACTUPGRADED"), "CONTRACTUPGRADED event not found");
}

#[test]
fn test_three_of_three_with_full_committee_approval() {
    let (env, client, admin) = setup();
    client.initialize_defaults(&admin);

    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    let s3 = Address::generate(&env);
    client.configure_multisig(&vec![&env, s1.clone(), s2.clone(), s3.clone()], &3u32);
    client.confirm_multisig();

    let id = client.propose(&s1, &ProposalAction::UpdateParameters(default_parameters()));
    client.approve(&s2, &id);
    assert!(client.try_execute(&id).is_err());

    client.approve(&s3, &id);
    client.execute(&id);
    assert!(client.get_proposal(&id).executed);
}
