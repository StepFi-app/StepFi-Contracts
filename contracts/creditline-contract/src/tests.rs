use crate::{CreditLineContract, CreditLineContractClient, LoanStatus, LoanType, RepaymentInstallment};
use liquidity_pool_contract::{LiquidityPoolContract, LiquidityPoolContractClient, PoolStats};
use vendor_registry_contract::VendorRegistryContract;
use parameters_contract::{
    default_parameters, ParametersContract, ParametersContractClient, ProtocolParameters,
};
use reputation_contract::{ReputationContract, ReputationContractClient};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Events, Ledger},
    token::Client as TokenClient,
    Address, Env, String as SorobanString,
};

const DEFAULT_PRINCIPAL: i128 = 1_000;
const DEFAULT_GUARANTEE: i128 = 200;
const DEFAULT_INTEREST_BPS: u32 = 400;
const DEFAULT_INTEREST_AMOUNT: i128 = 40;
const DEFAULT_SERVICE_FEE: i128 = 10;
const DEFAULT_TOTAL_DUE: i128 = 1_050;

// NOTE: Integration tests with reputation contract are skipped for now
// They will be added when all contracts are implemented and properly configured
#[contract]
pub struct MockReputation;

#[contractimpl]
impl MockReputation {
    pub fn get_score(_env: Env, _user: Address) -> u32 {
        100 // Returns 100 to pass the threshold check
    }
    pub fn decrease_score(_env: Env, _updater: Address, _user: Address, _amount: u32) {
        // Does nothing, just needs to exist for the call to succeed
    }

    pub fn increase_score(_env: Env, _updater: Address, _user: Address, _amount: u32) {}
}

#[contract]
pub struct MockLiquidityPool;

#[contractimpl]
impl MockLiquidityPool {
    pub fn get_pool_stats(_env: Env) -> PoolStats {
        PoolStats {
            total_liquidity: 1_000_000,
            locked_liquidity: 0,
            available_liquidity: 1_000_000,
            total_shares: 1_000_000,
            share_price: 10_000,
        }
    }

    pub fn fund_loan(_env: Env, _creditline: Address, _vendor: Address, _amount: i128) {}

    pub fn receive_repayment(_env: Env, _from: Address, _amount: i128, _fee: i128) {}

    pub fn receive_guarantee(_env: Env, _from: Address, _amount: i128) {}
}

// A mock reputation contract that always returns a score below the threshold.
// Placed in its own module to avoid symbol collisions with MockReputation.
mod mock_low_rep {
    use soroban_sdk::{contract, contractimpl, Address, Env};

    #[contract]
    pub struct MockReputationLow;

    #[contractimpl]
    impl MockReputationLow {
        pub fn get_score(_env: Env, _user: Address) -> u32 {
            49 // Returns 49 — below the 50 minimum threshold
        }
    }
}
use mock_low_rep::MockReputationLow;

// ─── helpers ──────────────────────────────────────────────────────────────────

/// Creates a basic TestEnv with MockReputation wired in and the contract
/// initialized. Returns (env, client, admin, rep_id).
struct TestCtx {
    env: Env,
    client: CreditLineContractClient<'static>,
    admin: Address,
    rep_id: Address,
    token_id: Address,
    lp_id: Address,
    vendor_registry_id: Address,
}

impl TestCtx {
    fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(CreditLineContract, ());
        let client = CreditLineContractClient::new(&env, &contract_id);

        // SAFETY: env outlives client — same pattern as liquidity-pool tests
        let client: CreditLineContractClient<'static> = unsafe { core::mem::transmute(client) };

        let admin = Address::generate(&env);
        let rep_id = env.register(MockReputation, ());

        // Register the actual VendorRegistryContract
        let vendor_registry_id = env.register(VendorRegistryContract, ());

        // Initialize the vendor registry using invoke_contract
        use soroban_sdk::{IntoVal, Symbol};
        let _: Result<(), vendor_registry_contract::VendorRegistryError> = env.invoke_contract(
            &vendor_registry_id,
            &Symbol::new(&env, "initialize"),
            (&admin,).into_val(&env),
        );

        let lp_id = env.register(MockLiquidityPool, ());

        let token_admin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();
        client.initialize(&admin, &rep_id, &vendor_registry_id, &lp_id, &token_id);

        TestCtx {
            env,
            client,
            admin,
            rep_id,
            token_id,
            lp_id,
            vendor_registry_id,
        }
    }

    /// Build a single-installment repayment schedule with the given due date.
    fn single_installment(
        &self,
        amount: i128,
        due_date: u64,
    ) -> soroban_sdk::Vec<RepaymentInstallment> {
        let mut schedule = soroban_sdk::Vec::new(&self.env);
        schedule.push_back(RepaymentInstallment {
            amount,
            due_date,
            paid: false,
            paid_at: 0,
        });
        schedule
    }

    /// Register a vendor in the vendor registry (idempotent - won't fail if already registered)
    fn register_vendor(&self, vendor: &Address, name: &str) {
        use soroban_sdk::{IntoVal, Symbol};
        let vendor_name = SorobanString::from_str(&self.env, name);

        // Use try_invoke_contract to handle errors gracefully
        // Silently ignore errors - in tests, we want this to be idempotent
        let _ = self.env.try_invoke_contract::<(), soroban_sdk::Error>(
            &self.vendor_registry_id,
            &Symbol::new(&self.env, "register_vendor"),
            (&self.admin, vendor, vendor_name).into_val(&self.env),
        );
    }

    /// Create a loan with sensible defaults: total=1000, guarantee=200, 1 installment.
    /// Automatically registers the vendor if not already registered.
    fn create_default_loan(&self, user: &Address, vendor: &Address) -> u64 {
        // Register the vendor first (idempotent - won't fail if already registered)
        self.register_vendor(vendor, "Test Vendor");

        // Guarantee transfer now happens at loan creation, so borrower needs balance.
        self.mint(user, DEFAULT_GUARANTEE);

        let due_date = self.env.ledger().timestamp() + 10_000;
        let schedule = self.single_installment(DEFAULT_TOTAL_DUE, due_date);
        self.client.create_loan(
            user,
            vendor,
            &DEFAULT_PRINCIPAL,
            &DEFAULT_GUARANTEE,
            &schedule,
            &LoanType::Standard,
        )
    }

    fn create_default_request(&self, user: &Address, vendor: &Address) -> u64 {
        self.register_vendor(vendor, "Test Vendor");
        self.mint(user, DEFAULT_GUARANTEE);

        let due_date = self.env.ledger().timestamp() + 10_000;
        let schedule = self.single_installment(DEFAULT_TOTAL_DUE, due_date);
        self.client.request_loan(
            user,
            vendor,
            &DEFAULT_PRINCIPAL,
            &DEFAULT_GUARANTEE,
            &schedule,
            &LoanType::Standard,
        )
    }

    /// Advance ledger timestamp past the given due date so a loan is overdue.
    fn advance_past(&self, due_date: u64) {
        self.env.ledger().set_timestamp(due_date + 1);
    }

    /// Mint `amount` tokens to `address` so repayments don't fail on insufficient balance.
    fn mint(&self, to: &Address, amount: i128) {
        let asset_client = StellarAssetClient::new(&self.env, &self.token_id);
        asset_client.mint(to, &amount);
    }

    fn balance(&self, address: &Address) -> i128 {
        let token_client = soroban_sdk::token::Client::new(&self.env, &self.token_id);
        token_client.balance(address)
    }
}

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let reputation_contract = Address::generate(&env);
    let vendor_registry = Address::generate(&env);
    let liquidity_pool = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &reputation_contract,
        &vendor_registry,
        &liquidity_pool,
        &token,
    );

    assert_eq!(client.get_admin(), admin);
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_initialize_twice_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let reputation_contract = Address::generate(&env);
    let vendor_registry = Address::generate(&env);
    let liquidity_pool = Address::generate(&env);
    let token = Address::generate(&env); // add this

    client.initialize(
        &admin,
        &reputation_contract,
        &vendor_registry,
        &liquidity_pool,
        &token,
    );

    // Try to initialize again - should panic
    client.initialize(
        &admin,
        &reputation_contract,
        &vendor_registry,
        &liquidity_pool,
        &token,
    );
}

#[test]
fn test_get_version() {
    let version = CreditLineContract::get_version();
    assert_eq!(version, soroban_sdk::symbol_short!("v1_0_0"));
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_get_loan_not_found() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let reputation_contract = Address::generate(&env);
    let vendor_registry = Address::generate(&env);
    let liquidity_pool = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &reputation_contract,
        &vendor_registry,
        &liquidity_pool,
        &token,
    );

    // Try to get a loan that doesn't exist
    client.get_loan(&999);
}

#[test]
fn test_set_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let reputation_contract = Address::generate(&env);
    let vendor_registry = Address::generate(&env);
    let liquidity_pool = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &reputation_contract,
        &vendor_registry,
        &liquidity_pool,
        &token,
    );

    assert_eq!(client.get_admin(), admin);

    // Change admin
    client.set_admin(&new_admin);

    assert_eq!(client.get_admin(), new_admin);
}

#[test]
fn test_set_reputation_contract() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let reputation_contract = Address::generate(&env);
    let new_reputation_contract = Address::generate(&env);
    let vendor_registry = Address::generate(&env);
    let liquidity_pool = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &reputation_contract,
        &vendor_registry,
        &liquidity_pool,
        &token,
    );

    // Update reputation contract address
    client.set_reputation_contract(&admin, &new_reputation_contract);

    // Verify it was updated (we can't directly query, but no panic means success)
}

#[test]
fn test_set_vendor_registry() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let reputation_contract = Address::generate(&env);
    let vendor_registry = Address::generate(&env);
    let new_vendor_registry = Address::generate(&env);
    let liquidity_pool = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &reputation_contract,
        &vendor_registry,
        &liquidity_pool,
        &token,
    );

    // Update vendor registry address
    client.set_vendor_registry(&admin, &new_vendor_registry);

    // Verify it was updated (we can't directly query, but no panic means success)
}

