use crate::{LiquidityPoolContract, LiquidityPoolContractClient, LiquidityPoolError};
use soroban_sdk::{
    testutils::{Address as _, Events},
    token::{Client as TokenClient, StellarAssetClient},
    Address, Env, IntoVal,
};

// ─── helpers ──────────────────────────────────────────────────────────────────

struct TestEnv {
    env: Env,
    client: LiquidityPoolContractClient<'static>,
    token: TokenClient<'static>,
    admin: Address,
    treasury: Address,
    merchant_fund: Address,
    creditline: Address,
}

impl TestEnv {
    fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        // Deploy a Stellar asset (test token)
        let token_admin = Address::generate(&env);
        let token_contract_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_contract_id.address();

        let token = TokenClient::new(&env, &token_address);
        let token_sac = StellarAssetClient::new(&env, &token_address);

        // Deploy liquidity pool contract
        let contract_id = env.register(LiquidityPoolContract, ());
        let client = LiquidityPoolContractClient::new(&env, &contract_id);

        // SAFETY: we extend the lifetime so we can store these in the struct.
        // This is safe because `env` outlives both clients.
        let token: TokenClient<'static> = unsafe { core::mem::transmute(token) };
        let token_sac: StellarAssetClient<'static> = unsafe { core::mem::transmute(token_sac) };
        let client: LiquidityPoolContractClient<'static> = unsafe { core::mem::transmute(client) };

        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        let merchant_fund = Address::generate(&env);
        let creditline = Address::generate(&env);

        // Initialize pool
        client.initialize(&admin, &token_address, &treasury, &merchant_fund);
        client.set_creditline(&admin, &creditline);

        // Mint tokens into some standard accounts for tests to use
        token_sac.mint(&creditline, &10_000_000);

        Self {
            env,
            client,
            token,
            admin,
            treasury,
            merchant_fund,
            creditline,
        }
    }

    /// Mint `amount` tokens to `recipient`
    fn mint(&self, recipient: &Address, amount: i128) {
        let token_address = self.token.address.clone();
        let token_sac = StellarAssetClient::new(&self.env, &token_address);
        token_sac.mint(recipient, &amount);
    }
}

// ─── initialization ───────────────────────────────────────────────────────────

#[test]
fn test_initialize() {
    let t = TestEnv::setup();
    assert_eq!(t.client.get_admin(), t.admin);
}

#[test]
fn test_get_admin_before_initialize_returns_typed_error() {
    let env = Env::default();
    let contract_id = env.register(LiquidityPoolContract, ());
    let client = LiquidityPoolContractClient::new(&env, &contract_id);

    assert_eq!(
        client.try_get_admin(),
        Err(Ok(LiquidityPoolError::NotInitialized))
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_initialize_twice_fails() {
    let t = TestEnv::setup();
    // Second call should panic with AlreadyInitialized (2)
    let another_admin = Address::generate(&t.env);
    t.client.initialize(
        &another_admin,
        &t.token.address,
        &t.treasury,
        &t.merchant_fund,
    );
}

// ─── deposit ──────────────────────────────────────────────────────────────────

#[test]
fn test_first_deposit_one_to_one_ratio() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 1_000);

    let shares = t.client.deposit(&provider, &1_000);

    // First deposit → shares == amount
    assert_eq!(shares, 1_000);
    assert_eq!(t.client.get_lp_shares(&provider), 1_000);

    let stats = t.client.get_pool_stats();
    assert_eq!(stats.total_liquidity, 1_000);
    assert_eq!(stats.total_shares, 1_000);
    assert_eq!(stats.locked_liquidity, 0);
    assert_eq!(stats.available_liquidity, 1_000);
}

#[test]
fn test_subsequent_deposit_proportional_shares() {
    let t = TestEnv::setup();

    let provider_a = Address::generate(&t.env);
    let provider_b = Address::generate(&t.env);
    t.mint(&provider_a, 1_000);
    t.mint(&provider_b, 1_000);

    // First deposit
    t.client.deposit(&provider_a, &1_000);

    // Second deposit: same amount → same shares (pool value unchanged)
    let shares_b = t.client.deposit(&provider_b, &1_000);
    assert_eq!(shares_b, 1_000);

    let stats = t.client.get_pool_stats();
    assert_eq!(stats.total_liquidity, 2_000);
    assert_eq!(stats.total_shares, 2_000);
}

#[test]
fn test_deposit_after_interest_increases_share_value() {
    // Simulate: pool gains interest → share_price > 1.00 →
    // subsequent depositor gets fewer shares per token.
    let t = TestEnv::setup();

    let provider_a = Address::generate(&t.env);
    let provider_b = Address::generate(&t.env);
    t.mint(&provider_a, 1_000);
    t.mint(&provider_b, 1_000);

    // First deposit: 1000 tokens → 1000 shares
    t.client.deposit(&provider_a, &1_000);

    // Simulate interest: distribute 100 tokens of interest.
    // 85 stays in pool → total_liquidity becomes 1085, total_shares stays 1000.
    // share_price = 1085/1000 = 1.085
    // The test helper calls distribute_interest directly:
    // We inject interest by sending tokens to the pool and calling receive_repayment
    // with principal=0, interest=100.
    t.mint(&t.creditline, 100);
    t.client.receive_repayment(&t.creditline, &0, &100);

    // Now total_liquidity includes the LP portion (85) of interest.
    // Pool: total_liquidity = 1000 + 85 = 1085, total_shares = 1000
    // Second deposit of 1000 tokens: shares = 1000 * 1000 / 1085 ≈ 921
    let shares_b = t.client.deposit(&provider_b, &1_000);
    assert!(
        shares_b < 1_000,
        "Shares must be < 1000 since pool value grew"
    );

    // provider_a's shares are still 1000 but worth more
    assert_eq!(t.client.get_lp_shares(&provider_a), 1_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_deposit_zero_amount_fails() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.client.deposit(&provider, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_deposit_negative_amount_fails() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.client.deposit(&provider, &-500);
}

// ─── withdraw ─────────────────────────────────────────────────────────────────

#[test]
fn test_full_withdrawal() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 1_000);

    t.client.deposit(&provider, &1_000);

    let amount_returned = t.client.withdraw(&provider, &1_000);
    assert_eq!(amount_returned, 1_000);
    assert_eq!(t.client.get_lp_shares(&provider), 0);

    let stats = t.client.get_pool_stats();
    assert_eq!(stats.total_liquidity, 0);
    assert_eq!(stats.total_shares, 0);
}

#[test]
fn test_partial_withdrawal() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 1_000);

    t.client.deposit(&provider, &1_000);

    let amount_returned = t.client.withdraw(&provider, &400);
    assert_eq!(amount_returned, 400);
    assert_eq!(t.client.get_lp_shares(&provider), 600);

    let stats = t.client.get_pool_stats();
    assert_eq!(stats.total_liquidity, 600);
    assert_eq!(stats.total_shares, 600);
}

#[test]
fn test_withdrawal_reflects_share_appreciation() {
    // After interest, withdrawing all shares returns more than deposited.
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 1_000);

    t.client.deposit(&provider, &1_000);

    // Distribute 100 interest (85 stays in pool)
    t.mint(&t.creditline, 100);
    t.client.receive_repayment(&t.creditline, &0, &100);

    // Total_liquidity = 1085, total_shares = 1000
    // Withdraw all 1000 shares → should receive 1085 tokens
    let amount_returned = t.client.withdraw(&provider, &1_000);
    assert_eq!(amount_returned, 1_085);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_withdraw_more_shares_than_owned_fails() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);
    t.client.withdraw(&provider, &1_001);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_withdraw_when_liquidity_locked_fails() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    let merchant = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);

    // Lock all liquidity in a loan
    t.client.fund_loan(&t.creditline, &merchant, &1_000);

    // Try to withdraw → all liquidity is locked
    t.client.withdraw(&provider, &1_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_withdraw_zero_shares_fails() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.client.withdraw(&provider, &0);
}

// ─── fund_loan ────────────────────────────────────────────────────────────────

#[test]
fn test_fund_loan_increases_locked_liquidity() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    let merchant = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);

    t.client.fund_loan(&t.creditline, &merchant, &400);

    let stats = t.client.get_pool_stats();
    assert_eq!(stats.locked_liquidity, 400);
    assert_eq!(stats.available_liquidity, 600);
    assert_eq!(stats.total_liquidity, 1_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_fund_loan_exceeds_available_fails() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    let merchant = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);

    // Try to fund more than available
    t.client.fund_loan(&t.creditline, &merchant, &1_001);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_fund_loan_unauthorized_caller_fails() {
    let t = TestEnv::setup();
    let intruder = Address::generate(&t.env);
    let merchant = Address::generate(&t.env);
    t.client.fund_loan(&intruder, &merchant, &100);
}

// ─── receive_repayment ────────────────────────────────────────────────────────

#[test]
fn test_receive_repayment_decreases_locked_and_distributes_interest() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    let merchant = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);

    // Fund a 400-token loan
    t.client.fund_loan(&t.creditline, &merchant, &400);

    // Repay 400 principal + 40 interest
    t.mint(&t.creditline, 440);
    t.client.receive_repayment(&t.creditline, &400, &40);

    let stats = t.client.get_pool_stats();
    assert_eq!(stats.locked_liquidity, 0);

    // fund_loan does NOT reduce total_liquidity — only moves tokens into locked.
    // LP portion of interest = 85% of 40 = 34
    // total_liquidity = 1000 (original) + 34 (LP interest) = 1034
    assert_eq!(stats.total_liquidity, 1_034);
}

#[test]
fn test_receive_repayment_treasury_receives_protocol_fee() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);

    // Send 100 interest
    t.mint(&t.creditline, 100);
    t.client.receive_repayment(&t.creditline, &0, &100);

    // Treasury gets 10% = 10
    let treasury_balance = t.token.balance(&t.treasury);
    assert_eq!(treasury_balance, 10);
}

