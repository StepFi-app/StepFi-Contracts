#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String,
};

/// Helper function to set up the environment, contract, and test addresses.
fn setup<'a>(env: &'a Env) -> (VendorRegistryContractClient<'a>, Address, Address) {
    // Using the updated register syntax
    let contract_id = env.register(VendorRegistryContract, ());
    let client = VendorRegistryContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let vendor = Address::generate(env);

    // Initialize the contract with the admin
    client.initialize(&admin);

    (client, admin, vendor)
}

#[test]
fn test_initialization() {
    let env = Env::default();
    // Using the updated register syntax
    let contract_id = env.register(VendorRegistryContract, ());
    let client = VendorRegistryContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    // Initial setup succeeds
    client.initialize(&admin);

    // Initializing twice throws an error
    let res = client.try_initialize(&admin);
    assert!(res.is_err());
}

#[test]
fn test_get_vendor_count_before_initialize_returns_typed_error() {
    let env = Env::default();
    let contract_id = env.register(VendorRegistryContract, ());
    let client = VendorRegistryContractClient::new(&env, &contract_id);

    assert_eq!(
        client.try_get_vendor_count(),
        Err(Ok(Error::NotInitialized))
    );
}

#[test]
fn test_registration_flow() {
    let env = Env::default();
    let (client, admin, vendor) = setup(&env);

    env.mock_all_auths();
    // Advance ledger time to test registration_date
    env.ledger().with_mut(|l| l.timestamp = 1000000);

    let name = String::from_str(&env, "Galaxy Tech Supplies");
    client.register_vendor(&admin, &vendor, &name);

    assert!(client.is_active(&vendor));

    // get_vendor_info automatically unwraps on success in the test client
    let info = client.get_vendor_info(&vendor);
    assert_eq!(info.name, name);
    assert_eq!(info.registration_date, 1000000);
    assert_eq!(info.active, true);
    assert_eq!(info.total_sales, 0);
    assert_eq!(client.get_vendor_count(), 1);
}

#[test]
fn test_duplicate_prevention() {
    let env = Env::default();
    let (client, admin, vendor) = setup(&env);
    env.mock_all_auths();

    let name = String::from_str(&env, "Stellar Books");
    client.register_vendor(&admin, &vendor, &name);

    // Registering the exact same address again should fail
    let res = client.try_register_vendor(&admin, &vendor, &name);
    assert!(res.is_err());
}

#[test]
fn test_admin_only_access() {
    let env = Env::default();
    let (client, _admin, vendor) = setup(&env);
    env.mock_all_auths();

    // Create a rogue admin address
    let fake_admin = Address::generate(&env);
    let name = String::from_str(&env, "Rogue Vendor");

    // Use try_register_vendor to catch the expected Unauthorized error
    let res = client.try_register_vendor(&fake_admin, &vendor, &name);
    assert!(res.is_err());
}

#[test]
fn test_activation_and_deactivation() {
    let env = Env::default();
    let (client, admin, vendor) = setup(&env);
    env.mock_all_auths();

    let name = String::from_str(&env, "Nebula Cafe");
    client.register_vendor(&admin, &vendor, &name);

    assert!(client.is_active(&vendor));

    // Deactivate vendor
    client.deactivate_vendor(&admin, &vendor);
    assert!(!client.is_active(&vendor));
    assert_eq!(client.get_vendor_info(&vendor).active, false);

    // Activate vendor
    client.activate_vendor(&admin, &vendor);
    assert!(client.is_active(&vendor));
    assert_eq!(client.get_vendor_info(&vendor).active, true);
}

#[test]
fn test_set_vendor_status() {
    let env = Env::default();
    let (client, admin, vendor) = setup(&env);
    env.mock_all_auths();

    let name = String::from_str(&env, "Quasar Goods");
    client.register_vendor(&admin, &vendor, &name);

    // Vendor starts active
    assert!(client.is_active(&vendor));

    // Deactivate via set_vendor_status
    client.set_vendor_status(&admin, &vendor, &false);
    assert!(!client.is_active(&vendor));

    // Reactivate via set_vendor_status
    client.set_vendor_status(&admin, &vendor, &true);
    assert!(client.is_active(&vendor));

    // Non-admin must be rejected
    let fake_admin = Address::generate(&env);
    let res = client.try_set_vendor_status(&fake_admin, &vendor, &false);
    assert!(res.is_err());
}