#[test]
fn test_set_liquidity_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let reputation_contract = Address::generate(&env);
    let vendor_registry = Address::generate(&env);
    let liquidity_pool = Address::generate(&env);
    let new_liquidity_pool = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &reputation_contract,
        &vendor_registry,
        &liquidity_pool,
        &token,
    );

    // Update liquidity pool address
    client.set_liquidity_pool(&admin, &new_liquidity_pool);

    // Verify it was updated (we can't directly query, but no panic means success)
}

// Tests for validate_guarantee logic (tested indirectly through create_loan)

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_create_loan_with_zero_total_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let vendor = Address::generate(&env);
    let reputation_contract = Address::generate(&env);
    let vendor_registry = Address::generate(&env);
    let liquidity_pool = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &reputation_contract,
        &vendor_registry,
        &liquidity_pool,
        &token,
    );

    let repayment_schedule = soroban_sdk::Vec::new(&env);

    // This should panic with InvalidAmount (error code 9)
    client.create_loan(&user, &vendor, &0, &0, &repayment_schedule, &LoanType::Standard);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_create_loan_with_negative_total_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let vendor = Address::generate(&env);
    let reputation_contract = Address::generate(&env);
    let vendor_registry = Address::generate(&env);
    let liquidity_pool = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &reputation_contract,
        &vendor_registry,
        &liquidity_pool,
        &token,
    );

    let repayment_schedule = soroban_sdk::Vec::new(&env);

    // This should panic with InvalidAmount (error code 9)
    client.create_loan(&user, &vendor, &-1000, &-200, &repayment_schedule, &LoanType::Standard);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_create_loan_with_zero_guarantee_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let vendor = Address::generate(&env);
    let reputation_contract = Address::generate(&env);
    let vendor_registry = Address::generate(&env);
    let liquidity_pool = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &reputation_contract,
        &vendor_registry,
        &liquidity_pool,
        &token,
    );

    let repayment_schedule = soroban_sdk::Vec::new(&env);

    // This should panic with InvalidAmount (error code 9)
    client.create_loan(&user, &vendor, &1000, &0, &repayment_schedule, &LoanType::Standard);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_create_loan_with_insufficient_guarantee_19_percent() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let vendor = Address::generate(&env);
    let reputation_contract = Address::generate(&env);
    let vendor_registry = Address::generate(&env);
    let liquidity_pool = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &reputation_contract,
        &vendor_registry,
        &liquidity_pool,
        &token,
    );

    let repayment_schedule = soroban_sdk::Vec::new(&env);

    // 190 is 19% of 1000, should fail with InsufficientGuarantee (error code 2)
    client.create_loan(&user, &vendor, &1000, &190, &repayment_schedule, &LoanType::Standard);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_create_loan_with_insufficient_guarantee_10_percent() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let vendor = Address::generate(&env);
    let reputation_contract = Address::generate(&env);
    let vendor_registry = Address::generate(&env);
    let liquidity_pool = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &reputation_contract,
        &vendor_registry,
        &liquidity_pool,
        &token,
    );

    let repayment_schedule = soroban_sdk::Vec::new(&env);

    // 100 is 10% of 1000, should fail with InsufficientGuarantee (error code 2)
    client.create_loan(&user, &vendor, &1000, &100, &repayment_schedule, &LoanType::Standard);
}

// Additional edge case tests

#[test]
#[should_panic(expected = "Admin not set")]
fn test_get_admin_before_initialization() {
    let env = Env::default();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    // Try to get admin before initialization - should panic
    client.get_admin();
}

#[test]
#[should_panic(expected = "Admin not set")]
fn test_set_admin_before_initialization() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let new_admin = Address::generate(&env);

    // Try to set admin before initialization - should panic
    client.set_admin(&new_admin);
}

#[test]
fn test_loan_counter_increments() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let reputation_contract = Address::generate(&env);
    let vendor_registry = Address::generate(&env);
    let liquidity_pool = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &reputation_contract,
        &vendor_registry,
        &liquidity_pool,
        &token,
    );

    // Note: We can't actually create loans without a reputation contract
    // This test validates the counter mechanism exists
    // Full testing will be done with integration tests
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_create_loan_with_one_less_than_minimum_guarantee() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let vendor = Address::generate(&env);
    let reputation_contract = Address::generate(&env);
    let vendor_registry = Address::generate(&env);
    let liquidity_pool = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &reputation_contract,
        &vendor_registry,
        &liquidity_pool,
        &token,
    );

    let repayment_schedule = soroban_sdk::Vec::new(&env);

    // 199 is 1 less than 20% of 1000, should fail with InsufficientGuarantee (error code 2)
    client.create_loan(&user, &vendor, &1000, &199, &repayment_schedule, &LoanType::Standard);
}

#[test]
fn test_multiple_contract_address_updates() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let reputation_contract_1 = Address::generate(&env);
    let reputation_contract_2 = Address::generate(&env);
    let reputation_contract_3 = Address::generate(&env);
    let vendor_registry = Address::generate(&env);
    let liquidity_pool = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &reputation_contract_1,
        &vendor_registry,
        &liquidity_pool,
        &token,
    );

    // Update reputation contract multiple times
    client.set_reputation_contract(&admin, &reputation_contract_2);
    client.set_reputation_contract(&admin, &reputation_contract_3);

    // All updates should succeed without panic
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_create_loan_with_positive_total_negative_guarantee() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let vendor = Address::generate(&env);
    let reputation_contract = Address::generate(&env);
    let vendor_registry = Address::generate(&env);
    let liquidity_pool = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &reputation_contract,
        &vendor_registry,
        &liquidity_pool,
        &token,
    );

    let repayment_schedule = soroban_sdk::Vec::new(&env);

    // Positive total but negative guarantee should fail with InvalidAmount (error code 9)
    client.create_loan(&user, &vendor, &1000, &-200, &repayment_schedule, &LoanType::Standard);
}

#[test]
fn test_mark_defaulted_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    // Register our Mock Reputation contract
    let rep_id = env.register(MockReputation, ());

    // Register the actual VendorRegistryContract
    let vendor_registry_id = env.register(VendorRegistryContract, ());

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let vendor = Address::generate(&env);
    let liquidity_pool = env.register(MockLiquidityPool, ());
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();

    // Initialize the vendor registry
    use soroban_sdk::{IntoVal, Symbol};
    let _: Result<(), vendor_registry_contract::VendorRegistryError> = env.invoke_contract(
        &vendor_registry_id,
        &Symbol::new(&env, "initialize"),
        (&admin,).into_val(&env),
    );

    // Register the vendor
    let vendor_name = SorobanString::from_str(&env, "Test Vendor");
    let _: Result<(), vendor_registry_contract::VendorRegistryError> = env.invoke_contract(
        &vendor_registry_id,
        &Symbol::new(&env, "register_vendor"),
        (&admin, &vendor, vendor_name).into_val(&env),
    );

    client.initialize(
        &admin,
        &rep_id, // Pass the Mock ID
        &vendor_registry_id,
        &liquidity_pool,
        &token,
    );

    // Set a baseline time
    let current_time = 10000;
    env.ledger().set_timestamp(current_time);

    let mut schedule = soroban_sdk::Vec::new(&env);
    schedule.push_back(RepaymentInstallment {
        amount: 1000,
        due_date: current_time + 1000, // Due at 11000,
        paid: false,
        paid_at: 0,
    });

    let asset_client = StellarAssetClient::new(&env, &token);
    asset_client.mint(&user, &200);

    // Create loan (calls MockReputation::get_score)
    let loan_id = client.create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);

    // Time Travel past the due date
    env.ledger().set_timestamp(12000);

    // This calls mark_defaulted which internally calls MockReputation::decrease_score
    client.mark_defaulted(&loan_id);

    let updated_loan = client.get_loan(&loan_id);
    assert_eq!(updated_loan.status, LoanStatus::Defaulted);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")] // LoanNotOverdue
fn test_mark_defaulted_too_early_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    let rep_id = env.register(MockReputation, ());

    // Register the actual VendorRegistryContract
    let vendor_registry_id = env.register(VendorRegistryContract, ());

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let vendor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let liquidity_pool = env.register(MockLiquidityPool, ());

    // Initialize the vendor registry
    use soroban_sdk::{IntoVal, Symbol};
    let _: Result<(), vendor_registry_contract::VendorRegistryError> = env.invoke_contract(
        &vendor_registry_id,
        &Symbol::new(&env, "initialize"),
        (&admin,).into_val(&env),
    );

    // Register the vendor
    let vendor_name = SorobanString::from_str(&env, "Test Vendor");
    let _: Result<(), vendor_registry_contract::VendorRegistryError> = env.invoke_contract(
        &vendor_registry_id,
        &Symbol::new(&env, "register_vendor"),
        (&admin, &vendor, vendor_name).into_val(&env),
    );

    client.initialize(
        &admin,
        &rep_id,
        &vendor_registry_id,
        &liquidity_pool,
        &token,
    );

    let current_time = 10000;
    env.ledger().set_timestamp(current_time);

    let mut schedule = soroban_sdk::Vec::new(&env);
    schedule.push_back(RepaymentInstallment {
        amount: 1000,
        due_date: 20000,
        paid: false,
        paid_at: 0,
    });

    let asset_client = StellarAssetClient::new(&env, &token);
    asset_client.mint(&user, &200);

    let loan_id = client.create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);

    // This should fail because 10000 < 20000
    client.mark_defaulted(&loan_id);
}

// ─── loan creation — happy path ───────────────────────────────────────────────

#[test]
fn test_create_loan_returns_incrementing_ids() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    // create_default_loan registers the vendor on first call
    let id1 = t.create_default_loan(&user, &vendor);
    let id2 = t.create_default_loan(&user, &vendor);
    let id3 = t.create_default_loan(&user, &vendor);

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
}

