use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    Address, Env, IntoVal, String,
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

    // Vendor starts as Pending, not active
    assert!(!client.is_active(&vendor));

    // get_vendor_info automatically unwraps on success in the test client
    let info = client.get_vendor_info(&vendor);
    assert_eq!(info.name, name);
    assert_eq!(info.registration_date, 1000000);
    assert_eq!(info.status, types::VendorStatus::Pending);
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

    // Vendor starts as Pending (not active)
    assert!(!client.is_active(&vendor));
    assert_eq!(
        client.get_vendor_info(&vendor).status,
        types::VendorStatus::Pending
    );

    // Approve vendor
    client.approve_vendor(&admin, &vendor);
    assert!(client.is_active(&vendor));
    assert_eq!(
        client.get_vendor_info(&vendor).status,
        types::VendorStatus::Approved
    );

    // Suspend vendor
    client.suspend_vendor(&admin, &vendor);
    assert!(!client.is_active(&vendor));
    assert_eq!(
        client.get_vendor_info(&vendor).status,
        types::VendorStatus::Suspended
    );

    // Reactivate via legacy activate_vendor
    client.activate_vendor(&admin, &vendor);
    assert!(client.is_active(&vendor));
    assert_eq!(
        client.get_vendor_info(&vendor).status,
        types::VendorStatus::Approved
    );
}

#[test]
fn test_set_vendor_status() {
    let env = Env::default();
    let (client, admin, vendor) = setup(&env);
    env.mock_all_auths();

    let name = String::from_str(&env, "Quasar Goods");
    client.register_vendor(&admin, &vendor, &name);

    // Vendor starts as Pending (not active)
    assert!(!client.is_active(&vendor));
    assert_eq!(
        client.get_vendor_info(&vendor).status,
        types::VendorStatus::Pending
    );

    // Deactivate via set_vendor_status (false → Suspended)
    client.set_vendor_status(&admin, &vendor, &false);
    assert!(!client.is_active(&vendor));
    assert_eq!(
        client.get_vendor_info(&vendor).status,
        types::VendorStatus::Suspended
    );

    // Reactivate via set_vendor_status (true → Approved)
    client.set_vendor_status(&admin, &vendor, &true);
    assert!(client.is_active(&vendor));
    assert_eq!(
        client.get_vendor_info(&vendor).status,
        types::VendorStatus::Approved
    );

    // Non-admin must be rejected
    let fake_admin = Address::generate(&env);
    let res = client.try_set_vendor_status(&fake_admin, &vendor, &false);
    assert!(res.is_err());
}

#[test]
fn test_approve_vendor_sets_status_to_approved() {
    let env = Env::default();
    let (client, admin, vendor) = setup(&env);
    env.mock_all_auths();

    let name = String::from_str(&env, "Approve Me");
    client.register_vendor(&admin, &vendor, &name);

    assert_eq!(
        client.get_vendor_info(&vendor).status,
        types::VendorStatus::Pending
    );

    client.approve_vendor(&admin, &vendor);
    assert_eq!(
        client.get_vendor_info(&vendor).status,
        types::VendorStatus::Approved
    );
    assert!(client.is_active(&vendor));
}

#[test]
fn test_approve_vendor_rejects_non_pending() {
    let env = Env::default();
    let (client, admin, vendor) = setup(&env);
    env.mock_all_auths();

    let name = String::from_str(&env, "Already Approved");
    client.register_vendor(&admin, &vendor, &name);
    client.approve_vendor(&admin, &vendor);

    // Approving an already approved vendor should fail
    let res = client.try_approve_vendor(&admin, &vendor);
    assert_eq!(res, Err(Ok(Error::VendorNotPending)));
}

#[test]
fn test_suspend_vendor_sets_status_to_suspended() {
    let env = Env::default();
    let (client, admin, vendor) = setup(&env);
    env.mock_all_auths();

    let name = String::from_str(&env, "Suspend Me");
    client.register_vendor(&admin, &vendor, &name);
    client.approve_vendor(&admin, &vendor);

    assert!(client.is_active(&vendor));

    client.suspend_vendor(&admin, &vendor);
    assert_eq!(
        client.get_vendor_info(&vendor).status,
        types::VendorStatus::Suspended
    );
    assert!(!client.is_active(&vendor));
}