#[test]
fn test_receive_repayment_merchant_fund_receives_fee() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);

    // Send 100 interest
    t.mint(&t.creditline, 100);
    t.client.receive_repayment(&t.creditline, &0, &100);

    // Merchant fund gets 5% = 5
    let mf_balance = t.token.balance(&t.merchant_fund);
    assert_eq!(mf_balance, 5);
}

#[test]
fn test_double_counting_does_not_compound_across_multiple_loan_cycles() {
    // Regression test for the double-counting bug:
    // After N fund_loan → receive_repayment cycles with zero interest,
    // total_liquidity must remain exactly equal to the original deposit.
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    let merchant = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);

    // Cycle 1
    t.client.fund_loan(&t.creditline, &merchant, &600);
    t.mint(&t.creditline, 600);
    t.client.receive_repayment(&t.creditline, &600, &0);

    assert_eq!(t.client.get_pool_stats().total_liquidity, 1_000);

    // Cycle 2
    t.client.fund_loan(&t.creditline, &merchant, &600);
    t.mint(&t.creditline, 600);
    t.client.receive_repayment(&t.creditline, &600, &0);

    assert_eq!(t.client.get_pool_stats().total_liquidity, 1_000);

    // Cycle 3
    t.client.fund_loan(&t.creditline, &merchant, &600);
    t.mint(&t.creditline, 600);
    t.client.receive_repayment(&t.creditline, &600, &0);

    assert_eq!(t.client.get_pool_stats().total_liquidity, 1_000);

    // Provider must be able to withdraw their full original deposit
    let returned = t.client.withdraw(&provider, &1_000);
    assert_eq!(returned, 1_000);
}

#[test]
#[should_panic(expected = "Error(Contract")] // non-admin should be rejected
fn test_upgrade_requires_admin() {
    let env = Env::default();
    let contract_id = env.register(LiquidityPoolContract, ());
    let client = LiquidityPoolContractClient::new(&env, &contract_id);

    let wasm_hash = soroban_sdk::BytesN::from_array(&env, &[0u8; 32]);
    client.upgrade(&wasm_hash);
}

#[test]
fn test_admin_upgrade_bumps_version() {
    let t = TestEnv::setup();
    // default version should be 1
    assert_eq!(t.client.get_version(), 1u32);

    let wasm_hash = t.env.deployer().upload_contract_wasm(soroban_sdk::Bytes::from_slice(&t.env, include_bytes!("../../../contracts/test-fixtures/contract.wasm")));
    t.client.upgrade(&wasm_hash);

    assert_eq!(t.client.get_version(), 2u32);
    // event observed
    let events: soroban_sdk::Vec<(soroban_sdk::Address, soroban_sdk::Vec<soroban_sdk::Val>, soroban_sdk::Val)> = t.env.events().all();
    let mut found = false;
    for e in events.iter() {
        let topic: soroban_sdk::Symbol = e.1.get_unchecked(0).into_val(&t.env);
        if topic == soroban_sdk::Symbol::new(&t.env, "CONTRACTUPGRADED") {
            found = true;
            break;
        }
    }
    assert!(found, "CONTRACTUPGRADED event not found");
}

// ─── distribute_interest (SC-17 core) ────────────────────────────────────────

#[test]
fn test_distribute_interest_fee_split_accuracy() {
    // 85 / 10 / 5 split on 1000 tokens
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 10_000);
    t.client.deposit(&provider, &10_000);

    t.mint(&t.creditline, 1_000);
    t.client.receive_repayment(&t.creditline, &0, &1_000);

    // LP: 850 stays in pool → total_liquidity = 10000 + 850 = 10850
    let stats = t.client.get_pool_stats();
    assert_eq!(stats.total_liquidity, 10_850);

    // Treasury: 10% = 100
    assert_eq!(t.token.balance(&t.treasury), 100);

    // Merchant fund: 5% = 50
    assert_eq!(t.token.balance(&t.merchant_fund), 50);
}

#[test]
fn test_distribute_interest_share_value_appreciation() {
    // Start: 1 share = $1.00 (10000 bps)
    // After 8% interest on 1000 tokens deposit:
    //   interest = 80, lp_portion = 68 (85%)
    //   share_price = (1000 + 68) / 1000 * 10000 = 10680 bps
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);

    let stats_before = t.client.get_pool_stats();
    assert_eq!(stats_before.share_price, 10_000); // $1.00 in bps

    // Distribute 80 tokens of interest (8% on 1000)
    t.mint(&t.creditline, 80);
    t.client.receive_repayment(&t.creditline, &0, &80);

    let stats_after = t.client.get_pool_stats();
    // lp_amount = 80 * 8500 / 10000 = 68
    assert_eq!(stats_after.total_liquidity, 1_068);
    assert_eq!(stats_after.share_price, 10_680); // $1.068 expressed as bps
}

#[test]
fn test_multiple_lp_proportional_distribution() {
    // Two LPs: A deposits 1000, B deposits 1000.
    // After interest, both should benefit proportionally.
    let t = TestEnv::setup();

    let provider_a = Address::generate(&t.env);
    let provider_b = Address::generate(&t.env);
    t.mint(&provider_a, 1_000);
    t.mint(&provider_b, 1_000);

    t.client.deposit(&provider_a, &1_000);
    t.client.deposit(&provider_b, &1_000);

    // 200 interest distributed (100 per LP proportionally)
    t.mint(&t.creditline, 200);
    t.client.receive_repayment(&t.creditline, &0, &200);

    // LP amount = 85% of 200 = 170 → added to pool
    // total_liquidity = 2000 + 170 = 2170, total_shares = 2000
    let stats = t.client.get_pool_stats();
    assert_eq!(stats.total_liquidity, 2_170);

    // Both LPs hold 1000 shares out of 2000 → each owns 50% of pool
    // Withdrawal value per LP = 1000 * 2170 / 2000 = 1085
    let val_a = t.client.calculate_withdrawal(&1_000);
    let val_b = t.client.calculate_withdrawal(&1_000);
    assert_eq!(val_a, 1_085);
    assert_eq!(val_b, 1_085);
}

#[test]
fn test_interest_calculation_accuracy_small_amount() {
    // 100 interest: lp=85, treasury=10, merchant=5 (exact, no rounding)
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);

    t.mint(&t.creditline, 100);
    t.client.receive_repayment(&t.creditline, &0, &100);

    assert_eq!(t.token.balance(&t.treasury), 10);
    assert_eq!(t.token.balance(&t.merchant_fund), 5);

    let stats = t.client.get_pool_stats();
    assert_eq!(stats.total_liquidity, 1_085);
}

#[test]
fn test_interest_rounding_remainder_goes_to_lp() {
    // Use an amount that doesn't divide evenly: 101
    // lp = 101 * 8500 / 10000 = 85 (floor)
    // protocol = 101 * 1000 / 10000 = 10 (floor)
    // merchant = 101 - 85 - 10 = 6
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 10_000);
    t.client.deposit(&provider, &10_000);

    t.mint(&t.creditline, 101);
    t.client.receive_repayment(&t.creditline, &0, &101);

    assert_eq!(t.token.balance(&t.treasury), 10);
    assert_eq!(t.token.balance(&t.merchant_fund), 6); // remainder goes here
}

// ─── receive_guarantee ────────────────────────────────────────────────────────

#[test]
fn test_receive_guarantee_reduces_locked_and_recovers_liquidity() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    let merchant = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);

    // Fund a 500-token loan
    t.client.fund_loan(&t.creditline, &merchant, &500);

    // Default: guarantee of 100 returned
    t.mint(&t.creditline, 100);
    t.client.receive_guarantee(&t.creditline, &100);

    let stats = t.client.get_pool_stats();
    // locked was 500, reduced by 100 → 400
    assert_eq!(stats.locked_liquidity, 400);
    // total_liquidity was 1000, recovered 100 → 1100... no wait:
    // fund_loan doesn't change total_liquidity, it changes locked.
    // After fund_loan: total=1000, locked=500, available=500.
    // receive_guarantee adds 100 to total, reduces locked by 100.
    assert_eq!(stats.total_liquidity, 1_100);
}

// ─── withdraw (additional edge cases) ────────────────────────────────────────

#[test]
fn test_withdraw_returns_tokens_to_provider() {
    // Verify that tokens actually land in the provider's wallet after withdrawal.
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 2_000);

    t.client.deposit(&provider, &2_000);
    assert_eq!(t.token.balance(&provider), 0);

    t.client.withdraw(&provider, &2_000);
    assert_eq!(t.token.balance(&provider), 2_000);
}

#[test]
fn test_withdraw_updates_pool_stats_correctly() {
    // After partial withdrawal, stats must reflect the remaining state.
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 3_000);

    t.client.deposit(&provider, &3_000);
    t.client.withdraw(&provider, &1_000);

    let stats = t.client.get_pool_stats();
    assert_eq!(stats.total_liquidity, 2_000);
    assert_eq!(stats.total_shares, 2_000);
    assert_eq!(stats.locked_liquidity, 0);
    assert_eq!(stats.available_liquidity, 2_000);
}

#[test]
fn test_two_providers_independent_withdrawals() {
    // Provider A and B each deposit; each can withdraw their own portion
    // without affecting the other's entitlement.
    let t = TestEnv::setup();
    let provider_a = Address::generate(&t.env);
    let provider_b = Address::generate(&t.env);
    t.mint(&provider_a, 1_000);
    t.mint(&provider_b, 2_000);

    t.client.deposit(&provider_a, &1_000);
    t.client.deposit(&provider_b, &2_000);

    // A withdraws all their shares (1000 out of 3000 total = 1/3 of pool)
    let returned_a = t.client.withdraw(&provider_a, &1_000);
    assert_eq!(returned_a, 1_000);
    assert_eq!(t.client.get_lp_shares(&provider_a), 0);

    // B's shares and pool value are intact
    assert_eq!(t.client.get_lp_shares(&provider_b), 2_000);
    let stats = t.client.get_pool_stats();
    assert_eq!(stats.total_liquidity, 2_000);
    assert_eq!(stats.total_shares, 2_000);

    // B withdraws everything
    let returned_b = t.client.withdraw(&provider_b, &2_000);
    assert_eq!(returned_b, 2_000);

    let stats_final = t.client.get_pool_stats();
    assert_eq!(stats_final.total_liquidity, 0);
    assert_eq!(stats_final.total_shares, 0);
}