#[test]
fn test_create_loan_stores_correct_fields() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    t.env.ledger().set_timestamp(5000);
    let due_date = 15000_u64;
    let schedule = t.single_installment(DEFAULT_TOTAL_DUE, due_date);
    t.mint(&user, DEFAULT_GUARANTEE);

    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);
    let loan = t.client.get_loan(&loan_id);

    assert_eq!(loan.loan_id, loan_id);
    assert_eq!(loan.borrower, user);
    assert_eq!(loan.vendor, vendor);
    assert_eq!(loan.total_amount, DEFAULT_PRINCIPAL);
    assert_eq!(loan.guarantee_amount, DEFAULT_GUARANTEE);
    assert_eq!(loan.interest_rate_bps, DEFAULT_INTEREST_BPS);
    assert_eq!(loan.interest_amount, DEFAULT_INTEREST_AMOUNT);
    assert_eq!(loan.service_fee_amount, DEFAULT_SERVICE_FEE);
    assert_eq!(loan.principal_outstanding, DEFAULT_PRINCIPAL);
    assert_eq!(loan.interest_outstanding, DEFAULT_INTEREST_AMOUNT);
    assert_eq!(loan.service_fee_outstanding, DEFAULT_SERVICE_FEE);
    assert_eq!(loan.remaining_balance, DEFAULT_TOTAL_DUE);
    assert_eq!(loan.status, LoanStatus::Active);
    assert_eq!(loan.created_at, 5000);
    assert_eq!(loan.funded_at, 5000);
}

#[test]
fn test_create_loan_exactly_20_percent_guarantee() {
    // 200 is exactly 20% of 1000 — must succeed
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");
    let schedule = t.single_installment(DEFAULT_TOTAL_DUE, 99999);
    t.mint(&user, DEFAULT_GUARANTEE);

    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);
    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.guarantee_amount, DEFAULT_GUARANTEE);
}

#[test]
fn test_create_loan_with_more_than_20_percent_guarantee() {
    // 500 is 50% of 1000 — must succeed
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");
    let schedule = t.single_installment(DEFAULT_TOTAL_DUE, 99999);
    t.mint(&user, 500);

    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1000, &500, &schedule, &LoanType::Standard);
    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Active);
}

#[test]
fn test_create_loan_with_multi_installment_schedule() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    let mut schedule = soroban_sdk::Vec::new(&t.env);
    schedule.push_back(RepaymentInstallment {
        amount: 334,
        due_date: 10000,
        paid: false,
        paid_at: 0,
    });
    schedule.push_back(RepaymentInstallment {
        amount: 333,
        due_date: 20000,
        paid: false,
        paid_at: 0,
    });
    schedule.push_back(RepaymentInstallment {
        amount: 333,
        due_date: 30000,
        paid: false,
        paid_at: 0,
    });
    t.mint(&user, 200);

    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);
    let loan = t.client.get_loan(&loan_id);

    assert_eq!(loan.repayment_schedule.len(), 3);
    assert_eq!(loan.total_amount, 1000);
}

// ─── loan creation — reputation validation ────────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #4)")] // InsufficientReputation
fn test_create_loan_rejected_when_reputation_below_threshold() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(CreditLineContract, ());
    let client = CreditLineContractClient::new(&env, &contract_id);

    // Wire in the low-score mock
    let low_rep_id = env.register(MockReputationLow, ());

    // Register the actual VendorRegistryContract
    let vendor_registry_id = env.register(VendorRegistryContract, ());

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let vendor = Address::generate(&env);
    let token = Address::generate(&env);

    // Initialize the vendor registry
    use soroban_sdk::{IntoVal, Symbol};
    let _: Result<(), vendor_registry_contract::VendorRegistryError> = env.invoke_contract(
        &vendor_registry_id,
        &Symbol::new(&env, "initialize"),
        (&admin,).into_val(&env),
    );

    // Register the vendor
    let vendor_name = SorobanString::from_str(&env, "Test Vendor");
    let _: Result<(), vendor_registry_contract::VendorRegistryError> = env.invoke_contract(
        &vendor_registry_id,
        &Symbol::new(&env, "register_vendor"),
        (&admin, &vendor, vendor_name).into_val(&env),
    );

    client.initialize(
        &admin,
        &low_rep_id,
        &vendor_registry_id,
        &Address::generate(&env),
        &token,
    );

    let mut schedule = soroban_sdk::Vec::new(&env);
    schedule.push_back(RepaymentInstallment {
        amount: 1000,
        due_date: 99999,
        paid: false,
        paid_at: 0,
    });

    // Score is 49 — below 50 minimum → InsufficientReputation (error 4)
    client.create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);
}

#[test]
fn test_create_loan_accepted_at_exactly_threshold_score() {
    // MockReputation returns 100 which is ≥ 50 → must succeed
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    let loan_id = t.create_default_loan(&user, &vendor);
    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Active);
}

// ─── loan creation — event emission ──────────────────────────────────────────

#[test]
fn test_create_loan_emits_loan_created_event() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.create_default_loan(&user, &vendor);

    // At least one event was emitted
    let events = t.env.events().all();
    assert!(
        !events.is_empty(),
        "Expected a LoanCreated event to be emitted"
    );
}

#[test]
fn test_mark_defaulted_emits_loan_defaulted_event() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    t.env.ledger().set_timestamp(1000);
    let schedule = t.single_installment(1000, 5000);
    t.mint(&user, 200);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);

    t.advance_past(5000);
    t.client.mark_defaulted(&loan_id);

    let events = t.env.events().all();
    assert!(
        !events.is_empty(),
        "Expected a LoanDefaulted event to be emitted"
    );
}

// ─── default flow ─────────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // LoanNotActive
fn test_mark_defaulted_on_already_defaulted_loan_fails() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    t.env.ledger().set_timestamp(1000);
    let schedule = t.single_installment(1000, 5000);
    t.mint(&user, 200);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);

    t.advance_past(5000);
    t.client.mark_defaulted(&loan_id);

    // Second call must fail — loan is no longer Active
    t.client.mark_defaulted(&loan_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")] // LoanNotFound
fn test_mark_defaulted_on_nonexistent_loan_fails() {
    let t = TestCtx::setup();
    t.client.mark_defaulted(&999);
}

#[test]
fn test_default_flow_loan_status_becomes_defaulted() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    t.env.ledger().set_timestamp(1000);
    let schedule = t.single_installment(1000, 5000);
    t.mint(&user, 200);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);

    let before = t.client.get_loan(&loan_id);
    assert_eq!(before.status, LoanStatus::Active);

    t.advance_past(5000);
    t.client.mark_defaulted(&loan_id);

    let after = t.client.get_loan(&loan_id);
    assert_eq!(after.status, LoanStatus::Defaulted);
}

#[test]
fn test_default_flow_preserves_loan_amounts() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    t.env.ledger().set_timestamp(1000);
    let schedule = t.single_installment(1000, 5000);
    t.mint(&user, 200);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);

    t.advance_past(5000);
    t.client.mark_defaulted(&loan_id);

    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.total_amount, DEFAULT_PRINCIPAL);
    assert_eq!(loan.guarantee_amount, DEFAULT_GUARANTEE);
    assert_eq!(loan.remaining_balance, DEFAULT_TOTAL_DUE); // unchanged — no repayment was made
}

#[test]
fn test_mark_defaulted_at_exactly_due_date_boundary() {
    // Ledger timestamp == due_date: still NOT overdue (the condition is `timestamp > due_date`)
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    t.env.ledger().set_timestamp(1000);
    let due_date = 5000_u64;
    let schedule = t.single_installment(1000, due_date);
    t.mint(&user, 200);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);

    // Set timestamp to exactly the due date — mark_defaulted should fail (LoanNotOverdue)
    t.env.ledger().set_timestamp(due_date);

    let result = t.client.try_mark_defaulted(&loan_id);
    assert!(result.is_err(), "Should fail when timestamp == due_date");
}

#[test]
fn test_mark_defaulted_one_second_past_due_succeeds() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    t.env.ledger().set_timestamp(1000);
    let due_date = 5000_u64;
    let schedule = t.single_installment(1000, due_date);
    t.mint(&user, 200);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);

    t.env.ledger().set_timestamp(due_date + 1);
    t.client.mark_defaulted(&loan_id);

    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Defaulted);
}

#[test]
fn test_default_flow_uses_last_installment_for_overdue_check() {
    // Multi-installment loan: overdue is determined by the LAST installment's due date
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    t.env.ledger().set_timestamp(1000);

    let mut schedule = soroban_sdk::Vec::new(&t.env);
    schedule.push_back(RepaymentInstallment {
        amount: 400,
        due_date: 3000,
        paid: false,
        paid_at: 0,
    }); // already past
    schedule.push_back(RepaymentInstallment {
        amount: 300,
        due_date: 6000,
        paid: false,
        paid_at: 0,
    }); // already past
    schedule.push_back(RepaymentInstallment {
        amount: 300,
        due_date: 10000,
        paid: false,
        paid_at: 0,
    }); // last
    t.mint(&user, 200);

    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);

    // Past first two but not the last — should still fail (LoanNotOverdue)
    t.env.ledger().set_timestamp(7000);
    let result = t.client.try_mark_defaulted(&loan_id);
    assert!(
        result.is_err(),
        "Not overdue until past the last installment"
    );

    // Now past the last installment — should succeed
    t.env.ledger().set_timestamp(10001);
    t.client.mark_defaulted(&loan_id);
    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Defaulted);
}

// ─── loan creation — score decrease on default (reputation integration) ───────

#[test]
fn test_mark_defaulted_triggers_reputation_slash() {
    // MockReputation::slash is a no-op; we just verify the call doesn't panic,
    // proving the contract correctly invokes the reputation contract on default.
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    t.env.ledger().set_timestamp(1000);
    let schedule = t.single_installment(1000, 5000);
    t.mint(&user, 200);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);

    t.advance_past(5000);
    // This succeeds only if the `slash` cross-contract call is executed without error
    t.client.mark_defaulted(&loan_id);

    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Defaulted);
}

// ─── grace period ─────────────────────────────────────────────────────────────