#[test]
fn test_suspended_vendor_can_be_approved_again() {
    let env = Env::default();
    let (client, admin, vendor) = setup(&env);
    env.mock_all_auths();

    let name = String::from_str(&env, "Back and Forth");
    client.register_vendor(&admin, &vendor, &name);
    client.approve_vendor(&admin, &vendor);
    client.suspend_vendor(&admin, &vendor);

    assert!(!client.is_active(&vendor));

    // Re-approve via legacy activate_vendor
    client.activate_vendor(&admin, &vendor);
    assert!(client.is_active(&vendor));
    assert_eq!(
        client.get_vendor_info(&vendor).status,
        types::VendorStatus::Approved
    );
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
        env.storage().instance().set(&types::DataKey::Locked, &true);
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
        env.storage().instance().set(&types::DataKey::Locked, &true);
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
        env.storage().instance().set(&types::DataKey::Locked, &true);
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
        env.storage().instance().set(&types::DataKey::Locked, &true);
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
    client.register_vendor(&admin, &vendor, &name);

    client.deactivate_vendor(&admin, &vendor);

    client.activate_vendor(&admin, &vendor);

    client.set_vendor_status(&admin, &vendor, &false);
}

#[test]
fn test_reentrancy_guard_is_released_after_call() {
    let env = Env::default();
    let (client, admin, vendor) = setup(&env);
    env.mock_all_auths();

    // First call should succeed
    let name = String::from_str(&env, "Test");
    client.register_vendor(&admin, &vendor, &name);

    // Lock should be released, second call should also succeed
    client.deactivate_vendor(&admin, &vendor);
    client.activate_vendor(&admin, &vendor);
}

#[test]
fn test_reentrancy_guard_on_approve_vendor() {
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
        env.storage().instance().set(&types::DataKey::Locked, &true);
    });

    let res = client.try_approve_vendor(&admin, &vendor);
    assert_eq!(res, Err(Ok(Error::ReentrancyDetected)));
}

#[test]
fn test_reentrancy_guard_on_suspend_vendor() {
    let env = Env::default();
    let contract_id = env.register(VendorRegistryContract, ());
    let client = VendorRegistryContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let vendor = Address::generate(&env);

    client.initialize(&admin);
    env.mock_all_auths();

    let name = String::from_str(&env, "Test");
    client.register_vendor(&admin, &vendor, &name);
    client.approve_vendor(&admin, &vendor);

    env.as_contract(&contract_id, || {
        env.storage().instance().set(&types::DataKey::Locked, &true);
    });

    let res = client.try_suspend_vendor(&admin, &vendor);
    assert_eq!(res, Err(Ok(Error::ReentrancyDetected)));
}

#[test]
#[should_panic(expected = "Error(Contract")] // non-admin upgrade rejected
fn test_upgrade_rejected_for_non_admin() {
    let env = Env::default();
    let contract_id = env.register(VendorRegistryContract, ());
    let client = VendorRegistryContractClient::new(&env, &contract_id);

    let wasm_hash = soroban_sdk::BytesN::from_array(&env, &[0u8; 32]);
    client.upgrade(&wasm_hash);
}

#[test]
fn test_admin_upgrade_increments_version_and_emits_event() {
    let env = Env::default();
    let (client, _admin, _vendor) = setup(&env);
    env.mock_all_auths();

    assert_eq!(client.get_version(), 1u32);
    let wasm_hash = env.deployer().upload_contract_wasm(soroban_sdk::Bytes::from_slice(
        &env,
        include_bytes!("../../../contracts/test-fixtures/contract.wasm"),
    ));
    client.upgrade(&wasm_hash);

    let events: soroban_sdk::Vec<(soroban_sdk::Address, soroban_sdk::Vec<soroban_sdk::Val>, soroban_sdk::Val)> = env.events().all();
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