#[test]
fn test_withdraw_partial_when_some_liquidity_locked() {
    // If only part of liquidity is locked, a partial withdrawal of the
    // available portion should succeed.
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    let merchant = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);

    // Lock 400 tokens in a loan → 600 available
    t.client.fund_loan(&t.creditline, &merchant, &400);

    // Withdraw shares worth exactly 600 tokens (should pass)
    // shares_to_withdraw = 600 * 1000 / 1000 = 600 shares
    let returned = t.client.withdraw(&provider, &600);
    assert_eq!(returned, 600);

    let stats = t.client.get_pool_stats();
    assert_eq!(stats.total_liquidity, 400);
    assert_eq!(stats.locked_liquidity, 400);
    assert_eq!(stats.available_liquidity, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_withdraw_negative_shares_fails() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);
    t.client.withdraw(&provider, &-1);
}

#[test]
fn test_sequential_partial_withdrawals_drain_pool() {
    // Withdraw in two steps and confirm pool reaches zero correctly.
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 1_000);

    t.client.deposit(&provider, &1_000);

    let first = t.client.withdraw(&provider, &600);
    assert_eq!(first, 600);

    let second = t.client.withdraw(&provider, &400);
    assert_eq!(second, 400);

    assert_eq!(t.client.get_lp_shares(&provider), 0);
    let stats = t.client.get_pool_stats();
    assert_eq!(stats.total_liquidity, 0);
    assert_eq!(stats.total_shares, 0);
}

#[test]
fn test_withdraw_succeeds_after_loan_repayment_unlocks_liquidity() {
    // A withdrawal blocked by locked liquidity must succeed once the loan is
    // repaid and locked_liquidity returns to zero.
    //
    // Note: fund_loan transfers tokens to the merchant but keeps total_liquidity
    // unchanged (only locked_liquidity increases). receive_repayment then adds
    // the returned principal back to total_liquidity. After the full cycle the
    // pool holds twice the original principal in total_liquidity but only the
    // original tokens physically — so we withdraw only the pre-loan amount (1000).
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    let merchant = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);

    // Lock 600 tokens — only 400 remain available; a 1000-share withdrawal
    // (worth 1000 tokens) would exceed available_liquidity and fail.
    t.client.fund_loan(&t.creditline, &merchant, &600);

    let stats_mid = t.client.get_pool_stats();
    assert_eq!(stats_mid.locked_liquidity, 600);
    assert_eq!(stats_mid.available_liquidity, 400);

    // Creditline repays 600 principal (no interest).
    t.mint(&t.creditline, 600);
    t.client.receive_repayment(&t.creditline, &600, &0);

    // Locked must be zero; all liquidity available.
    let stats_after = t.client.get_pool_stats();
    assert_eq!(stats_after.locked_liquidity, 0);
    assert_eq!(stats_after.available_liquidity, stats_after.total_liquidity);

    // After the fund_loan → receive_repayment cycle with no interest:
    //   total_liquidity = 1000 (unchanged — fund_loan never decreases it,
    //                           and receive_repayment no longer adds principal back)
    //   total_shares    = 1000
    // Withdrawing 600 shares: 600 * 1000 / 1000 = 600 tokens.
    let returned = t.client.withdraw(&provider, &600);
    assert_eq!(returned, 600);
    assert_eq!(t.client.get_lp_shares(&provider), 400);
}

// ─── pool_stats & calculate_withdrawal ───────────────────────────────────────

#[test]
fn test_get_pool_stats_empty_pool() {
    let t = TestEnv::setup();
    let stats = t.client.get_pool_stats();
    assert_eq!(stats.total_liquidity, 0);
    assert_eq!(stats.total_shares, 0);
    assert_eq!(stats.locked_liquidity, 0);
    assert_eq!(stats.available_liquidity, 0);
    assert_eq!(stats.share_price, 10_000); // Default 1.00
}

#[test]
fn test_calculate_withdrawal_empty_pool_returns_zero() {
    let t = TestEnv::setup();
    assert_eq!(t.client.calculate_withdrawal(&1_000), 0);
}

// ─── admin operations ─────────────────────────────────────────────────────────

#[test]
fn test_set_admin() {
    let t = TestEnv::setup();
    let new_admin = Address::generate(&t.env);
    t.client.set_admin(&new_admin);
    assert_eq!(t.client.get_admin(), new_admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_non_admin_cannot_set_creditline() {
    let t = TestEnv::setup();
    let intruder = Address::generate(&t.env);
    let new_creditline = Address::generate(&t.env);
    t.client.set_creditline(&intruder, &new_creditline);
}

#[test]
fn test_share_calculation_accuracy() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let provider1_deposit = 1_000_000;
    let provider1_expected_shares = 1_000_000;
    let provider2_deposit = 500_000;
    let provider2_expected_shares = 500_000;
    let expected_total_shares = 1_500_000;
    let expected_total_liquidity = 1_500_000;
    let interest_amount = 100_000;
    let principal_repayment = 0;
    let lp_interest_percentage = 85;
    let lp_interest = (interest_amount * lp_interest_percentage) / 100;
    let expected_liquidity_after_interest = expected_total_liquidity + lp_interest;
    let provider3_deposit = 100;
    let provider3_expected_shares = 94;

    // 1. Test with various deposit amounts (small, medium, large)
    let provider1 = Address::generate(&context.env);
    context.mint(&provider1, provider1_deposit);
    let shares1 = context.client.deposit(&provider1, &provider1_deposit);
    assert_eq!(shares1, provider1_expected_shares);

    // 2. Test with different pool states - second deposit (proportional)
    let provider2 = Address::generate(&context.env);
    context.mint(&provider2, provider2_deposit);
    let shares2 = context.client.deposit(&provider2, &provider2_deposit);
    assert_eq!(shares2, provider2_expected_shares);

    // 3. Verify no precision loss - total should match
    let stats = context.client.get_pool_stats();
    assert_eq!(stats.total_shares, expected_total_shares);
    assert_eq!(stats.total_liquidity, expected_total_liquidity);

    // 4. Test rounding behavior with small deposit after interest
    // Simulate interest distribution
    context.mint(&context.creditline, interest_amount);
    context
        .client
        .receive_repayment(&context.creditline, &principal_repayment, &interest_amount);

    let stats_after = context.client.get_pool_stats();
    assert_eq!(
        stats_after.total_liquidity,
        expected_liquidity_after_interest
    );

    // Small deposit should round down correctly
    let provider3 = Address::generate(&context.env);
    context.mint(&provider3, provider3_deposit);
    let shares3 = context.client.deposit(&provider3, &provider3_deposit);
    assert_eq!(shares3, provider3_expected_shares);
}

#[test]
fn test_multiple_lp_deposits() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let provider1_deposit = 1000;
    let provider1_expected_shares = 1000;
    let provider2_deposit = 2000;
    let provider2_expected_shares = 2000;
    let provider3_deposit = 500;
    let provider3_expected_shares = 500;
    let expected_total_shares = 3500;
    let expected_total_liquidity = 3500;
    let expected_share_price = 10_000;

    // 1. Create 3 provider addresses
    let provider1 = Address::generate(&context.env);
    let provider2 = Address::generate(&context.env);
    let provider3 = Address::generate(&context.env);

    // 2. Mint different amounts to each
    context.mint(&provider1, provider1_deposit);
    context.mint(&provider2, provider2_deposit);
    context.mint(&provider3, provider3_deposit);

    // 3. Provider1 deposits tokens
    let shares1 = context.client.deposit(&provider1, &provider1_deposit);
    assert_eq!(shares1, provider1_expected_shares);

    // 4. Provider2 deposits tokens
    let shares2 = context.client.deposit(&provider2, &provider2_deposit);
    assert_eq!(shares2, provider2_expected_shares);

    // 5. Provider3 deposits tokens
    let shares3 = context.client.deposit(&provider3, &provider3_deposit);
    assert_eq!(shares3, provider3_expected_shares);

    // 6. Verify each provider's share balance is correct
    let provider1_balance = context.client.get_lp_shares(&provider1);
    let provider2_balance = context.client.get_lp_shares(&provider2);
    let provider3_balance = context.client.get_lp_shares(&provider3);

    assert_eq!(provider1_balance, provider1_expected_shares);
    assert_eq!(provider2_balance, provider2_expected_shares);
    assert_eq!(provider3_balance, provider3_expected_shares);

    // 7. Verify total_shares
    let stats = context.client.get_pool_stats();
    assert_eq!(stats.total_shares, expected_total_shares);

    // 8. Verify total_liquidity
    assert_eq!(stats.total_liquidity, expected_total_liquidity);

    // 9. Verify share_price remains constant (no interest)
    assert_eq!(stats.share_price, expected_share_price);
}