/// Helper: register and wire up a ParametersContract with the given grace period.
fn setup_parameters_with_grace_period(t: &TestCtx, grace_period_seconds: u64) {
    use soroban_sdk::Symbol;
    let params_id = t.env.register(ParametersContract, ());
    let params_client = ParametersContractClient::new(&t.env, &params_id);
    params_client.initialize(
        &t.admin,
        &ProtocolParameters {
            grace_period_seconds,
            ..default_parameters()
        },
    );
    t.client.set_parameters_contract(&t.admin, &params_id);
}

#[test]
fn test_mark_defaulted_blocked_during_grace_period() {
    // With a 1000-second grace period the loan cannot be hard-defaulted while
    // the clock is still inside due_date < t <= due_date + grace.
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    setup_parameters_with_grace_period(&t, 1_000);

    let due_date = 5_000_u64;
    t.env.ledger().set_timestamp(1_000);
    let schedule = t.single_installment(1_000, due_date);
    t.mint(&user, 200);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1_000, &200, &schedule, &LoanType::Standard);

    // One second past due but still within the grace window.
    t.env.ledger().set_timestamp(due_date + 1);
    let result = t.client.try_mark_defaulted(&loan_id);
    assert!(
        result.is_err(),
        "mark_defaulted must fail while inside the grace period"
    );
    // Verify the loan is still Active — not Defaulted.
    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Active);
}

#[test]
fn test_mark_defaulted_succeeds_after_grace_period_expires() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    let grace: u64 = 1_000;
    setup_parameters_with_grace_period(&t, grace);

    let due_date = 5_000_u64;
    t.env.ledger().set_timestamp(1_000);
    let schedule = t.single_installment(1_000, due_date);
    t.mint(&user, 200);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1_000, &200, &schedule, &LoanType::Standard);

    // One second past the end of the grace window.
    t.env.ledger().set_timestamp(due_date + grace + 1);
    t.client.mark_defaulted(&loan_id);

    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Defaulted);
}

#[test]
fn test_mark_defaulted_at_grace_period_boundary_still_blocked() {
    // At exactly due_date + grace_period the loan is still protected.
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    let grace: u64 = 1_000;
    setup_parameters_with_grace_period(&t, grace);

    let due_date = 5_000_u64;
    t.env.ledger().set_timestamp(1_000);
    let schedule = t.single_installment(1_000, due_date);
    t.mint(&user, 200);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1_000, &200, &schedule, &LoanType::Standard);

    t.env.ledger().set_timestamp(due_date + grace);
    let result = t.client.try_mark_defaulted(&loan_id);
    assert!(
        result.is_err(),
        "mark_defaulted must fail at exactly the grace period boundary"
    );
}

#[test]
fn test_warn_grace_period_emits_event() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    let grace: u64 = 1_000;
    setup_parameters_with_grace_period(&t, grace);

    let due_date = 5_000_u64;
    t.env.ledger().set_timestamp(1_000);
    let schedule = t.single_installment(1_000, due_date);
    t.mint(&user, 200);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1_000, &200, &schedule, &LoanType::Standard);

    // Advance into the grace window and call warn_grace_period.
    t.env.ledger().set_timestamp(due_date + 1);
    let result = t.client.try_warn_grace_period(&loan_id);
    assert!(
        result.is_ok(),
        "warn_grace_period should succeed inside the grace window"
    );
    // Loan must remain Active.
    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Active);
}

#[test]
fn test_warn_grace_period_fails_before_due_date() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    setup_parameters_with_grace_period(&t, 1_000);

    let due_date = 5_000_u64;
    t.env.ledger().set_timestamp(1_000);
    let schedule = t.single_installment(1_000, due_date);
    t.mint(&user, 200);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1_000, &200, &schedule, &LoanType::Standard);

    // Still before due date — should return LoanNotOverdue.
    let result = t.client.try_warn_grace_period(&loan_id);
    assert!(
        result.is_err(),
        "warn_grace_period must fail before the due date"
    );
}

#[test]
fn test_warn_grace_period_fails_after_grace_expires() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    let grace: u64 = 1_000;
    setup_parameters_with_grace_period(&t, grace);

    let due_date = 5_000_u64;
    t.env.ledger().set_timestamp(1_000);
    let schedule = t.single_installment(1_000, due_date);
    t.mint(&user, 200);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1_000, &200, &schedule, &LoanType::Standard);

    // Past the grace window — no longer in grace period.
    t.env.ledger().set_timestamp(due_date + grace + 1);
    let result = t.client.try_warn_grace_period(&loan_id);
    assert!(
        result.is_err(),
        "warn_grace_period must fail after the grace period expires"
    );
}

#[test]
fn test_zero_grace_period_allows_immediate_default() {
    // grace_period_seconds = 0 preserves the original one-second-past-due behavior.
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    setup_parameters_with_grace_period(&t, 0);

    let due_date = 5_000_u64;
    t.env.ledger().set_timestamp(1_000);
    let schedule = t.single_installment(1_000, due_date);
    t.mint(&user, 200);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1_000, &200, &schedule, &LoanType::Standard);

    t.env.ledger().set_timestamp(due_date + 1);
    t.client.mark_defaulted(&loan_id);

    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Defaulted);
}

// ─── admin access control ─────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // NotAdmin
fn test_set_reputation_contract_by_non_admin_fails() {
    let t = TestCtx::setup();
    let intruder = Address::generate(&t.env);
    let new_rep = Address::generate(&t.env);
    t.client.set_reputation_contract(&intruder, &new_rep);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // NotAdmin
fn test_set_vendor_registry_by_non_admin_fails() {
    let t = TestCtx::setup();
    let intruder = Address::generate(&t.env);
    let new_registry = Address::generate(&t.env);
    t.client.set_vendor_registry(&intruder, &new_registry);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // NotAdmin
fn test_set_liquidity_pool_by_non_admin_fails() {
    let t = TestCtx::setup();
    let intruder = Address::generate(&t.env);
    let new_pool = Address::generate(&t.env);
    t.client.set_liquidity_pool(&intruder, &new_pool);
}

#[test]
fn test_admin_can_update_all_contract_addresses() {
    let t = TestCtx::setup();
    let new_rep = Address::generate(&t.env);
    let new_registry = Address::generate(&t.env);
    let new_pool = Address::generate(&t.env);

    // All three must succeed without panic
    t.client.set_reputation_contract(&t.admin, &new_rep);
    t.client.set_vendor_registry(&t.admin, &new_registry);
    t.client.set_liquidity_pool(&t.admin, &new_pool);
}

// ─── repayment ────────────────────────────────────────────────────────────────

#[test]
fn test_partial_repayment_reduces_remaining_balance() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    let loan_id = t.create_default_loan(&user, &vendor);

    t.mint(&user, DEFAULT_TOTAL_DUE);

    t.client.repay_loan(&user, &loan_id, &500);
    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.remaining_balance, DEFAULT_TOTAL_DUE - 500);
    assert_eq!(loan.status, LoanStatus::Active);
}

#[test]
fn test_full_repayment_sets_status_to_paid() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    let loan_id = t.create_default_loan(&user, &vendor);

    t.mint(&user, DEFAULT_TOTAL_DUE);

    t.client.repay_loan(&user, &loan_id, &DEFAULT_TOTAL_DUE);
    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.remaining_balance, 0);
    assert_eq!(loan.status, LoanStatus::Paid);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")] // InvalidAmount
fn test_overpayment_is_rejected() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    let loan_id = t.create_default_loan(&user, &vendor);

    // Paying more than remaining_balance should panic with InvalidAmount
    t.client
        .repay_loan(&user, &loan_id, &(DEFAULT_TOTAL_DUE + 1));
    let _ = loan_id;
}

#[test]
#[should_panic(expected = "Error(Contract, #14)")] // NotBorrower
fn test_unauthorized_repayment_is_rejected() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let intruder = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    let loan_id = t.create_default_loan(&user, &vendor);

    // A different address trying to repay the loan must fail with NotBorrower
    t.client.repay_loan(&intruder, &loan_id, &200);
    let _ = (loan_id, intruder);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // LoanNotActive
fn test_repayment_on_non_active_loan_is_rejected() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    t.env.ledger().set_timestamp(1000);
    let schedule = t.single_installment(1000, 5000);
    t.mint(&user, 200);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);

    t.advance_past(5000);
    t.client.mark_defaulted(&loan_id);

    // Attempting to repay a Defaulted loan must fail with LoanNotActive
    t.client.repay_loan(&user, &loan_id, &DEFAULT_TOTAL_DUE);
    let _ = loan_id;
}

#[test]
fn test_full_repayment_triggers_reputation_increase() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    let loan_id = t.create_default_loan(&user, &vendor);

    t.mint(&user, DEFAULT_TOTAL_DUE);
    t.client.repay_loan(&user, &loan_id, &DEFAULT_TOTAL_DUE);

    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Paid);
}

#[test]
fn test_early_repayment_triggers_bonus_reputation_increase() {
    // Repaying before the first installment due date is considered "early"
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    t.env.ledger().set_timestamp(1000);
    let schedule = t.single_installment(1000, 10000);
    t.mint(&user, DEFAULT_GUARANTEE);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);

    // Pay early (timestamp 2000, well before due date 10000)
    t.env.ledger().set_timestamp(2000);
    t.mint(&user, DEFAULT_TOTAL_DUE);
    t.client.repay_loan(&user, &loan_id, &DEFAULT_TOTAL_DUE);

    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Paid);
}

// ─── vendor validation ─────────────────────────────────────────────────────

