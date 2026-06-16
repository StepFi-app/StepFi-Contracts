use crate::{
    default_parameters, ParametersContract, ParametersContractClient, ParametersError,
    ProtocolParameters,
};
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup() -> (Env, ParametersContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ParametersContract, ());
    let client = ParametersContractClient::new(&env, &contract_id);
    let client: ParametersContractClient<'static> = unsafe { core::mem::transmute(client) };
    let admin = Address::generate(&env);

    (env, client, admin)
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
fn test_update_parameters() {
    let (_env, client, admin) = setup();
    client.initialize_defaults(&admin);

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

    client.update_parameters(&admin, &params);
    assert_eq!(client.get_parameters(), params);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_non_admin_cannot_update_parameters() {
    let (_env, client, admin) = setup();
    client.initialize_defaults(&admin);

    let intruder = Address::generate(&_env);
    let params = default_parameters();
    client.update_parameters(&intruder, &params);
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