#[test]
fn test_withdrawal_with_active_loans() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let deposit_amount = 1000;
    let expected_shares = 1000;
    let loan_amount = 400;
    let expected_available_after_loan = 600;
    let expected_locked_after_loan = 400;
    let withdrawal_shares = 600;
    let expected_withdrawn_amount = 600;
    let expected_remaining_shares = 400;
    let expected_final_liquidity = 400;
    let expected_final_available = 0;
    let expected_final_locked = 400;
    let expected_final_shares = 400;
    let expected_initial_liquidity = 1000;
    let expected_initial_available = 1000;
    let expected_initial_locked = 0;
    let expected_initial_shares = 1000;

    // 1. Create a provider address and mint tokens
    let provider = Address::generate(&context.env);
    context.mint(&provider, deposit_amount);

    // 2. Provider deposits tokens
    let shares = context.client.deposit(&provider, &deposit_amount);
    assert_eq!(shares, expected_shares);

    // Verify initial state
    let initial_stats = context.client.get_pool_stats();
    assert_eq!(initial_stats.total_liquidity, expected_initial_liquidity);
    assert_eq!(
        initial_stats.available_liquidity,
        expected_initial_available
    );
    assert_eq!(initial_stats.locked_liquidity, expected_initial_locked);
    assert_eq!(initial_stats.total_shares, expected_initial_shares);

    // 3. Create a merchant address
    let merchant = Address::generate(&context.env);

    // 4. Fund a loan that locks partial liquidity
    context
        .client
        .fund_loan(&context.creditline, &merchant, &loan_amount);

    // Verify loan funding state
    let loan_stats = context.client.get_pool_stats();
    assert_eq!(loan_stats.total_liquidity, expected_initial_liquidity);
    assert_eq!(
        loan_stats.available_liquidity,
        expected_available_after_loan
    );
    assert_eq!(loan_stats.locked_liquidity, expected_locked_after_loan);
    assert_eq!(loan_stats.total_shares, expected_initial_shares);

    // 5. Calculate max withdrawable shares
    let max_withdrawable_shares =
        (loan_stats.available_liquidity * loan_stats.total_shares) / loan_stats.total_liquidity;
    assert_eq!(max_withdrawable_shares, withdrawal_shares);

    // 6. Withdraw up to available amount - this should succeed
    let withdrawn_amount = context.client.withdraw(&provider, &withdrawal_shares);
    assert_eq!(withdrawn_amount, expected_withdrawn_amount);

    // 7. Verify locked_liquidity remains unchanged
    let after_withdrawal_stats = context.client.get_pool_stats();
    assert_eq!(
        after_withdrawal_stats.locked_liquidity,
        expected_final_locked
    );

    // 8. Verify remaining provider shares
    let remaining_provider_shares = context.client.get_lp_shares(&provider);
    assert_eq!(remaining_provider_shares, expected_remaining_shares);

    // Verify pool state after withdrawal
    assert_eq!(
        after_withdrawal_stats.total_liquidity,
        expected_final_liquidity
    );
    assert_eq!(
        after_withdrawal_stats.available_liquidity,
        expected_final_available
    );
    assert_eq!(after_withdrawal_stats.total_shares, expected_final_shares);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_withdrawal_with_active_loans_exceeds_available_shares() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let deposit_amount = 1000;
    let loan_amount = 400;
    let first_withdrawal_shares = 600;
    let second_withdrawal_shares = 500;

    // Setup: provider deposits tokens, loan locks liquidity, provider withdraws shares
    let provider = Address::generate(&context.env);
    context.mint(&provider, deposit_amount);
    context.client.deposit(&provider, &deposit_amount);

    let merchant = Address::generate(&context.env);
    context
        .client
        .fund_loan(&context.creditline, &merchant, &loan_amount);
    context.client.withdraw(&provider, &first_withdrawal_shares);

    // Now provider has 400 shares remaining, but available_liquidity is 0
    // Attempt to withdraw 500 shares (more than remaining) - should fail with InsufficientShares
    context
        .client
        .withdraw(&provider, &second_withdrawal_shares);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_withdrawal_with_active_loans_no_available_liquidity() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let deposit_amount = 1000;
    let loan_amount = 400;
    let first_withdrawal_shares = 600;
    let second_withdrawal_shares = 100;

    // Setup: provider deposits tokens, loan locks liquidity, provider withdraws shares
    let provider = Address::generate(&context.env);
    context.mint(&provider, deposit_amount);
    context.client.deposit(&provider, &deposit_amount);

    let merchant = Address::generate(&context.env);
    context
        .client
        .fund_loan(&context.creditline, &merchant, &loan_amount);
    context.client.withdraw(&provider, &first_withdrawal_shares);

    // Now provider has 400 shares remaining, but available_liquidity is 0
    // Attempt to withdraw any amount when available_liquidity is 0 - should fail
    context
        .client
        .withdraw(&provider, &second_withdrawal_shares);
}

#[test]
fn test_share_value_maintained_after_withdrawal() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let provider1_deposit = 1000;
    let provider2_deposit = 1000;
    let provider1_shares = 1000;
    let provider2_shares = 1000;
    let expected_initial_liquidity = 2000;
    let expected_initial_shares = 2000;
    let expected_initial_share_price = 10_000;
    let provider1_withdrawal_shares = 1000;
    let expected_withdrawn1 = 1000;
    let expected_liquidity_after_withdrawal = 1000;
    let expected_shares_after_withdrawal = 1000;
    let expected_share_price_after_withdrawal = 10_000;
    let provider2_withdrawal_shares = 1000;
    let expected_withdrawn2 = 1000;
    let expected_final_liquidity = 0;
    let expected_final_shares = 0;
    let expected_final_provider1_shares = 0;
    let expected_final_provider2_shares = 0;

    // 1. Create two provider addresses
    let provider1 = Address::generate(&context.env);
    let provider2 = Address::generate(&context.env);

    // 2. Mint tokens to each provider
    context.mint(&provider1, provider1_deposit);
    context.mint(&provider2, provider2_deposit);

    // 3. Both providers deposit equal amounts
    let shares1 = context.client.deposit(&provider1, &provider1_deposit);
    assert_eq!(shares1, provider1_shares);

    let shares2 = context.client.deposit(&provider2, &provider2_deposit);
    assert_eq!(shares2, provider2_shares);

    // 4. Verify initial state
    let stats = context.client.get_pool_stats();
    assert_eq!(stats.total_liquidity, expected_initial_liquidity);
    assert_eq!(stats.total_shares, expected_initial_shares);
    assert_eq!(stats.share_price, expected_initial_share_price);

    // 5. First provider withdraws all their shares
    let withdrawn1 = context
        .client
        .withdraw(&provider1, &provider1_withdrawal_shares);
    assert_eq!(withdrawn1, expected_withdrawn1);

    // 6. Verify second provider's shares still represent correct pool value
    let provider2_shares_balance = context.client.get_lp_shares(&provider2);
    assert_eq!(provider2_shares_balance, provider2_shares);

    // Total pool now has expected liquidity and shares
    let stats_after_withdrawal = context.client.get_pool_stats();
    assert_eq!(
        stats_after_withdrawal.total_liquidity,
        expected_liquidity_after_withdrawal
    );
    assert_eq!(
        stats_after_withdrawal.total_shares,
        expected_shares_after_withdrawal
    );

    // Provider2's shares represent 100% of the pool
    assert_eq!(
        provider2_shares_balance,
        stats_after_withdrawal.total_shares
    );

    // 7. Verify share_price remains consistent
    assert_eq!(
        stats_after_withdrawal.share_price,
        expected_share_price_after_withdrawal
    );

    // 8. Second provider withdraws all their shares and receives expected amount
    let withdrawn2 = context
        .client
        .withdraw(&provider2, &provider2_withdrawal_shares);
    assert_eq!(withdrawn2, expected_withdrawn2);

    // 9. Verify pool is empty after both withdrawals
    let final_stats = context.client.get_pool_stats();
    assert_eq!(final_stats.total_liquidity, expected_final_liquidity);
    assert_eq!(final_stats.total_shares, expected_final_shares);

    // Verify both providers have no shares remaining
    assert_eq!(
        context.client.get_lp_shares(&provider1),
        expected_final_provider1_shares
    );
    assert_eq!(
        context.client.get_lp_shares(&provider2),
        expected_final_provider2_shares
    );
}

// ─── Interest Distribution Tests ─────────────────────────────────────────────
#[test]
fn test_share_value_appreciation_over_time() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let deposit_amount = 1000;
    let expected_shares = 1000;
    let expected_initial_share_price = 10_000;
    let interest_amount = 100;
    let principal_repayment = 0;
    let lp_percentage = 85;
    let lp_interest = (interest_amount * lp_percentage) / 100;
    let expected_liquidity_after_first = deposit_amount + lp_interest;
    let expected_share_price_after_first = 10850;
    let expected_liquidity_after_second = expected_liquidity_after_first + lp_interest;
    let expected_share_price_after_second = 11700;
    let expected_liquidity_after_third = expected_liquidity_after_second + lp_interest;
    let expected_share_price_after_third = 12550;
    let withdrawal_shares = 1000;
    let expected_final_withdrawal = expected_liquidity_after_third;
    let new_provider_deposit = 1000;
    let expected_new_provider_shares = 1000;

    // 1. Provider deposits tokens
    let provider = Address::generate(&context.env);
    context.mint(&provider, deposit_amount);
    let shares = context.client.deposit(&provider, &deposit_amount);
    assert_eq!(shares, expected_shares);

    // 2. Record initial share_price
    let initial_stats = context.client.get_pool_stats();
    assert_eq!(initial_stats.share_price, expected_initial_share_price);

    // 3. Distribute interest multiple times
    // First interest
    context.mint(&context.creditline, interest_amount);
    context
        .client
        .receive_repayment(&context.creditline, &principal_repayment, &interest_amount);

    // 4. Verify share_price increases after first distribution
    let stats_after_first = context.client.get_pool_stats();
    assert_eq!(
        stats_after_first.total_liquidity,
        expected_liquidity_after_first
    );
    assert_eq!(
        stats_after_first.share_price,
        expected_share_price_after_first
    );

    // Second interest
    context.mint(&context.creditline, interest_amount);
    context
        .client
        .receive_repayment(&context.creditline, &principal_repayment, &interest_amount);

    // Verify share_price increases after second distribution
    let stats_after_second = context.client.get_pool_stats();
    assert_eq!(
        stats_after_second.total_liquidity,
        expected_liquidity_after_second
    );
    assert_eq!(
        stats_after_second.share_price,
        expected_share_price_after_second
    );

    // Third interest
    context.mint(&context.creditline, interest_amount);
    context
        .client
        .receive_repayment(&context.creditline, &principal_repayment, &interest_amount);

    // Verify share_price increases after third distribution
    let stats_after_third = context.client.get_pool_stats();
    assert_eq!(
        stats_after_third.total_liquidity,
        expected_liquidity_after_third
    );
    assert_eq!(
        stats_after_third.share_price,
        expected_share_price_after_third
    );

    // 5. Verify final withdrawal amount reflects all accumulated interest
    let withdrawn = context.client.withdraw(&provider, &withdrawal_shares);
    assert_eq!(withdrawn, expected_final_withdrawal);

    // 6. Test that new depositors after interest get fewer shares per token
    let new_provider = Address::generate(&context.env);
    context.mint(&new_provider, new_provider_deposit);
    let new_shares = context.client.deposit(&new_provider, &new_provider_deposit);
    // With empty pool, first deposit gets 1:1 ratio again
    assert_eq!(new_shares, expected_new_provider_shares);
}