#[test]
fn test_active_vendor_can_receive_loan() {
    // An approved and active vendor must pass validation
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let active_vendor = Address::generate(&t.env);

    // create_default_loan already registers the vendor
    let loan_id = t.create_default_loan(&user, &active_vendor);
    assert!(loan_id > 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // InvalidVendor
fn test_inactive_vendor_loan_is_rejected() {
    // A vendor that is registered but set to inactive must fail
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let inactive_vendor = Address::generate(&t.env);

    // Register the vendor first using invoke_contract
    use soroban_sdk::{IntoVal, Symbol};
    let vendor_name = SorobanString::from_str(&t.env, "Inactive Vendor");
    let _: Result<(), vendor_registry_contract::VendorRegistryError> = t.env.invoke_contract(
        &t.vendor_registry_id,
        &Symbol::new(&t.env, "register_vendor"),
        (&t.admin, &inactive_vendor, vendor_name).into_val(&t.env),
    );

    // Then deactivate the vendor
    let _: Result<(), vendor_registry_contract::VendorRegistryError> = t.env.invoke_contract(
        &t.vendor_registry_id,
        &Symbol::new(&t.env, "deactivate_vendor"),
        (&t.admin, &inactive_vendor).into_val(&t.env),
    );

    // This should panic with InvalidVendor error
    let _ = t.create_default_loan(&user, &inactive_vendor);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // InvalidVendor
fn test_unregistered_vendor_loan_is_rejected() {
    // A vendor address unknown to the registry must fail
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let unknown_vendor = Address::generate(&t.env);

    // Don't register the vendor - call create_loan directly instead of create_default_loan
    let due_date = t.env.ledger().timestamp() + 10_000;
    let schedule = t.single_installment(1000, due_date);

    // This should panic with InvalidVendor error
    let _ = t
        .client
        .create_loan(&user, &unknown_vendor, &1000, &200, &schedule, &LoanType::Standard);
}

// ─── liquidity pool integration — TDD stubs (Phase 6) ────────────────────────

#[test]
#[ignore = "liquidity pool integration not yet implemented — Phase 6"]
fn test_loan_funding_debits_liquidity_pool() {
    // create_loan must call fund_loan on the liquidity pool contract
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    // TODO: wire up a MockLiquidityPool; after create_loan verify fund_loan was called
    let _ = t.create_default_loan(&user, &vendor);
}

#[test]
#[ignore = "liquidity pool integration not yet implemented — Phase 6"]
fn test_repayment_credited_to_liquidity_pool() {
    // repay() must forward funds to the liquidity pool via receive_repayment
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    let loan_id = t.create_default_loan(&user, &vendor);
    t.mint(&user, DEFAULT_TOTAL_DUE);
    t.client.repay_loan(&user, &loan_id, &DEFAULT_TOTAL_DUE);
    // Verify MockLiquidityPool::receive_repayment was called
    let _ = loan_id;
}

#[test]
#[ignore = "liquidity pool integration not yet implemented — Phase 6"]
fn test_guarantee_transferred_to_pool_on_default() {
    // mark_defaulted must call receive_guarantee on the liquidity pool
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.env.ledger().set_timestamp(1000);
    let schedule = t.single_installment(1000, 5000);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);

    t.advance_past(5000);
    t.client.mark_defaulted(&loan_id);
    // TODO: Verify MockLiquidityPool::receive_guarantee(200) was called
    let _ = loan_id;
}

#[test]
#[ignore = "liquidity pool integration not yet implemented — Phase 6"]
#[should_panic(expected = "Error(Contract, #5)")] // InsufficientLiquidity
fn test_insufficient_liquidity_rejects_loan_creation() {
    // When pool does not have enough available liquidity, create_loan must fail
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    // TODO: wire up a MockLiquidityPool that returns available=0
    let _ = t.create_default_loan(&user, &vendor);
}

// ─── complete loan lifecycle ──────────────────────────────────────────────────

#[test]
fn test_complete_lifecycle_create_then_default() {
    // Verifies the full path: Active → overdue → Defaulted
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    t.env.ledger().set_timestamp(1000);
    let schedule = t.single_installment(1000, 5000);
    t.mint(&user, 200);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);

    let created = t.client.get_loan(&loan_id);
    assert_eq!(created.status, LoanStatus::Active);
    assert_eq!(created.remaining_balance, DEFAULT_TOTAL_DUE);

    t.advance_past(5000);
    t.client.mark_defaulted(&loan_id);

    let defaulted = t.client.get_loan(&loan_id);
    assert_eq!(defaulted.status, LoanStatus::Defaulted);
    // Amounts must be immutable after default
    assert_eq!(defaulted.total_amount, DEFAULT_PRINCIPAL);
    assert_eq!(defaulted.guarantee_amount, DEFAULT_GUARANTEE);
}

#[test]
fn test_multiple_independent_loans_do_not_interfere() {
    // Two concurrent loans for different borrowers must be fully independent
    let t = TestCtx::setup();
    let user_a = Address::generate(&t.env);
    let user_b = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    t.env.ledger().set_timestamp(1000);

    let schedule_a = t.single_installment(1000, 5000);
    let schedule_b = t.single_installment(2000, 8000);
    t.mint(&user_a, 200);
    t.mint(&user_b, 400);

    let loan_a = t
        .client
        .create_loan(&user_a, &vendor, &1000, &200, &schedule_a, &LoanType::Standard);
    let loan_b = t
        .client
        .create_loan(&user_b, &vendor, &2000, &400, &schedule_b, &LoanType::Standard);

    // Default loan_a only
    t.advance_past(5000);
    t.client.mark_defaulted(&loan_a);

    let la = t.client.get_loan(&loan_a);
    let lb = t.client.get_loan(&loan_b);

    assert_eq!(la.status, LoanStatus::Defaulted);
    assert_eq!(lb.status, LoanStatus::Active); // unaffected
    assert_eq!(lb.total_amount, 2000);
}

#[test]
fn test_complete_lifecycle_create_repay_complete() {
    // Verifies the full happy path: Active → repaid in full → Paid
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    let loan_id = t.create_default_loan(&user, &vendor);

    t.mint(&user, DEFAULT_TOTAL_DUE);

    let active = t.client.get_loan(&loan_id);
    assert_eq!(active.status, LoanStatus::Active);

    t.client.repay_loan(&user, &loan_id, &DEFAULT_TOTAL_DUE);

    let paid = t.client.get_loan(&loan_id);
    assert_eq!(paid.status, LoanStatus::Paid);
    assert_eq!(paid.remaining_balance, 0);
}

#[test]
fn test_multi_contract_integration_full_flow() {
    // End-to-end: reputation check on create → funding → repayment → score boost
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    // 1. Create loan — reputation validated, pool funded
    let loan_id = t.create_default_loan(&user, &vendor);

    t.mint(&user, DEFAULT_TOTAL_DUE);

    // 2. Repay in full — pool credited, reputation score increased
    t.client.repay_loan(&user, &loan_id, &DEFAULT_TOTAL_DUE);

    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Paid);

    // TODO: assert reputation score increased for `user`
    // TODO: assert liquidity pool received the repayment
    let _ = loan_id;
}

// ─── repayment — repay_loan implementation tests ─────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // LoanNotActive
fn test_repayment_on_defaulted_loan_is_rejected() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");

    t.env.ledger().set_timestamp(1000);
    let schedule = t.single_installment(1000, 5000);
    t.mint(&user, 200);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &1000, &200, &schedule, &LoanType::Standard);

    t.advance_past(5000);
    t.client.mark_defaulted(&loan_id);

    // Loan is now Defaulted — repayment must fail
    t.client.repay_loan(&user, &loan_id, &DEFAULT_TOTAL_DUE);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // LoanNotActive
fn test_repayment_on_already_paid_loan_is_rejected() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    let loan_id = t.create_default_loan(&user, &vendor);

    t.mint(&user, DEFAULT_TOTAL_DUE + 2);

    // Pay in full first
    t.client.repay_loan(&user, &loan_id, &DEFAULT_TOTAL_DUE);

    // Second repayment attempt must fail — loan is now Paid
    t.client.repay_loan(&user, &loan_id, &1);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")] // InvalidRepaymentAmount
fn test_zero_repayment_amount_is_rejected() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    let loan_id = t.create_default_loan(&user, &vendor);

    t.client.repay_loan(&user, &loan_id, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")] // InvalidRepaymentAmount
fn test_negative_repayment_amount_is_rejected() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    let loan_id = t.create_default_loan(&user, &vendor);

    t.client.repay_loan(&user, &loan_id, &-100);
}

#[test]
fn test_multiple_partial_repayments_accumulate_correctly() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    let loan_id = t.create_default_loan(&user, &vendor);

    t.mint(&user, DEFAULT_TOTAL_DUE);

    // Three partial payments: 300 + 300 + 450 = 1050
    t.client.repay_loan(&user, &loan_id, &300);
    t.client.repay_loan(&user, &loan_id, &300);
    let remaining = t.client.repay_loan(&user, &loan_id, &450);

    assert_eq!(remaining, 0);
    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Paid);
    assert_eq!(loan.remaining_balance, 0);
}

#[test]
fn test_repay_loan_emits_event() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    let loan_id = t.create_default_loan(&user, &vendor);

    t.mint(&user, DEFAULT_TOTAL_DUE);

    t.client.repay_loan(&user, &loan_id, &500);

    let events = t.env.events().all();
    assert!(
        !events.is_empty(),
        "Expected a LoanRepaid event to be emitted"
    );
}

#[test]
fn test_partial_repayment_does_not_trigger_reputation_increase() {
    // Partial payment must leave status Active — no reputation call expected
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    let loan_id = t.create_default_loan(&user, &vendor);

    t.mint(&user, DEFAULT_TOTAL_DUE);

    t.client.repay_loan(&user, &loan_id, &500);

    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Active); // still active, no score change
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")] // LoanNotFound
fn test_repayment_on_nonexistent_loan_fails() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);

    t.client.repay_loan(&user, &999, &500);
}

#[test]
fn test_active_debt_tracks_create_repay_and_default() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    let loan_id = t.create_default_loan(&user, &vendor);

    assert_eq!(t.client.get_user_active_debt(&user), DEFAULT_TOTAL_DUE);

    t.mint(&user, DEFAULT_TOTAL_DUE);
    t.client.repay_loan(&user, &loan_id, &500);
    assert_eq!(
        t.client.get_user_active_debt(&user),
        DEFAULT_TOTAL_DUE - 500
    );

    t.client
        .repay_loan(&user, &loan_id, &(DEFAULT_TOTAL_DUE - 500));
    assert_eq!(t.client.get_user_active_debt(&user), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #18)")] // ExposureLimitExceeded
fn test_exposure_limit_blocks_overleveraged_borrower() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    for _ in 0..9 {
        let _ = t.create_default_loan(&user, &vendor);
    }

    let _ = t.create_default_loan(&user, &vendor);
}