// ============================================================================
// Reentrancy Guard Tests
// ============================================================================

#[test]
fn test_reentrancy_guard_on_register_vendor() {
    let env = Env::default();
    let contract_id = env.register(VendorRegistryContract, ());
    let client = VendorRegistryContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let vendor = Address::generate(&env);

    client.initialize(&admin);
    env.mock_all_auths();

    // Lock the contract
    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&types::DataKey::Locked, &true);
    });

    let name = String::from_str(&env, "Test");
    let res = client.try_register_vendor(&admin, &vendor, &name);
    assert_eq!(res, Err(Ok(Error::ReentrancyDetected)));
}

#[test]
fn test_reentrancy_guard_on_deactivate_vendor() {
    let env = Env::default();
    let contract_id = env.register(VendorRegistryContract, ());
    let client = VendorRegistryContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let vendor = Address::generate(&env);

    client.initialize(&admin);
    env.mock_all_auths();

    let name = String::from_str(&env, "Test");
    client.register_vendor(&admin, &vendor, &name);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&types::DataKey::Locked, &true);
    });

    let res = client.try_deactivate_vendor(&admin, &vendor);
    assert_eq!(res, Err(Ok(Error::ReentrancyDetected)));
}

#[test]
fn test_reentrancy_guard_on_activate_vendor() {
    let env = Env::default();
    let contract_id = env.register(VendorRegistryContract, ());
    let client = VendorRegistryContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let vendor = Address::generate(&env);

    client.initialize(&admin);
    env.mock_all_auths();

    let name = String::from_str(&env, "Test");
    client.register_vendor(&admin, &vendor, &name);
    client.deactivate_vendor(&admin, &vendor);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&types::DataKey::Locked, &true);
    });

    let res = client.try_activate_vendor(&admin, &vendor);
    assert_eq!(res, Err(Ok(Error::ReentrancyDetected)));
}

#[test]
fn test_reentrancy_guard_on_set_vendor_status() {
    let env = Env::default();
    let contract_id = env.register(VendorRegistryContract, ());
    let client = VendorRegistryContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let vendor = Address::generate(&env);

    client.initialize(&admin);
    env.mock_all_auths();

    let name = String::from_str(&env, "Test");
    client.register_vendor(&admin, &vendor, &name);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&types::DataKey::Locked, &true);
    });

    let res = client.try_set_vendor_status(&admin, &vendor, &false);
    assert_eq!(res, Err(Ok(Error::ReentrancyDetected)));
}

#[test]
fn test_reentrancy_guard_allows_normal_operations() {
    let env = Env::default();
    let (client, admin, vendor) = setup(&env);
    env.mock_all_auths();

    // Normal operations should still succeed
    let name = String::from_str(&env, "Test");
    assert!(client.register_vendor(&admin, &vendor, &name).is_ok());

    assert!(client.deactivate_vendor(&admin, &vendor).is_ok());

    assert!(client.activate_vendor(&admin, &vendor).is_ok());

    assert!(client
        .set_vendor_status(&admin, &vendor, &false)
        .is_ok());
}

#[test]
fn test_reentrancy_guard_is_released_after_call() {
    let env = Env::default();
    let (client, admin, vendor) = setup(&env);
    env.mock_all_auths();

    // First call should succeed
    let name = String::from_str(&env, "Test");
    assert!(client.register_vendor(&admin, &vendor, &name).is_ok());

    // Lock should be released, second call should also succeed
    assert!(client.deactivate_vendor(&admin, &vendor).is_ok());
    assert!(client.activate_vendor(&admin, &vendor).is_ok());
}