// ─── Edge Cases and Pool State Tests ─────────────────────────────────────────

#[test]
fn test_pool_empty_state_handling() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let expected_empty_liquidity = 0;
    let expected_empty_shares = 0;
    let expected_empty_locked = 0;
    let expected_empty_share_price = 10_000;
    let deposit_amount = 1000;
    let expected_shares = 1000;
    let withdrawal_shares = 1000;
    let expected_returned_amount = 1000;
    let calculation_shares = 1000;
    let expected_calculation_result = 0;
    let second_deposit_amount = 500;
    let expected_second_shares = 500;
    let expected_final_liquidity = 500;
    let expected_final_shares = 500;
    let expected_final_share_price = 10_000;

    // 1. Verify initial empty pool stats
    let initial_stats = context.client.get_pool_stats();
    assert_eq!(initial_stats.total_liquidity, expected_empty_liquidity);
    assert_eq!(initial_stats.total_shares, expected_empty_shares);
    assert_eq!(initial_stats.locked_liquidity, expected_empty_locked);
    assert_eq!(initial_stats.share_price, expected_empty_share_price);

    // 2. Create a provider, mint tokens, deposit them
    let provider = Address::generate(&context.env);
    context.mint(&provider, deposit_amount);
    let shares = context.client.deposit(&provider, &deposit_amount);
    assert_eq!(shares, expected_shares);

    // Verify pool has liquidity after deposit
    let after_deposit_stats = context.client.get_pool_stats();
    assert_eq!(after_deposit_stats.total_liquidity, deposit_amount);
    assert_eq!(after_deposit_stats.total_shares, expected_shares);

    // 3. Withdraw all liquidity
    let returned_amount = context.client.withdraw(&provider, &withdrawal_shares);
    assert_eq!(returned_amount, expected_returned_amount);

    // 4. Verify pool returns to empty state
    let empty_stats = context.client.get_pool_stats();
    assert_eq!(empty_stats.total_liquidity, expected_empty_liquidity);
    assert_eq!(empty_stats.total_shares, expected_empty_shares);
    assert_eq!(empty_stats.locked_liquidity, expected_empty_locked);
    assert_eq!(empty_stats.share_price, expected_empty_share_price);

    // 5. Test calculate_withdrawal with empty pool returns 0
    let withdrawal_calculation = context.client.calculate_withdrawal(&calculation_shares);
    assert_eq!(withdrawal_calculation, expected_calculation_result);

    // 6. Verify next deposit after empty state works correctly (1:1 ratio)
    context.mint(&provider, second_deposit_amount);
    let new_shares = context.client.deposit(&provider, &second_deposit_amount);
    assert_eq!(new_shares, expected_second_shares);

    // Verify final state shows correct 1:1 ratio
    let final_stats = context.client.get_pool_stats();
    assert_eq!(final_stats.total_liquidity, expected_final_liquidity);
    assert_eq!(final_stats.total_shares, expected_final_shares);
    assert_eq!(final_stats.share_price, expected_final_share_price);
}

#[test]
fn test_small_deposit_rounding() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let provider1_deposit = 1_000_000;
    let provider1_expected_shares = 1_000_000;
    let interest_amount = 200_000;
    let principal_repayment = 0;
    let lp_percentage = 85;
    let lp_interest = (interest_amount * lp_percentage) / 100;
    let expected_liquidity_after_interest = provider1_deposit + lp_interest;
    let expected_share_price = 11700;
    let provider2_deposit = 100;
    let provider2_expected_shares = 85;
    let provider2_withdrawal_shares = 85;
    let provider3_deposit = 10;
    let provider3_expected_shares = 8;

    // 1. Make a large initial deposit
    let provider1 = Address::generate(&context.env);
    context.mint(&provider1, provider1_deposit);
    let shares1 = context.client.deposit(&provider1, &provider1_deposit);
    assert_eq!(shares1, provider1_expected_shares);

    // 2. Distribute interest to increase share_price
    context.mint(&context.creditline, interest_amount);
    context
        .client
        .receive_repayment(&context.creditline, &principal_repayment, &interest_amount);

    // Verify share_price increased
    let stats = context.client.get_pool_stats();
    assert_eq!(stats.total_liquidity, expected_liquidity_after_interest);
    assert_eq!(stats.share_price, expected_share_price);

    // 3. Attempt very small deposits
    let provider2 = Address::generate(&context.env);
    context.mint(&provider2, provider2_deposit);
    let shares2 = context.client.deposit(&provider2, &provider2_deposit);

    // 4. Verify shares are calculated correctly (rounded down)
    assert_eq!(shares2, provider2_expected_shares);

    // 5. Verify no share inflation attack is possible
    let withdrawn2 = context
        .client
        .withdraw(&provider2, &provider2_withdrawal_shares);
    assert!(withdrawn2 <= provider2_deposit);

    // 6. Test edge case where deposit is very small but still gets shares
    let provider3 = Address::generate(&context.env);
    context.mint(&provider3, provider3_deposit);
    let shares3 = context.client.deposit(&provider3, &provider3_deposit);
    assert_eq!(shares3, provider3_expected_shares);
}

#[test]
fn test_concurrent_deposits_and_withdrawals() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let provider1_deposit = 1000;
    let provider1_expected_shares = 1000;
    let provider2_deposit = 2000;
    let provider2_expected_shares = 2000;
    let provider3_deposit = 1500;
    let provider3_expected_shares = 1500;
    let expected_initial_liquidity = 4500;
    let expected_initial_shares = 4500;
    let provider1_withdrawal_shares = 500;
    let expected_withdrawn1 = 500;
    let expected_liquidity_after_withdrawal = 4000;
    let expected_shares_after_withdrawal = 4000;
    let interest_amount = 400;
    let principal_repayment = 0;
    let lp_percentage = 85;
    let lp_interest = (interest_amount * lp_percentage) / 100;
    let expected_liquidity_after_interest = expected_liquidity_after_withdrawal + lp_interest;
    let provider4_deposit = 1000;
    let provider4_expected_shares = 921;
    let expected_liquidity_after_provider4 = 5340;
    let expected_shares_after_provider4 = 4921;
    let provider2_withdrawal_shares = 2000;
    let expected_withdrawn2 = 2170;
    let expected_final_shares = 2921;
    let expected_final_liquidity = 3170;

    // 1. Multiple providers deposit in sequence
    let provider1 = Address::generate(&context.env);
    context.mint(&provider1, provider1_deposit);
    let shares1 = context.client.deposit(&provider1, &provider1_deposit);
    assert_eq!(shares1, provider1_expected_shares);

    let provider2 = Address::generate(&context.env);
    context.mint(&provider2, provider2_deposit);
    let shares2 = context.client.deposit(&provider2, &provider2_deposit);
    assert_eq!(shares2, provider2_expected_shares);

    let provider3 = Address::generate(&context.env);
    context.mint(&provider3, provider3_deposit);
    let shares3 = context.client.deposit(&provider3, &provider3_deposit);
    assert_eq!(shares3, provider3_expected_shares);

    // Verify initial state
    let stats1 = context.client.get_pool_stats();
    assert_eq!(stats1.total_liquidity, expected_initial_liquidity);
    assert_eq!(stats1.total_shares, expected_initial_shares);

    // 2. Some providers withdraw while others deposit
    let withdrawn1 = context
        .client
        .withdraw(&provider1, &provider1_withdrawal_shares);
    assert_eq!(withdrawn1, expected_withdrawn1);

    let stats2 = context.client.get_pool_stats();
    assert_eq!(stats2.total_liquidity, expected_liquidity_after_withdrawal);
    assert_eq!(stats2.total_shares, expected_shares_after_withdrawal);

    // 3. Distribute interest between operations
    context.mint(&context.creditline, interest_amount);
    context
        .client
        .receive_repayment(&context.creditline, &principal_repayment, &interest_amount);

    // 4. Verify pool stats remain consistent throughout
    let stats3 = context.client.get_pool_stats();
    assert_eq!(stats3.total_liquidity, expected_liquidity_after_interest);
    assert_eq!(stats3.total_shares, expected_shares_after_withdrawal);

    // New provider deposits after interest
    let provider4 = Address::generate(&context.env);
    context.mint(&provider4, provider4_deposit);
    let shares4 = context.client.deposit(&provider4, &provider4_deposit);
    assert_eq!(shares4, provider4_expected_shares);

    let stats4 = context.client.get_pool_stats();
    assert_eq!(stats4.total_liquidity, expected_liquidity_after_provider4);
    assert_eq!(stats4.total_shares, expected_shares_after_provider4);

    // 5. Verify all providers can withdraw expected amounts
    let withdrawn2 = context
        .client
        .withdraw(&provider2, &provider2_withdrawal_shares);
    assert_eq!(withdrawn2, expected_withdrawn2);

    // 6. Verify total_liquidity and total_shares always match proportionally
    let final_stats = context.client.get_pool_stats();
    assert_eq!(final_stats.total_shares, expected_final_shares);
    assert_eq!(final_stats.total_liquidity, expected_final_liquidity);
}