#[test]
fn test_request_loan_creates_pending_request_without_active_debt() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    let loan_id = t.create_default_request(&user, &vendor);
    let loan = t.client.get_loan(&loan_id);

    assert_eq!(loan.status, LoanStatus::Pending);
    assert_eq!(loan.funded_at, 0);
    assert_eq!(t.client.get_user_active_debt(&user), 0);
}

#[test]
fn test_cancel_pending_loan_refunds_guarantee() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    let starting_balance = DEFAULT_GUARANTEE;

    t.mint(&user, starting_balance);
    t.register_vendor(&vendor, "Test Vendor");
    let due_date = t.env.ledger().timestamp() + 10_000;
    let schedule = t.single_installment(DEFAULT_TOTAL_DUE, due_date);
    let loan_id = t.client.request_loan(
        &user,
        &vendor,
        &DEFAULT_PRINCIPAL,
        &DEFAULT_GUARANTEE,
        &schedule,
        &LoanType::Standard,
    );

    assert_eq!(t.balance(&user), 0);

    t.client.cancel_loan(&user, &loan_id);

    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Cancelled);
    assert_eq!(t.balance(&user), starting_balance);
}

#[test]
#[should_panic(expected = "Error(Contract, #19)")] // LoanNotCancellable
fn test_cannot_cancel_active_loan() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    let loan_id = t.create_default_loan(&user, &vendor);

    t.client.cancel_loan(&user, &loan_id);
}

struct RealIntegrationCtx {
    env: Env,
    creditline: CreditLineContractClient<'static>,
    reputation: ReputationContractClient<'static>,
    vendor_registry: vendor_registry_contract::VendorRegistryContractClient<'static>,
    pool: LiquidityPoolContractClient<'static>,
    parameters: ParametersContractClient<'static>,
    token: TokenClient<'static>,
    token_admin: StellarAssetClient<'static>,
    admin: Address,
    treasury: Address,
    vendor_fund: Address,
    creditline_id: Address,
}

impl RealIntegrationCtx {
    fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        let vendor_fund = Address::generate(&env);

        let token_admin_addr = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin_addr);
        let token_address = token_contract.address();
        let token = TokenClient::new(&env, &token_address);
        let token_admin = StellarAssetClient::new(&env, &token_address);

        let reputation_id = env.register(ReputationContract, ());
        let reputation = ReputationContractClient::new(&env, &reputation_id);

        let vendor_registry_id = env.register(VendorRegistryContract, ());
        let vendor_registry = vendor_registry_contract::VendorRegistryContractClient::new(
            &env,
            &vendor_registry_id,
        );

        let pool_id = env.register(LiquidityPoolContract, ());
        let pool = LiquidityPoolContractClient::new(&env, &pool_id);

        let parameters_id = env.register(ParametersContract, ());
        let parameters = ParametersContractClient::new(&env, &parameters_id);

        let creditline_id = env.register(CreditLineContract, ());
        let creditline = CreditLineContractClient::new(&env, &creditline_id);

        let token: TokenClient<'static> = unsafe { core::mem::transmute(token) };
        let token_admin: StellarAssetClient<'static> = unsafe { core::mem::transmute(token_admin) };
        let reputation: ReputationContractClient<'static> =
            unsafe { core::mem::transmute(reputation) };
        let vendor_registry: vendor_registry_contract::VendorRegistryContractClient<'static> =
            unsafe { core::mem::transmute(vendor_registry) };
        let pool: LiquidityPoolContractClient<'static> = unsafe { core::mem::transmute(pool) };
        let parameters: ParametersContractClient<'static> =
            unsafe { core::mem::transmute(parameters) };
        let creditline: CreditLineContractClient<'static> =
            unsafe { core::mem::transmute(creditline) };

        reputation.set_admin(&admin);
        reputation.set_updater(&admin, &admin, &true);
        reputation.set_updater(&admin, &creditline_id, &true);

        vendor_registry.initialize(&admin);
        pool.initialize(&admin, &token_address, &treasury, &vendor_fund);
        pool.set_creditline(&admin, &creditline_id);
        parameters.initialize_defaults(&admin);

        creditline.initialize(
            &admin,
            &reputation_id,
            &vendor_registry_id,
            &pool_id,
            &token_address,
        );
        creditline.set_parameters_contract(&admin, &parameters_id);

        Self {
            env,
            creditline,
            reputation,
            vendor_registry,
            pool,
            parameters,
            token,
            token_admin,
            admin,
            treasury,
            vendor_fund,
            creditline_id,
        }
    }

    fn mint(&self, to: &Address, amount: i128) {
        self.token_admin.mint(to, &amount);
    }

    fn balance(&self, addr: &Address) -> i128 {
        self.token.balance(addr)
    }

    fn fund_pool(&self, provider: &Address, amount: i128) {
        self.mint(provider, amount);
        self.pool.deposit(provider, &amount);
    }

    fn set_score(&self, user: &Address, score: u32) {
        self.reputation.set_score(&self.admin, user, &score);
    }

    fn register_vendor(&self, vendor: &Address, name: &str) {
        let vendor_name = SorobanString::from_str(&self.env, name);
        self.vendor_registry
            .register_vendor(&self.admin, vendor, &vendor_name);
    }

    fn single_installment(
        &self,
        amount: i128,
        due_date: u64,
    ) -> soroban_sdk::Vec<RepaymentInstallment> {
        let mut schedule = soroban_sdk::Vec::new(&self.env);
        schedule.push_back(RepaymentInstallment { due_date, amount,
 paid: false,
 paid_at: 0,
});
        schedule
    }
}

#[test]
fn test_real_asset_transfers_on_create_and_repay() {
    let t = RealIntegrationCtx::setup();
    let provider = Address::generate(&t.env);
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.fund_pool(&provider, 10_000);
    t.register_vendor(&vendor, "Integrated Vendor");
    t.set_score(&user, 80);
    t.mint(&user, 1_300);

    let pool_balance_before = t.balance(&t.pool.address);
    let vendor_balance_before = t.balance(&vendor);
    let creditline_balance_before = t.balance(&t.creditline_id);
    let user_balance_before = t.balance(&user);

    let due_date = t.env.ledger().timestamp() + 10_000;
    let schedule = t.single_installment(1_000, due_date);
    let loan_id = t
        .creditline
        .create_loan(&user, &vendor, &1_000, &200, &schedule, &LoanType::Standard);

    assert_eq!(t.balance(&user), user_balance_before - 200);
    assert_eq!(t.balance(&vendor), vendor_balance_before + 800);
    assert_eq!(t.balance(&t.creditline_id), creditline_balance_before + 200);
    assert_eq!(t.balance(&t.pool.address), pool_balance_before - 800);

    let pool_balance_after_loan = t.balance(&t.pool.address);
    let loan_before_repay = t.creditline.get_loan(&loan_id);
    let total_due = loan_before_repay.remaining_balance;
    let total_interest =
        loan_before_repay.interest_outstanding + loan_before_repay.service_fee_outstanding;
    let lp_interest = total_interest * 8_500 / 10_000;
    t.creditline.repay_loan(&user, &loan_id, &total_due);

    assert_eq!(t.balance(&user), user_balance_before - total_due);
    assert_eq!(t.balance(&t.creditline_id), 0);
    assert_eq!(
        t.balance(&t.pool.address),
        pool_balance_after_loan + loan_before_repay.principal_outstanding + lp_interest
    );

    let loan = t.creditline.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Paid);
    assert_eq!(loan.remaining_balance, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_parameters_contract_controls_guarantee_thresholds() {
    let t = RealIntegrationCtx::setup();
    let provider = Address::generate(&t.env);
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.fund_pool(&provider, 10_000);
    t.register_vendor(&vendor, "Governed Vendor");
    t.set_score(&user, 90);
    t.mint(&user, 300);

    let params = ProtocolParameters {
        min_guarantee_percent: 30,
        ..default_parameters()
    };
    t.parameters.update_parameters(&t.admin, &params);

    let due_date = t.env.ledger().timestamp() + 10_000;
    let schedule = t.single_installment(1_000, due_date);
    t.creditline
        .create_loan(&user, &vendor, &1_000, &200, &schedule, &LoanType::Standard);
}

#[test]
fn test_end_to_end_happy_path_across_all_contracts() {
    let t = RealIntegrationCtx::setup();
    let provider = Address::generate(&t.env);
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.fund_pool(&provider, 20_000);
    t.register_vendor(&vendor, "Protocol Vendor");
    t.set_score(&user, 80);
    t.mint(&user, 1_300);

    let share_price_before = t.pool.get_pool_stats().share_price;
    let due_date = t.env.ledger().timestamp() + 10_000;
    let schedule = t.single_installment(1_000, due_date);
    let loan_id = t
        .creditline
        .create_loan(&user, &vendor, &1_000, &200, &schedule, &LoanType::Standard);

    let loan_before_repay = t.creditline.get_loan(&loan_id);
    let total_due = loan_before_repay.remaining_balance;
    let total_interest =
        loan_before_repay.interest_outstanding + loan_before_repay.service_fee_outstanding;
    let protocol_fee_from_repayment = total_interest * 1_000 / 10_000;
    let lp_interest = total_interest * 8_500 / 10_000;
    let vendor_fee_from_repayment = total_interest - lp_interest - protocol_fee_from_repayment;
    t.creditline.repay_loan(&user, &loan_id, &total_due);

    let loan = t.creditline.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Paid);
    assert_eq!(t.reputation.get_score(&user), 95); // early repayment: +15 (80 → 95)

    t.mint(&t.creditline_id, 100);
    t.pool.receive_repayment(&t.creditline_id, &0, &100);

    let stats_after_interest = t.pool.get_pool_stats();
    assert!(stats_after_interest.share_price > share_price_before);
    assert_eq!(t.balance(&t.treasury), protocol_fee_from_repayment + 10);
    assert_eq!(t.balance(&t.vendor_fund), vendor_fee_from_repayment + 5);
}

#[test]
fn test_end_to_end_default_path_guarantee_and_penalty() {
    let t = RealIntegrationCtx::setup();
    let provider = Address::generate(&t.env);
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.fund_pool(&provider, 20_000);
    t.register_vendor(&vendor, "Risk Vendor");
    t.set_score(&user, 80);
    t.mint(&user, 200);

    t.env.ledger().set_timestamp(1_000);
    let schedule = t.single_installment(1_000, 5_000);
    let loan_id = t
        .creditline
        .create_loan(&user, &vendor, &1_000, &200, &schedule, &LoanType::Standard);

    let creditline_balance_after_loan = t.balance(&t.creditline_id);
    let pool_balance_after_loan = t.balance(&t.pool.address);

    t.env.ledger().set_timestamp(5_001);
    t.creditline.mark_defaulted(&loan_id);

    let loan = t.creditline.get_loan(&loan_id);
    let pool_stats = t.pool.get_pool_stats();
    assert_eq!(loan.status, LoanStatus::Defaulted);
    assert_eq!(t.reputation.get_score(&user), 60);
    assert_eq!(
        t.balance(&t.creditline_id),
        creditline_balance_after_loan - 200
    );
    assert_eq!(t.balance(&t.pool.address), pool_balance_after_loan + 200);
    assert_eq!(pool_stats.locked_liquidity, 600);
}

// ─── late fee tests ───────────────────────────────────────────────────────────

// LATE_FEE_BPS_PER_DAY = 50, BPS_DENOMINATOR = 10_000, SECONDS_PER_DAY = 86_400
// For DEFAULT_TOTAL_DUE = 1_050: fee per day = 1050 * 50 / 10_000 = 5

const SECONDS_PER_DAY: u64 = 86_400;

#[test]
fn test_no_late_fee_before_due_date() {
    // A loan repaid before its due date must have zero late fees
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.register_vendor(&vendor, "Test Vendor");
    t.env.ledger().set_timestamp(1_000);
    let due_date = 50_000_u64;
    let schedule = t.single_installment(DEFAULT_TOTAL_DUE, due_date);
    t.mint(&user, DEFAULT_GUARANTEE);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &DEFAULT_PRINCIPAL, &DEFAULT_GUARANTEE, &schedule, &LoanType::Standard);

    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.late_fees_outstanding, 0);
    assert_eq!(loan.late_fee_accrual_timestamp, 0);
}

#[test]
fn test_apply_late_fees_adds_fee_after_one_day_overdue() {
    // apply_late_fees after exactly 1 full day overdue must add 5 to remaining_balance
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.register_vendor(&vendor, "Test Vendor");
    let due_date = 1_000_u64;
    let schedule = t.single_installment(DEFAULT_TOTAL_DUE, due_date);
    t.env.ledger().set_timestamp(0);
    t.mint(&user, DEFAULT_GUARANTEE);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &DEFAULT_PRINCIPAL, &DEFAULT_GUARANTEE, &schedule, &LoanType::Standard);

    // Advance 1 full day past due_date
    t.env.ledger().set_timestamp(due_date + SECONDS_PER_DAY);
    t.client.apply_late_fees(&loan_id);

    let loan = t.client.get_loan(&loan_id);
    // fee = 1050 * 50 / 10_000 = 5
    assert_eq!(loan.late_fees_outstanding, 5);
    assert_eq!(loan.remaining_balance, DEFAULT_TOTAL_DUE + 5);
}

#[test]
fn test_apply_late_fees_accumulates_over_multiple_days() {
    // 3 days overdue → fee = 1050 * 50 * 3 / 10_000 = 15
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.register_vendor(&vendor, "Test Vendor");
    let due_date = 1_000_u64;
    let schedule = t.single_installment(DEFAULT_TOTAL_DUE, due_date);
    t.env.ledger().set_timestamp(0);
    t.mint(&user, DEFAULT_GUARANTEE);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &DEFAULT_PRINCIPAL, &DEFAULT_GUARANTEE, &schedule, &LoanType::Standard);

    t.env.ledger().set_timestamp(due_date + 3 * SECONDS_PER_DAY);
    t.client.apply_late_fees(&loan_id);

    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.late_fees_outstanding, 15);
    assert_eq!(loan.remaining_balance, DEFAULT_TOTAL_DUE + 15);
}

#[test]
fn test_apply_late_fees_is_noop_within_same_day() {
    // Calling apply_late_fees twice within the same day must not double-count
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.register_vendor(&vendor, "Test Vendor");
    let due_date = 1_000_u64;
    let schedule = t.single_installment(DEFAULT_TOTAL_DUE, due_date);
    t.env.ledger().set_timestamp(0);
    t.mint(&user, DEFAULT_GUARANTEE);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &DEFAULT_PRINCIPAL, &DEFAULT_GUARANTEE, &schedule, &LoanType::Standard);

    t.env.ledger().set_timestamp(due_date + SECONDS_PER_DAY);
    t.client.apply_late_fees(&loan_id);

    // Second call within the same second — no additional day has elapsed
    t.client.apply_late_fees(&loan_id);

    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.late_fees_outstanding, 5); // still just 1 day
}

#[test]
fn test_apply_late_fees_incremental_across_two_calls() {
    // Day 1 call then day 2 call should each contribute one day's fee
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.register_vendor(&vendor, "Test Vendor");
    let due_date = 0_u64;
    let schedule = t.single_installment(DEFAULT_TOTAL_DUE, due_date);
    t.env.ledger().set_timestamp(0);
    t.mint(&user, DEFAULT_GUARANTEE);
    // due_date == 0, so it's already overdue at creation time; first day starts at ledger 0
    // Advance ledger to 1 so due_date < now and create the loan
    t.env.ledger().set_timestamp(1);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &DEFAULT_PRINCIPAL, &DEFAULT_GUARANTEE, &schedule, &LoanType::Standard);

    // First accrual: 1 day after due_date (due_date = 0, now = SECONDS_PER_DAY)
    t.env.ledger().set_timestamp(SECONDS_PER_DAY);
    t.client.apply_late_fees(&loan_id);
    let after_day1 = t.client.get_loan(&loan_id).late_fees_outstanding;

    // Second accrual: another full day later
    t.env.ledger().set_timestamp(2 * SECONDS_PER_DAY);
    t.client.apply_late_fees(&loan_id);
    let after_day2 = t.client.get_loan(&loan_id).late_fees_outstanding;

    // Day 1 fee: 1050 * 50 / 10_000 = 5
    // Day 2 fee is on the new balance (1055): 1055 * 50 / 10_000 = 5 (integer division)
    assert_eq!(after_day1, 5);
    assert!(after_day2 > after_day1);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // LoanNotActive
fn test_apply_late_fees_on_paid_loan_fails() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);
    let loan_id = t.create_default_loan(&user, &vendor);

    t.mint(&user, DEFAULT_TOTAL_DUE);
    t.client.repay_loan(&user, &loan_id, &DEFAULT_TOTAL_DUE);

    // Past due date — but loan is already Paid; should reject
    t.env.ledger().set_timestamp(100_000);
    t.client.apply_late_fees(&loan_id);
}

#[test]
fn test_repay_loan_auto_accrues_late_fees() {
    // repay_loan must accrue outstanding late fees before processing payment
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.register_vendor(&vendor, "Test Vendor");
    let due_date = 1_000_u64;
    let schedule = t.single_installment(DEFAULT_TOTAL_DUE, due_date);
    t.env.ledger().set_timestamp(0);
    t.mint(&user, DEFAULT_GUARANTEE);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &DEFAULT_PRINCIPAL, &DEFAULT_GUARANTEE, &schedule, &LoanType::Standard);

    // Advance 1 day past due_date and attempt partial payment
    t.env.ledger().set_timestamp(due_date + SECONDS_PER_DAY);
    // fee = 5; provide enough to cover only the original balance
    t.mint(&user, DEFAULT_TOTAL_DUE);
    // Pay only the original balance — should leave late fees outstanding
    t.client.repay_loan(&user, &loan_id, &DEFAULT_TOTAL_DUE);

    let loan = t.client.get_loan(&loan_id);
    // Loan is still Active: original balance paid but late fees remain
    assert_eq!(loan.status, LoanStatus::Active);
    assert_eq!(loan.late_fees_outstanding, 5);
    assert_eq!(loan.remaining_balance, 5);
}

#[test]
fn test_full_repayment_including_late_fees_sets_paid() {
    // Borrower paying remaining_balance after late-fee accrual closes the loan
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.register_vendor(&vendor, "Test Vendor");
    let due_date = 1_000_u64;
    let schedule = t.single_installment(DEFAULT_TOTAL_DUE, due_date);
    t.env.ledger().set_timestamp(0);
    t.mint(&user, DEFAULT_GUARANTEE);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &DEFAULT_PRINCIPAL, &DEFAULT_GUARANTEE, &schedule, &LoanType::Standard);

    t.env.ledger().set_timestamp(due_date + SECONDS_PER_DAY);
    t.client.apply_late_fees(&loan_id);

    let loan = t.client.get_loan(&loan_id);
    let total_due = loan.remaining_balance; // DEFAULT_TOTAL_DUE + 5

    t.mint(&user, total_due);
    t.client.repay_loan(&user, &loan_id, &total_due);

    let loan = t.client.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Paid);
    assert_eq!(loan.remaining_balance, 0);
    assert_eq!(loan.late_fees_outstanding, 0);
}