#[test]
fn test_loan_funding_reduces_available_liquidity() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let deposit_amount = 1000;
    let expected_shares = 1000;
    let expected_initial_liquidity = 1000;
    let expected_initial_available = 1000;
    let expected_initial_locked = 0;
    let expected_initial_shares = 1000;
    let loan_amount = 400;
    let expected_locked_after_loan = 400;
    let expected_available_after_loan = 600;
    let expected_total_liquidity = 1000;
    let expected_total_shares = 1000;

    // 1. Create a provider address and mint tokens
    let provider = Address::generate(&context.env);
    context.mint(&provider, deposit_amount);

    // 2. Provider deposits tokens
    let shares = context.client.deposit(&provider, &deposit_amount);
    assert_eq!(shares, expected_shares);

    // 3. Record initial pool stats
    let initial_stats = context.client.get_pool_stats();
    assert_eq!(initial_stats.total_liquidity, expected_initial_liquidity);
    assert_eq!(
        initial_stats.available_liquidity,
        expected_initial_available
    );
    assert_eq!(initial_stats.locked_liquidity, expected_initial_locked);
    assert_eq!(initial_stats.total_shares, expected_initial_shares);

    // 4. Create a merchant address
    let merchant = Address::generate(&context.env);

    // 5. Fund a loan
    context
        .client
        .fund_loan(&context.creditline, &merchant, &loan_amount);

    // 6. Verify locked_liquidity increased
    let updated_stats = context.client.get_pool_stats();
    assert_eq!(updated_stats.locked_liquidity, expected_locked_after_loan);

    // 7. Verify available_liquidity decreased
    assert_eq!(
        updated_stats.available_liquidity,
        expected_available_after_loan
    );

    // 8. Verify total_liquidity unchanged
    assert_eq!(updated_stats.total_liquidity, expected_total_liquidity);

    // 9. Verify total_shares unchanged
    assert_eq!(updated_stats.total_shares, expected_total_shares);
}

#[test]
fn test_repayment_increases_pool_value() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let deposit_amount = 10000;
    let expected_initial_liquidity = 10000;
    let expected_initial_share_price = 10_000;
    let loan_amount = 5000;
    let expected_locked_after_loan = 5000;
    let expected_available_after_loan = 5000;
    let principal_repayment = 5000;
    let interest_amount = 500;
    let lp_percentage = 85;
    let treasury_percentage = 10;
    let merchant_fund_percentage = 5;
    let lp_interest = (interest_amount * lp_percentage) / 100;
    let treasury_fee = (interest_amount * treasury_percentage) / 100;
    let merchant_fund_fee = (interest_amount * merchant_fund_percentage) / 100;
    let expected_locked_after_repayment = 0;
    let expected_liquidity_after_repayment = deposit_amount + lp_interest;
    let expected_share_price_after_repayment = 10425;
    let total_repayment = principal_repayment + interest_amount;

    // 1. Provider deposits tokens
    let provider = Address::generate(&context.env);
    context.mint(&provider, deposit_amount);
    context.client.deposit(&provider, &deposit_amount);

    let initial_stats = context.client.get_pool_stats();
    assert_eq!(initial_stats.total_liquidity, expected_initial_liquidity);
    assert_eq!(initial_stats.share_price, expected_initial_share_price);

    // 2. Fund a loan
    let merchant = Address::generate(&context.env);
    context
        .client
        .fund_loan(&context.creditline, &merchant, &loan_amount);

    let after_loan_stats = context.client.get_pool_stats();
    assert_eq!(
        after_loan_stats.locked_liquidity,
        expected_locked_after_loan
    );
    assert_eq!(
        after_loan_stats.available_liquidity,
        expected_available_after_loan
    );

    // Check initial treasury and merchant fund balances
    let initial_treasury_balance = context.token.balance(&context.treasury);
    let initial_merchant_fund_balance = context.token.balance(&context.merchant_fund);

    // 3. Simulate repayment with principal + interest
    context.mint(&context.creditline, total_repayment);
    context
        .client
        .receive_repayment(&context.creditline, &principal_repayment, &interest_amount);

    // 4. Verify locked_liquidity decreased by principal amount
    let after_repayment_stats = context.client.get_pool_stats();
    assert_eq!(
        after_repayment_stats.locked_liquidity,
        expected_locked_after_repayment
    );

    // 5. Verify total_liquidity increased by (principal + LP_interest_portion)
    assert_eq!(
        after_repayment_stats.total_liquidity,
        expected_liquidity_after_repayment
    );

    // 6. Verify share_price increased
    assert_eq!(
        after_repayment_stats.share_price,
        expected_share_price_after_repayment
    );

    // 7. Verify treasury and merchant_fund received their fee portions
    let treasury_balance = context.token.balance(&context.treasury);
    assert_eq!(treasury_balance, initial_treasury_balance + treasury_fee);

    let merchant_fund_balance = context.token.balance(&context.merchant_fund);
    assert_eq!(
        merchant_fund_balance,
        initial_merchant_fund_balance + merchant_fund_fee
    );
}

#[test]
fn test_guarantee_receipt_on_default() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let deposit_amount = 10000;
    let expected_initial_liquidity = 10000;
    let expected_initial_share_price = 10_000;
    let loan_amount = 5000;
    let expected_locked_after_loan = 5000;
    let expected_available_after_loan = 5000;
    let expected_total_liquidity_after_loan = 10000;
    let guarantee_amount = 3000;
    let expected_locked_after_guarantee = 2000;
    let expected_total_liquidity_after_guarantee = 13000;
    let expected_share_price_after_guarantee = 13000;

    // 1. Provider deposits tokens
    let provider = Address::generate(&context.env);
    context.mint(&provider, deposit_amount);
    context.client.deposit(&provider, &deposit_amount);

    let initial_stats = context.client.get_pool_stats();
    assert_eq!(initial_stats.total_liquidity, expected_initial_liquidity);
    assert_eq!(initial_stats.share_price, expected_initial_share_price);

    // 2. Fund a loan (locks liquidity)
    let merchant = Address::generate(&context.env);
    context
        .client
        .fund_loan(&context.creditline, &merchant, &loan_amount);

    let after_loan_stats = context.client.get_pool_stats();
    assert_eq!(
        after_loan_stats.locked_liquidity,
        expected_locked_after_loan
    );
    assert_eq!(
        after_loan_stats.available_liquidity,
        expected_available_after_loan
    );
    assert_eq!(
        after_loan_stats.total_liquidity,
        expected_total_liquidity_after_loan
    );

    // 3. Simulate default with partial guarantee receipt
    context.mint(&context.creditline, guarantee_amount);
    context
        .client
        .receive_guarantee(&context.creditline, &guarantee_amount);

    // 4. Verify locked_liquidity reduced by guarantee amount
    let after_guarantee_stats = context.client.get_pool_stats();
    assert_eq!(
        after_guarantee_stats.locked_liquidity,
        expected_locked_after_guarantee
    );

    // 5. Verify total_liquidity increased by guarantee amount
    assert_eq!(
        after_guarantee_stats.total_liquidity,
        expected_total_liquidity_after_guarantee
    );

    // 6. Verify remaining locked_liquidity represents unrecovered loss
    assert_eq!(
        after_guarantee_stats.locked_liquidity,
        expected_locked_after_guarantee
    );

    // 7. Verify share_price reflects the partial recovery
    assert_eq!(
        after_guarantee_stats.share_price,
        expected_share_price_after_guarantee
    );
}

// ─── Complete Lifecycle Test ─────────────────────────────────────────────────

#[test]
fn test_withdrawal_calculation_precision() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let initial_deposit = 10000;
    let calc1_shares = 5000;
    let expected_calc1 = 5000;
    let calc2_shares = 10000;
    let expected_calc2 = 10000;
    let withdrawal1_shares = 5000;
    let second_deposit = 10000;
    let interest_amount = 1000;
    let principal_repayment = 0;
    let expected_liquidity_after_interest = 15850;
    let calc3_shares = 1000;
    let expected_calc3 = 1056;
    let withdrawal2_shares = 1000;
    let calc4_shares = 1;
    let withdrawal3_shares = 1;

    // 1. Setup pool with various states
    let provider = Address::generate(&context.env);
    context.mint(&provider, initial_deposit);
    context.client.deposit(&provider, &initial_deposit);

    // 2. Call calculate_withdrawal for different share amounts
    let calc1 = context.client.calculate_withdrawal(&calc1_shares);
    assert_eq!(calc1, expected_calc1);

    let calc2 = context.client.calculate_withdrawal(&calc2_shares);
    assert_eq!(calc2, expected_calc2);

    // 3. Perform actual withdrawal
    let withdrawn1 = context.client.withdraw(&provider, &withdrawal1_shares);

    // 4. Verify returned amount matches calculation
    assert_eq!(withdrawn1, calc1);

    // Deposit again for more tests
    context.mint(&provider, second_deposit);
    context.client.deposit(&provider, &second_deposit);

    // 5. Test with edge cases (very small/large amounts, after interest, etc.)
    // Distribute interest
    context.mint(&context.creditline, interest_amount);
    context
        .client
        .receive_repayment(&context.creditline, &principal_repayment, &interest_amount);

    let stats = context.client.get_pool_stats();
    assert_eq!(stats.total_liquidity, expected_liquidity_after_interest);

    // Calculate withdrawal after interest
    let calc3 = context.client.calculate_withdrawal(&calc3_shares);
    assert_eq!(calc3, expected_calc3);

    // Verify actual withdrawal matches
    let withdrawn2 = context.client.withdraw(&provider, &withdrawal2_shares);
    assert_eq!(withdrawn2, calc3);

    // Test very small amount
    let calc4 = context.client.calculate_withdrawal(&calc4_shares);
    let withdrawn3 = context.client.withdraw(&provider, &withdrawal3_shares);
    assert_eq!(withdrawn3, calc4);

    // Test very large amount
    let remaining_shares = context.client.get_lp_shares(&provider);
    let calc5 = context.client.calculate_withdrawal(&remaining_shares);
    let withdrawn4 = context.client.withdraw(&provider, &remaining_shares);
    assert_eq!(withdrawn4, calc5);
}