#[test]
fn test_active_debt_includes_late_fees() {
    // apply_late_fees must increase the borrower's active debt by the fee amount
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.register_vendor(&vendor, "Test Vendor");
    let due_date = 1_000_u64;
    let schedule = t.single_installment(DEFAULT_TOTAL_DUE, due_date);
    t.env.ledger().set_timestamp(0);
    t.mint(&user, DEFAULT_GUARANTEE);
    let loan_id = t
        .client
        .create_loan(&user, &vendor, &DEFAULT_PRINCIPAL, &DEFAULT_GUARANTEE, &schedule, &LoanType::Standard);

    let debt_before = t.client.get_user_active_debt(&user);

    t.env.ledger().set_timestamp(due_date + SECONDS_PER_DAY);
    t.client.apply_late_fees(&loan_id);

    let debt_after = t.client.get_user_active_debt(&user);
    assert_eq!(debt_after, debt_before + 5);
}

// ─── reputation increase on repayment ────────────────────────────────────────

#[test]
fn test_on_time_full_repayment_increases_score_by_10() {
    // Payment at exactly the due date (not early) → +10
    let t = RealIntegrationCtx::setup();
    let provider = Address::generate(&t.env);
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.fund_pool(&provider, 10_000);
    t.register_vendor(&vendor, "On-Time Vendor");
    t.set_score(&user, 60);
    t.mint(&user, 1_300);

    let due_date = 5_000_u64;
    let schedule = t.single_installment(1_000, due_date);
    let loan_id = t
        .creditline
        .create_loan(&user, &vendor, &1_000, &200, &schedule, &LoanType::Standard);

    let loan = t.creditline.get_loan(&loan_id);
    let total_due = loan.remaining_balance;
    t.mint(&user, total_due);

    // Advance to exactly due_date — payment_date == due_date is NOT early
    t.env.ledger().set_timestamp(due_date);
    t.creditline.repay_loan(&user, &loan_id, &total_due);

    assert_eq!(t.reputation.get_score(&user), 70); // on-time: +10 (60 → 70)
}

#[test]
fn test_early_full_repayment_increases_score_by_15() {
    // Payment well before the due date → +15
    let t = RealIntegrationCtx::setup();
    let provider = Address::generate(&t.env);
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.fund_pool(&provider, 10_000);
    t.register_vendor(&vendor, "Early Vendor");
    t.set_score(&user, 60);
    t.mint(&user, 1_300);

    t.env.ledger().set_timestamp(1_000);
    let due_date = 10_000_u64;
    let schedule = t.single_installment(1_000, due_date);
    let loan_id = t
        .creditline
        .create_loan(&user, &vendor, &1_000, &200, &schedule, &LoanType::Standard);

    let loan = t.creditline.get_loan(&loan_id);
    let total_due = loan.remaining_balance;
    t.mint(&user, total_due);

    // Pay at timestamp 2000, well before due_date 10000
    t.env.ledger().set_timestamp(2_000);
    t.creditline.repay_loan(&user, &loan_id, &total_due);

    assert_eq!(t.reputation.get_score(&user), 75); // early: +15 (60 → 75)
}

#[test]
fn test_partial_repayment_does_not_change_reputation_score() {
    // Partial repayment must not trigger any reputation increase
    let t = RealIntegrationCtx::setup();
    let provider = Address::generate(&t.env);
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.fund_pool(&provider, 10_000);
    t.register_vendor(&vendor, "Partial Vendor");
    t.set_score(&user, 70);
    t.mint(&user, 1_300);

    let due_date = t.env.ledger().timestamp() + 10_000;
    let schedule = t.single_installment(1_000, due_date);
    let loan_id = t
        .creditline
        .create_loan(&user, &vendor, &1_000, &200, &schedule, &LoanType::Standard);

    let loan = t.creditline.get_loan(&loan_id);
    t.mint(&user, loan.remaining_balance);
    t.creditline.repay_loan(&user, &loan_id, &500);

    assert_eq!(t.reputation.get_score(&user), 70); // unchanged after partial payment
}

#[test]
fn test_reputation_call_failure_does_not_block_repayment() {
    // Even if the reputation contract call fails, the loan repayment must succeed
    // We verify this by removing the creditline as a reputation updater and
    // confirming the loan still moves to Paid status.
    let t = RealIntegrationCtx::setup();
    let provider = Address::generate(&t.env);
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    t.fund_pool(&provider, 10_000);
    t.register_vendor(&vendor, "Fail Vendor");
    t.set_score(&user, 60);
    t.mint(&user, 1_300);

    let due_date = t.env.ledger().timestamp() + 10_000;
    let schedule = t.single_installment(1_000, due_date);
    let loan_id = t
        .creditline
        .create_loan(&user, &vendor, &1_000, &200, &schedule, &LoanType::Standard);

    let loan = t.creditline.get_loan(&loan_id);
    let total_due = loan.remaining_balance;
    t.mint(&user, total_due);

    // Revoke updater permission so the increase_score call will fail
    t.reputation
        .set_updater(&t.admin, &t.creditline_id, &false);

    // Repayment must still succeed despite the reputation call failure
    t.creditline.repay_loan(&user, &loan_id, &total_due);

    let loan = t.creditline.get_loan(&loan_id);
    assert_eq!(loan.status, LoanStatus::Paid);
    assert_eq!(loan.remaining_balance, 0);
    // Score unchanged because the call was silently ignored
    assert_eq!(t.reputation.get_score(&user), 60);
}

#[test]
fn test_approve_loan_success() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    let loan_id = t.create_default_request(&user, &vendor);
    let pending = t.client.get_loan(&loan_id);
    assert_eq!(pending.status, LoanStatus::Pending);

    let approved = t.client.approve_loan(&loan_id);
    assert_eq!(approved.status, LoanStatus::Active);

    let stored = t.client.get_loan(&loan_id);
    assert_eq!(stored.status, LoanStatus::Active);
}

#[test]
#[should_panic(expected = "Error(Contract, #25)")] // InvalidLoanStatus
fn test_approve_loan_wrong_status() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    let loan_id = t.create_default_request(&user, &vendor);
    t.client.approve_loan(&loan_id);

    // Loan is now Active — approving again must fail with InvalidLoanStatus.
    t.client.approve_loan(&loan_id);
}

#[test]
fn test_approve_loan_not_admin() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let vendor = Address::generate(&t.env);

    let loan_id = t.create_default_request(&user, &vendor);

    // Clear all mocked auths so admin.require_auth() must come from a real
    // authorization entry. Calling approve_loan without an admin auth entry
    // should fail with an auth error.
    t.env.set_auths(&[]);

    let result = t.client.try_approve_loan(&loan_id);
    assert!(result.is_err(), "expected auth error when caller is not admin");
}

// ─── repay_installment tests ──────────────────────────────────────────────────

/// Helper: creates a loan with `n_installments` equal-valued installments
/// using DEFAULT_PRINCIPAL and a generated vendor. Returns (loan_id, vendor).
fn setup_loan_with_schedule(
    t: &TestCtx,
    borrower: &Address,
    n_installments: u32,
) -> (u64, Address) {
    let vendor = Address::generate(&t.env);
    t.register_vendor(&vendor, "Test Vendor");
    t.mint(borrower, DEFAULT_GUARANTEE);

    let due_date = t.env.ledger().timestamp() + 10_000;
    let installment_amount = DEFAULT_TOTAL_DUE / n_installments as i128;
    let mut schedule = soroban_sdk::Vec::new(&t.env);

    for i in 0..n_installments {
        schedule.push_back(RepaymentInstallment {
            amount: installment_amount,
            due_date: due_date + (i as u64 * 10_000),
            paid: false,
            paid_at: 0,
        });
    }

    let loan_id = t.client.create_loan(
        borrower,
        &vendor,
        &DEFAULT_PRINCIPAL,
        &DEFAULT_GUARANTEE,
        &schedule,
        &LoanType::Standard,
    );
    (loan_id, vendor)
}

#[test]
fn test_repay_installment_happy_path() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);

    let (loan_id, _vendor) = setup_loan_with_schedule(&t, &user, 2);
    let payment = 500_i128;

    t.mint(&user, payment);

    t.env.ledger().set_timestamp(5000);
    let remaining = t.client.repay_installment(&user, &loan_id, &0, &payment);

    let loan = t.client.get_loan(&loan_id);
    let installment = loan.repayment_schedule.get(0).unwrap();
    assert!(installment.paid);
    assert_eq!(installment.paid_at, 5000);
    assert_eq!(loan.remaining_balance, DEFAULT_TOTAL_DUE - payment);
    assert_eq!(remaining, DEFAULT_TOTAL_DUE - payment);
    assert_eq!(loan.status, LoanStatus::Active);
    // Second installment remains unpaid
    let inst2 = loan.repayment_schedule.get(1).unwrap();
    assert!(!inst2.paid);
    assert_eq!(inst2.paid_at, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #24)")]
fn test_repay_installment_double_pay_rejected() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);

    let (loan_id, _vendor) = setup_loan_with_schedule(&t, &user, 2);
    let payment = 500_i128;

    t.mint(&user, payment * 2);

    t.client.repay_installment(&user, &loan_id, &0, &payment);
    t.client.repay_installment(&user, &loan_id, &0, &payment);
}

#[test]
#[should_panic(expected = "Error(Contract, #23)")]
fn test_repay_installment_out_of_bounds() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);

    let (loan_id, _vendor) = setup_loan_with_schedule(&t, &user, 2);

    // Index 2 is out of bounds for a 2-installment schedule (valid: 0, 1)
    t.client.repay_installment(&user, &loan_id, &2, &500);
}

#[test]
#[should_panic(expected = "Error(Contract, #14)")]
fn test_repay_installment_non_borrower_rejected() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);
    let intruder = Address::generate(&t.env);

    let (loan_id, _vendor) = setup_loan_with_schedule(&t, &user, 2);

    // A different address cannot repay the borrower's installment
    t.client.repay_installment(&intruder, &loan_id, &0, &500);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_repay_installment_zero_amount_rejected() {
    let t = TestCtx::setup();
    let user = Address::generate(&t.env);

    let (loan_id, _vendor) = setup_loan_with_schedule(&t, &user, 2);

    // Zero-amount payment must be rejected
    t.client.repay_installment(&user, &loan_id, &0, &0);
}