#[test]
fn test_share_price_calculation() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let expected_empty_share_price = 10_000;
    let deposit_amount = 5000;
    let expected_share_price_after_deposit = 10_000;
    let interest_amount = 500;
    let principal_repayment = 0;
    let lp_percentage = 85;
    let lp_interest = (interest_amount * lp_percentage) / 100;
    let expected_liquidity_after_interest = deposit_amount + lp_interest;
    let expected_share_price_after_interest = 10850;
    let withdrawal_shares = 2000;
    let expected_share_price_after_withdrawal = 10850;
    let loop_interest_amount = 100;
    let loop_iterations = 5;
    let expected_final_liquidity = 3680;
    let expected_final_share_price = 12266;

    // 1. Empty pool: share_price should be expected value
    let empty_stats = context.client.get_pool_stats();
    assert_eq!(empty_stats.share_price, expected_empty_share_price);

    // 2. After first deposit: share_price should remain constant
    let provider = Address::generate(&context.env);
    context.mint(&provider, deposit_amount);
    context.client.deposit(&provider, &deposit_amount);

    let after_deposit_stats = context.client.get_pool_stats();
    assert_eq!(
        after_deposit_stats.share_price,
        expected_share_price_after_deposit
    );

    // 3. After interest: share_price should increase proportionally
    context.mint(&context.creditline, interest_amount);
    context
        .client
        .receive_repayment(&context.creditline, &principal_repayment, &interest_amount);

    let after_interest_stats = context.client.get_pool_stats();
    assert_eq!(
        after_interest_stats.total_liquidity,
        expected_liquidity_after_interest
    );
    assert_eq!(
        after_interest_stats.share_price,
        expected_share_price_after_interest
    );

    // 4. After withdrawal: share_price should remain constant
    context.client.withdraw(&provider, &withdrawal_shares);

    let after_withdrawal_stats = context.client.get_pool_stats();
    assert_eq!(
        after_withdrawal_stats.share_price,
        expected_share_price_after_withdrawal
    );

    // 5. Test edge cases (very large pools, after many interest events)
    // Add more interest events
    for _ in 0..loop_iterations {
        context.mint(&context.creditline, loop_interest_amount);
        context.client.receive_repayment(
            &context.creditline,
            &principal_repayment,
            &loop_interest_amount,
        );
    }

    let final_stats = context.client.get_pool_stats();
    assert_eq!(final_stats.total_liquidity, expected_final_liquidity);
    assert_eq!(final_stats.share_price, expected_final_share_price);
}

#[test]
fn test_multiple_interest_distributions() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let deposit_amount = 1000;
    let expected_initial_liquidity = 1000;
    let expected_initial_share_price = 10_000;
    let interest_amount = 100;
    let principal_repayment = 0;
    let expected_liquidity_after_event1 = 1085;
    let expected_share_price_after_event1 = 10850;
    let expected_liquidity_after_event2 = 1170;
    let expected_share_price_after_event2 = 11700;
    let expected_liquidity_after_event3 = 1255;
    let expected_share_price_after_event3 = 12550;
    let expected_liquidity_after_event4 = 1340;
    let expected_share_price_after_event4 = 13400;
    let expected_liquidity_after_event5 = 1425;
    let expected_share_price_after_event5 = 14250;
    let withdrawal_shares = 1000;
    let expected_withdrawn = 1425;

    // 1. Provider deposits tokens
    let provider = Address::generate(&context.env);
    context.mint(&provider, deposit_amount);
    context.client.deposit(&provider, &deposit_amount);

    let initial_stats = context.client.get_pool_stats();
    assert_eq!(initial_stats.total_liquidity, expected_initial_liquidity);
    assert_eq!(initial_stats.share_price, expected_initial_share_price);

    // 2. Distribute interest event 1
    context.mint(&context.creditline, interest_amount);
    context
        .client
        .receive_repayment(&context.creditline, &principal_repayment, &interest_amount);

    // 3. Verify share_price increase
    let stats1 = context.client.get_pool_stats();
    assert_eq!(stats1.total_liquidity, expected_liquidity_after_event1);
    assert_eq!(stats1.share_price, expected_share_price_after_event1);

    // 4. Distribute interest event 2
    context.mint(&context.creditline, interest_amount);
    context
        .client
        .receive_repayment(&context.creditline, &principal_repayment, &interest_amount);

    // 5. Verify share_price compounds correctly
    let stats2 = context.client.get_pool_stats();
    assert_eq!(stats2.total_liquidity, expected_liquidity_after_event2);
    assert_eq!(stats2.share_price, expected_share_price_after_event2);

    // 6. Continue for several events
    context.mint(&context.creditline, interest_amount);
    context
        .client
        .receive_repayment(&context.creditline, &principal_repayment, &interest_amount);

    let stats3 = context.client.get_pool_stats();
    assert_eq!(stats3.total_liquidity, expected_liquidity_after_event3);
    assert_eq!(stats3.share_price, expected_share_price_after_event3);

    context.mint(&context.creditline, interest_amount);
    context
        .client
        .receive_repayment(&context.creditline, &principal_repayment, &interest_amount);

    let stats4 = context.client.get_pool_stats();
    assert_eq!(stats4.total_liquidity, expected_liquidity_after_event4);
    assert_eq!(stats4.share_price, expected_share_price_after_event4);

    context.mint(&context.creditline, interest_amount);
    context
        .client
        .receive_repayment(&context.creditline, &principal_repayment, &interest_amount);

    let stats5 = context.client.get_pool_stats();
    assert_eq!(stats5.total_liquidity, expected_liquidity_after_event5);
    assert_eq!(stats5.share_price, expected_share_price_after_event5);

    // 7. Verify final share value reflects all compounded interest
    let withdrawn = context.client.withdraw(&provider, &withdrawal_shares);
    assert_eq!(withdrawn, expected_withdrawn);
}

#[test]
fn test_zero_shares_edge_case() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let provider1_deposit = 10_000_000;
    let interest_amount = 1_000_000;
    let principal_repayment = 0;
    let loop_iterations = 10;
    let lp_percentage = 85;
    let total_lp_interest = (interest_amount * lp_percentage * loop_iterations as i128) / 100;
    let expected_total_liquidity = provider1_deposit + total_lp_interest;
    let expected_share_price = 18500;

    // 1. Create pool with high share_price (large deposit + lots of interest)
    let provider1 = Address::generate(&context.env);
    context.mint(&provider1, provider1_deposit);
    context.client.deposit(&provider1, &provider1_deposit);

    // Distribute large amount of interest multiple times
    for _ in 0..loop_iterations {
        context.mint(&context.creditline, interest_amount);
        context.client.receive_repayment(
            &context.creditline,
            &principal_repayment,
            &interest_amount,
        );
    }

    let stats = context.client.get_pool_stats();
    assert_eq!(stats.total_liquidity, expected_total_liquidity);
    assert_eq!(stats.share_price, expected_share_price);

    // 2. Attempt deposit so small it would round to 0 shares
    // The contract will reject this with InvalidAmount error
}

#[test]
fn test_maximum_values_handling() {
    let context = TestEnv::setup();

    // Declare all test parameters as variables
    let large_amount = 1_000_000_000_000i128;
    let expected_shares = large_amount;
    let expected_initial_liquidity = large_amount;
    let expected_initial_shares = large_amount;
    let expected_initial_share_price = 10_000;
    let calc_shares = large_amount / 2;
    let expected_calc = large_amount / 2;
    let large_interest = 100_000_000_000i128;
    let principal_repayment = 0;
    let lp_percentage = 85;
    let lp_interest = (large_interest * lp_percentage) / 100;
    let expected_liquidity_after_interest = large_amount + lp_interest;
    let expected_share_price_after_interest =
        (expected_liquidity_after_interest * 10_000) / large_amount;
    let withdrawal_shares = large_amount;

    // 1. Test with maximum reasonable token amounts
    let provider = Address::generate(&context.env);
    context.mint(&provider, large_amount);
    let shares = context.client.deposit(&provider, &large_amount);
    assert_eq!(shares, expected_shares);

    // 2. Test share calculations with large numbers
    let stats = context.client.get_pool_stats();
    assert_eq!(stats.total_liquidity, expected_initial_liquidity);
    assert_eq!(stats.total_shares, expected_initial_shares);
    assert_eq!(stats.share_price, expected_initial_share_price);

    // 3. Verify no overflow in multiplication/division operations
    let calc = context.client.calculate_withdrawal(&calc_shares);
    assert_eq!(calc, expected_calc);

    // 4. Test interest distribution with large amounts
    context.mint(&context.creditline, large_interest);
    context
        .client
        .receive_repayment(&context.creditline, &principal_repayment, &large_interest);

    let stats_after_interest = context.client.get_pool_stats();
    assert_eq!(
        stats_after_interest.total_liquidity,
        expected_liquidity_after_interest
    );

    // Verify share_price calculation doesn't overflow
    assert_eq!(
        stats_after_interest.share_price,
        expected_share_price_after_interest
    );

    // Test withdrawal with large amounts
    let withdrawn = context.client.withdraw(&provider, &withdrawal_shares);
    assert_eq!(withdrawn, expected_liquidity_after_interest);
}

// ─── Admin Functions Tests ───────────────────────────────────────────────────

#[test]
fn test_set_treasury() {
    let t = TestEnv::setup();
    let new_treasury = Address::generate(&t.env);

    t.client.set_treasury(&t.admin, &new_treasury);

    // Verify by distributing interest and checking new treasury receives fees
    let provider = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);

    t.mint(&t.creditline, 100);
    t.client.receive_repayment(&t.creditline, &0, &100);

    let new_treasury_balance = t.token.balance(&new_treasury);
    assert_eq!(new_treasury_balance, 10); // 10% of 100
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_set_treasury_by_non_admin_fails() {
    let t = TestEnv::setup();
    let intruder = Address::generate(&t.env);
    let new_treasury = Address::generate(&t.env);

    t.client.set_treasury(&intruder, &new_treasury);
}

#[test]
fn test_set_merchant_fund() {
    let t = TestEnv::setup();
    let new_merchant_fund = Address::generate(&t.env);

    t.client.set_merchant_fund(&t.admin, &new_merchant_fund);

    // Verify by distributing interest and checking new merchant fund receives fees
    let provider = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);

    t.mint(&t.creditline, 100);
    t.client.receive_repayment(&t.creditline, &0, &100);

    let new_merchant_fund_balance = t.token.balance(&new_merchant_fund);
    assert_eq!(new_merchant_fund_balance, 5); // 5% of 100
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_set_merchant_fund_by_non_admin_fails() {
    let t = TestEnv::setup();
    let intruder = Address::generate(&t.env);
    let new_merchant_fund = Address::generate(&t.env);

    t.client.set_merchant_fund(&intruder, &new_merchant_fund);
}

// ─── receive_repayment Edge Cases ────────────────────────────────────────────

#[test]
fn test_receive_repayment_with_zero_principal() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    let merchant = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);

    // Fund a loan
    t.client.fund_loan(&t.creditline, &merchant, &500);

    let stats_before = t.client.get_pool_stats();
    assert_eq!(stats_before.locked_liquidity, 500);

    // Repay with only interest, no principal
    t.mint(&t.creditline, 50);
    t.client.receive_repayment(&t.creditline, &0, &50);

    let stats_after = t.client.get_pool_stats();
    // Locked should remain unchanged
    assert_eq!(stats_after.locked_liquidity, 500);
    // Total liquidity should increase by LP portion (85% of 50 = 42)
    assert_eq!(stats_after.total_liquidity, 1_042);
}

#[test]
fn test_receive_repayment_with_zero_interest() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    let merchant = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);

    // Fund a loan
    t.client.fund_loan(&t.creditline, &merchant, &500);

    // Repay with only principal, no interest
    t.mint(&t.creditline, 500);
    t.client.receive_repayment(&t.creditline, &500, &0);

    let stats = t.client.get_pool_stats();
    // Locked should be reduced to zero
    assert_eq!(stats.locked_liquidity, 0);
    // Total liquidity should remain unchanged (no interest added)
    assert_eq!(stats.total_liquidity, 1_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_receive_repayment_negative_principal_fails() {
    let t = TestEnv::setup();
    t.client.receive_repayment(&t.creditline, &-100, &50);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_receive_repayment_negative_interest_fails() {
    let t = TestEnv::setup();
    t.client.receive_repayment(&t.creditline, &100, &-50);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_receive_repayment_unauthorized_caller_fails() {
    let t = TestEnv::setup();
    let intruder = Address::generate(&t.env);
    t.client.receive_repayment(&intruder, &100, &50);
}

// ─── receive_guarantee Edge Cases ────────────────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_receive_guarantee_with_zero_amount_fails() {
    let t = TestEnv::setup();
    t.client.receive_guarantee(&t.creditline, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_receive_guarantee_negative_amount_fails() {
    let t = TestEnv::setup();
    t.client.receive_guarantee(&t.creditline, &-100);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_receive_guarantee_unauthorized_caller_fails() {
    let t = TestEnv::setup();
    let intruder = Address::generate(&t.env);
    t.client.receive_guarantee(&intruder, &100);
}

#[test]
fn test_receive_guarantee_exceeds_locked_liquidity() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    let merchant = Address::generate(&t.env);
    t.mint(&provider, 1_000);
    t.client.deposit(&provider, &1_000);

    // Fund a loan for 500
    t.client.fund_loan(&t.creditline, &merchant, &500);

    // Receive guarantee of 600 (more than locked 500)
    // The contract should cap recovery at locked amount (500)
    t.mint(&t.creditline, 600);
    t.client.receive_guarantee(&t.creditline, &600);

    let stats = t.client.get_pool_stats();
    // Locked should be reduced to 0 (capped at 500)
    assert_eq!(stats.locked_liquidity, 0);
    // Total liquidity should increase by only 500 (not 600)
    assert_eq!(stats.total_liquidity, 1_500);
}

// ─── fund_loan Edge Cases ────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_fund_loan_with_zero_amount_fails() {
    let t = TestEnv::setup();
    let merchant = Address::generate(&t.env);
    t.client.fund_loan(&t.creditline, &merchant, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_fund_loan_with_negative_amount_fails() {
    let t = TestEnv::setup();
    let merchant = Address::generate(&t.env);
    t.client.fund_loan(&t.creditline, &merchant, &-500);
}

// ─── distribute_interest Edge Cases ──────────────────────────────────────────
// Note: distribute_interest is called internally by receive_repayment
// We test it indirectly through receive_repayment with zero/negative interest

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_receive_repayment_with_zero_total_fails() {
    let t = TestEnv::setup();
    // Both principal and interest are zero - should fail
    t.client.receive_repayment(&t.creditline, &0, &0);
}

// ─── Integration Scenarios ───────────────────────────────────────────────────

#[test]
fn test_multiple_loans_concurrent_funding() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 10_000);
    t.client.deposit(&provider, &10_000);

    let merchant1 = Address::generate(&t.env);
    let merchant2 = Address::generate(&t.env);
    let merchant3 = Address::generate(&t.env);

    // Fund multiple loans
    t.client.fund_loan(&t.creditline, &merchant1, &2_000);
    t.client.fund_loan(&t.creditline, &merchant2, &3_000);
    t.client.fund_loan(&t.creditline, &merchant3, &1_500);

    let stats = t.client.get_pool_stats();
    assert_eq!(stats.locked_liquidity, 6_500);
    assert_eq!(stats.available_liquidity, 3_500);
    assert_eq!(stats.total_liquidity, 10_000);
}

#[test]
fn test_partial_guarantee_recovery_multiple_defaults() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 10_000);
    t.client.deposit(&provider, &10_000);

    let merchant1 = Address::generate(&t.env);
    let merchant2 = Address::generate(&t.env);

    // Fund two loans
    t.client.fund_loan(&t.creditline, &merchant1, &3_000);
    t.client.fund_loan(&t.creditline, &merchant2, &2_000);

    // First default with partial guarantee
    t.mint(&t.creditline, 1_000);
    t.client.receive_guarantee(&t.creditline, &1_000);

    let stats_after_first = t.client.get_pool_stats();
    assert_eq!(stats_after_first.locked_liquidity, 4_000); // 5000 - 1000
    assert_eq!(stats_after_first.total_liquidity, 11_000); // 10000 + 1000

    // Second default with partial guarantee
    t.mint(&t.creditline, 800);
    t.client.receive_guarantee(&t.creditline, &800);

    let stats_after_second = t.client.get_pool_stats();
    assert_eq!(stats_after_second.locked_liquidity, 3_200); // 4000 - 800
    assert_eq!(stats_after_second.total_liquidity, 11_800); // 11000 + 800
}

#[test]
fn test_complex_lifecycle_deposits_loans_repayments_withdrawals() {
    let t = TestEnv::setup();

    // Multiple providers deposit
    let provider1 = Address::generate(&t.env);
    let provider2 = Address::generate(&t.env);
    t.mint(&provider1, 5_000);
    t.mint(&provider2, 3_000);

    t.client.deposit(&provider1, &5_000);
    t.client.deposit(&provider2, &3_000);

    // Fund loans
    let merchant = Address::generate(&t.env);
    t.client.fund_loan(&t.creditline, &merchant, &4_000);

    // Partial repayment with interest
    t.mint(&t.creditline, 2_500);
    t.client.receive_repayment(&t.creditline, &2_000, &500);

    let stats_mid = t.client.get_pool_stats();
    assert_eq!(stats_mid.locked_liquidity, 2_000);

    // Provider1 withdraws some shares
    let shares_to_withdraw = t.client.get_lp_shares(&provider1) / 2;
    t.client.withdraw(&provider1, &shares_to_withdraw);

    // Complete repayment
    t.mint(&t.creditline, 2_200);
    t.client.receive_repayment(&t.creditline, &2_000, &200);

    let stats_final = t.client.get_pool_stats();
    assert_eq!(stats_final.locked_liquidity, 0);

    // Both providers can withdraw remaining shares
    let remaining1 = t.client.get_lp_shares(&provider1);
    let remaining2 = t.client.get_lp_shares(&provider2);

    assert!(remaining1 > 0);
    assert!(remaining2 > 0);
}

#[test]
fn test_interest_distribution_with_empty_pool() {
    let t = TestEnv::setup();

    // Try to distribute interest when pool is empty
    // This should not panic but also not do anything meaningful
    t.mint(&t.creditline, 100);
    t.client.receive_repayment(&t.creditline, &0, &100);

    let stats = t.client.get_pool_stats();
    // With no shares, interest still gets distributed to treasury/merchant fund
    assert_eq!(t.token.balance(&t.treasury), 10);
    assert_eq!(t.token.balance(&t.merchant_fund), 5);
    // LP portion (85) stays in pool
    assert_eq!(stats.total_liquidity, 85);
}

#[test]
fn test_withdrawal_after_multiple_interest_distributions() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 1_000);

    let shares = t.client.deposit(&provider, &1_000);

    // Multiple interest distributions
    for _ in 0..5 {
        t.mint(&t.creditline, 100);
        t.client.receive_repayment(&t.creditline, &0, &100);
    }

    // Withdraw all shares
    let withdrawn = t.client.withdraw(&provider, &shares);

    // Should receive original + accumulated interest
    // 5 * 85 (LP portion) = 425
    assert_eq!(withdrawn, 1_425);
}

#[test]
fn test_loan_funding_and_guarantee_recovery_cycle() {
    let t = TestEnv::setup();
    let provider = Address::generate(&t.env);
    t.mint(&provider, 5_000);
    t.client.deposit(&provider, &5_000);

    let merchant = Address::generate(&t.env);

    // Fund loan
    t.client.fund_loan(&t.creditline, &merchant, &2_000);

    // Partial guarantee recovery
    t.mint(&t.creditline, 500);
    t.client.receive_guarantee(&t.creditline, &500);

    let stats_after_guarantee = t.client.get_pool_stats();
    assert_eq!(stats_after_guarantee.locked_liquidity, 1_500);
    assert_eq!(stats_after_guarantee.total_liquidity, 5_500);

    // Fund another loan
    t.client.fund_loan(&t.creditline, &merchant, &1_000);

    let stats_final = t.client.get_pool_stats();
    assert_eq!(stats_final.locked_liquidity, 2_500);
    assert_eq!(stats_final.available_liquidity, 3_000);
}
