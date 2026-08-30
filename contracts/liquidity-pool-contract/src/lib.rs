#![no_std]
use soroban_sdk::{contract, contractimpl, panic_with_error, token, Address, Env};

mod errors;
mod events;
mod safe_math;
mod storage;
mod types;

pub use errors::LiquidityPoolError;
pub use types::PoolStats;

#[contract]
pub struct LiquidityPoolContract;

#[contractimpl]
impl LiquidityPoolContract {
    // -------------------------------------------------------------------------
    // Initialization
    // -------------------------------------------------------------------------

    /// Initialize the contract. Can only be called once.
    ///
    /// * `admin`          ΓÇô Contract administrator (can update addresses/caps)
    /// * `token`          ΓÇô SEP-41 token used by the pool (e.g. USDC)
    /// * `treasury`       ΓÇô Address that receives the 10% protocol fee
    /// * `merchant_fund`  ΓÇô Address that receives the 5% merchant incentive fee
    /// * `vendor_registry`ΓÇô Optional registered vendor-registry contract. When
    ///   set, `fund_loan` requires the recipient merchant to be an active
    ///   (approved) vendor before transferring. When `None`, the check is
    ///   skipped (backward compatible).
    pub fn initialize(
        env: Env,
        admin: Address,
        token: Address,
        treasury: Address,
        merchant_fund: Address,
        vendor_registry: Option<Address>,
    ) {
        if storage::has_admin(&env) {
            panic_with_error!(&env, LiquidityPoolError::AlreadyInitialized);
        }
        admin.require_auth();

        storage::set_admin(&env, &admin);
        storage::set_token(&env, &token);
        storage::set_treasury(&env, &treasury);
        storage::set_merchant_fund(&env, &merchant_fund);
        storage::set_vendor_registry(&env, &vendor_registry);
    }

    // -------------------------------------------------------------------------
    // Admin setters
    // -------------------------------------------------------------------------

    pub fn set_creditline(env: Env, admin: Address, creditline: Address) {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        storage::set_creditline(&env, &creditline);
    }

    /// Configure the per-ledger outflow cap (basis points of available liquidity).
    /// `0` = cap disabled (only the available-liquidity check applies);
    /// `1..=10_000` caps cumulative `fund_loan` outflows within a single ledger
    /// to that fraction of available liquidity. Admin only.
    pub fn set_outflow_cap_bps(env: Env, admin: Address, bps: u32) {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        if bps > 10_000 {
            panic_with_error!(&env, LiquidityPoolError::InvalidCap);
        }
        storage::set_outflow_cap_bps(&env, bps);
        events::emit_caps_updated(&env, &admin, bps, storage::get_merchant_exposure_cap(&env));
    }

    /// Configure the cumulative single-recipient exposure ceiling (token units).
    /// `0` = cap disabled. Admin only.
    pub fn set_merchant_exposure_cap(env: Env, admin: Address, amount: i128) {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        if amount < 0 {
            panic_with_error!(&env, LiquidityPoolError::InvalidCap);
        }
        storage::set_merchant_exposure_cap(&env, amount);
        events::emit_caps_updated(&env, &admin, storage::get_outflow_cap_bps(&env), amount);
    }

    /// Set or clear (`None`) the vendor-registry contract used to cross-check
    /// recipients before funding. Admin only.
    pub fn set_vendor_registry(env: Env, admin: Address, registry: Option<Address>) {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        storage::set_vendor_registry(&env, &registry);
        events::emit_vendor_registry_updated(&env, &admin, &registry);
    }

    pub fn set_treasury(env: Env, admin: Address, treasury: Address) {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        storage::set_treasury(&env, &treasury);
    }

    pub fn set_merchant_fund(env: Env, admin: Address, merchant_fund: Address) {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        storage::set_merchant_fund(&env, &merchant_fund);
    }

    pub fn set_admin(env: Env, new_admin: Address) {
        let old_admin = storage::get_admin(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        old_admin.require_auth();
        Self::require_admin(&env, &old_admin);
        storage::set_admin(&env, &new_admin);
    }

    pub fn set_parameters_contract(env: Env, address: Address) {
        let admin = storage::get_admin(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        admin.require_auth();
        storage::set_parameters_contract(&env, &address);
    }

    /// Propose a timelocked contract WASM upgrade ΓÇö admin only
    pub fn propose_upgrade(env: Env, new_wasm_hash: soroban_sdk::BytesN<32>) {
        let admin = storage::get_admin(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        admin.require_auth();

        let delay = Self::get_upgrade_delay_seconds(&env);
        let proposed_at = env.ledger().timestamp();
        let unlock_at = proposed_at
            .checked_add(delay)
            .unwrap_or_else(|| panic_with_error!(&env, LiquidityPoolError::Overflow));

        let pending = types::PendingUpgrade {
            wasm_hash: new_wasm_hash.clone(),
            proposed_at,
            unlock_at,
        };
        storage::set_pending_upgrade(&env, &pending);
        events::emit_upgrade_proposed(&env, &new_wasm_hash, proposed_at, unlock_at);
    }

    /// Execute a previously proposed and timelocked contract WASM upgrade ΓÇö admin only
    pub fn execute_upgrade(env: Env, new_wasm_hash: soroban_sdk::BytesN<32>) {
        let admin = storage::get_admin(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        admin.require_auth();

        let pending = storage::get_pending_upgrade(&env)
            .unwrap_or_else(|err| panic_with_error!(&env, err))
            .ok_or(LiquidityPoolError::UpgradeNotProposed)
            .unwrap_or_else(|err| panic_with_error!(&env, err));

        if pending.wasm_hash != new_wasm_hash {
            panic_with_error!(&env, LiquidityPoolError::UpgradeHashMismatch);
        }

        let now = env.ledger().timestamp();
        if now < pending.unlock_at {
            panic_with_error!(&env, LiquidityPoolError::UpgradeTimelockNotMet);
        }

        let old_version = storage::get_version(&env).unwrap_or(1u32);
        let new_version = old_version
            .checked_add(1)
            .ok_or(LiquidityPoolError::Overflow)
            .unwrap_or_else(|err| panic_with_error!(&env, err));
        storage::set_version(&env, new_version);

        storage::clear_pending_upgrade(&env);
        env.deployer().update_current_contract_wasm(new_wasm_hash);
        events::emit_contract_upgraded(&env, old_version, new_version);
    }

    /// Upgrade the contract WASM ΓÇö admin only (enforces prior propose_upgrade timelock)
    pub fn upgrade(env: Env, new_wasm_hash: soroban_sdk::BytesN<32>) {
        Self::execute_upgrade(env, new_wasm_hash);
    }
    pub fn get_admin(env: Env) -> Result<Address, LiquidityPoolError> {
        storage::get_admin(&env)
    }

    pub fn get_version(env: Env) -> u32 {
        storage::get_version(&env).unwrap_or_else(|err| panic_with_error!(&env, err))
    }

    pub fn pause(env: Env, admin: Address) {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        storage::set_paused(&env, true);
        events::emit_paused(&env, &admin);
    }

    pub fn unpause(env: Env, admin: Address) {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        storage::set_paused(&env, false);
        events::emit_unpaused(&env, &admin);
    }

    pub fn is_paused(env: Env) -> bool {
        storage::is_paused(&env)
    }

    // -------------------------------------------------------------------------
    // LP Operations
    // -------------------------------------------------------------------------

    /// Deposit `amount` tokens and receive shares representing pool ownership.
    ///
    /// Shares are issued at the current share price:
    /// `shares = (amount × PRECISION) / share_price`
    ///
    /// For the first deposit share_price == PRECISION, so `shares == amount`.
    ///
    /// Returns the number of shares issued.
    pub fn deposit(env: Env, provider: Address, amount: i128) -> Result<i128, LiquidityPoolError> {
        provider.require_auth();
        Self::require_not_paused(&env);

        if amount < types::MIN_AMOUNT {
            return Err(LiquidityPoolError::InvalidAmount);
        }

        Self::enter_non_reentrant(&env);

        // Seed dead (unclaimable) shares on the very first deposit so a dust
        // depositor cannot own 100 % of a yield-bearing pool.
        let total_shares = storage::get_total_shares(&env)
            .unwrap_or_else(|err| panic_with_error!(&env, err));
        let is_first_deposit = total_shares == 0;

        if is_first_deposit {
            // Mint dead (unclaimable) shares to the contract itself — nobody can
            // withdraw them because `withdraw` requires `provider.require_auth()`.
            //
            // The dead shares are backed by an equal amount of *virtual* liquidity
            // that is NOT stored in `total_liquidity` but is added in
            // `calculate_share_price_internal` as `total_liquidity + DEAD_SHARES_AMOUNT`.
            // This keeps the share price at PRECISION for the first honest depositor
            // (1:1 share → token) so they are NOT taxed, while keeping visible
            // `total_liquidity` equal to real tokens (preserving honest-path stats).
            // The dead shares still prevent a dust depositor from owning 100% of yield.
            storage::set_total_shares(&env, types::DEAD_SHARES_AMOUNT);
        }

        // With virtual backing the price is (total_liquidity+DEAD)*PRECISION/total_shares,
        // which for the first deposit is (0+1000)*10000/1000 = 10000 (1:1).
        let share_price = Self::calculate_share_price_internal(&env)?;
        // Floor-division: depositor always receives fewer shares, never more.
        let shares_issued = safe_math::div_i128(
            safe_math::mul_i128(amount, types::SHARE_PRICE_PRECISION)?,
            share_price,
        )?;

        if shares_issued <= 0 {
            return Err(LiquidityPoolError::InvalidAmount);
        }

        let token = storage::get_token(&env).unwrap_or_else(|err| panic_with_error!(&env, err));

        // Update provider's shares
        let provider_shares = storage::get_lp_shares(&env, &provider)
            .unwrap_or_else(|err| panic_with_error!(&env, err));
        let new_shares = safe_math::add_i128(provider_shares, shares_issued)?;
        storage::set_lp_shares(&env, &provider, new_shares);

        // Update total shares (includes dead shares)
        let total_shares = storage::get_total_shares(&env)
            .unwrap_or_else(|err| panic_with_error!(&env, err));
        let new_total_shares = safe_math::add_i128(total_shares, shares_issued)?;
        storage::set_total_shares(&env, new_total_shares);

        // Update total liquidity
        let total_liquidity = storage::get_total_liquidity(&env)
            .unwrap_or_else(|err| panic_with_error!(&env, err));
        let new_total_liquidity = safe_math::add_i128(total_liquidity, amount)?;
        storage::set_total_liquidity(&env, new_total_liquidity);

        // Transfer tokens from provider to pool
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&provider, &env.current_contract_address(), &amount);

        events::emit_liquidity_deposited(&env, &provider, amount, shares_issued);
        Self::exit_non_reentrant(&env);

        Ok(shares_issued)
    }

    /// Burn `shares` and return the proportional token amount to `provider`.
    ///
    /// `amount = (shares × share_price) / PRECISION`
    ///
    /// Returns the number of tokens returned.
    pub fn withdraw(env: Env, provider: Address, shares: i128) -> Result<i128, LiquidityPoolError> {
        provider.require_auth();
        Self::require_not_paused(&env);

        if shares <= 0 {
            return Err(LiquidityPoolError::InvalidAmount);
        }

        Self::enter_non_reentrant(&env);

        let provider_shares = storage::get_lp_shares(&env, &provider)
            .unwrap_or_else(|err| panic_with_error!(&env, err));
        if provider_shares < shares {
            return Err(LiquidityPoolError::InsufficientShares);
        }

        let share_price = Self::calculate_share_price_internal(&env)?;
        // Floor-division: the pool always keeps the rounding remainder —
        // the provider receives slightly fewer tokens, never more.
        let amount_returned = safe_math::div_i128(
            safe_math::mul_i128(shares, share_price)?,
            types::SHARE_PRICE_PRECISION,
        )?;

        let total_liquidity =
            storage::get_total_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let locked_liquidity =
            storage::get_locked_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let available_liquidity = safe_math::sub_i128(total_liquidity, locked_liquidity)?;

        if amount_returned > available_liquidity {
            return Err(LiquidityPoolError::InsufficientLiquidity);
        }

        // Burn shares
        let new_provider_shares = safe_math::sub_i128(provider_shares, shares)?;
        storage::set_lp_shares(&env, &provider, new_provider_shares);

        let total_shares =
            storage::get_total_shares(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let new_total_shares = safe_math::sub_i128(total_shares, shares)?;
        storage::set_total_shares(&env, new_total_shares);

        let new_total_liquidity = safe_math::sub_i128(total_liquidity, amount_returned)?;
        storage::set_total_liquidity(&env, new_total_liquidity);

        events::emit_liquidity_withdrawn(&env, &provider, shares, amount_returned);
        // Transfer tokens back to provider after state effects.
        let token = storage::get_token(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &provider, &amount_returned);
        Self::exit_non_reentrant(&env);

        Ok(amount_returned)
    }

    // -------------------------------------------------------------------------
    // CreditLine Operations (access-restricted)
    // -------------------------------------------------------------------------

    /// Transfer `amount` tokens to `merchant` to fund a loan.
    /// Only the registered CreditLine contract may call this.
    ///
    /// Defense-in-depth guards (layered on top of the existing
    /// `require_creditline` restriction, which is kept):
    ///
    /// 1. Optional vendor cross-check: if a vendor registry is configured,
    ///    `merchant` must be an active (approved) vendor.
    /// 2. Per-ledger outflow cap: cumulative `fund_loan` outflows within the
    ///    current ledger are bounded to a configurable fraction of available
    ///    liquidity. The window rolls automatically on ledger change.
    /// 3. Single-recipient concentration cap: cumulative funding to a single
    ///    merchant is bounded by a configurable ceiling.
    pub fn fund_loan(
        env: Env,
        creditline: Address,
        merchant: Address,
        amount: i128,
    ) -> Result<(), LiquidityPoolError> {
        creditline.require_auth();
        Self::require_not_paused(&env);
        Self::require_creditline(&env, &creditline);

        if amount <= 0 {
            return Err(LiquidityPoolError::InvalidAmount);
        }

        // Optional vendor cross-check (skipped entirely when no registry set).
        if let Some(registry) = storage::get_vendor_registry(&env)
            .unwrap_or_else(|err| panic_with_error!(&env, err))
        {
            if !Self::vendor_is_active(&env, &registry, &merchant) {
                return Err(LiquidityPoolError::VendorNotActive);
            }
        }

        Self::enter_non_reentrant(&env);

        let total_liquidity =
            storage::get_total_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let locked_liquidity =
            storage::get_locked_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let available = safe_math::sub_i128(total_liquidity, locked_liquidity)?;

        if amount > available {
            return Err(LiquidityPoolError::InsufficientLiquidity);
        }

        // Per-ledger outflow cap with rolling window reset. Disabled when the
        // configured cap is zero. The window is keyed to the current ledger
        // sequence: entering a new ledger resets the used counter.
        let mut outflow_remaining = 0_i128;
        let outflow_cap_bps = storage::get_outflow_cap_bps(&env);
        if outflow_cap_bps > 0 {
            let outflow_cap = safe_math::div_i128(
                safe_math::mul_i128(available, outflow_cap_bps as i128)?,
                types::TOTAL_BPS,
            )?;
            let current_seq = env.ledger().sequence();
            let (window_seq, mut used) = storage::get_outflow_window(&env);
            if window_seq != current_seq {
                used = 0;
            }
            let new_used = safe_math::add_i128(used, amount)?;
            if new_used > outflow_cap {
                return Err(LiquidityPoolError::OutflowCapExceeded);
            }
            storage::set_outflow_window(&env, current_seq, new_used);
            outflow_remaining = safe_math::sub_i128(outflow_cap, new_used)?;
        }

        // Single-recipient concentration cap (cumulative, never reset).
        // Disabled when the configured ceiling is zero. Cumulative exposure is
        // tracked regardless so admins can monitor it via `get_merchant_funded`.
        let mut merchant_remaining = 0_i128;
        let exposure_cap = storage::get_merchant_exposure_cap(&env);
        let funded_so_far = storage::get_merchant_funded(&env, &merchant)
            .unwrap_or_else(|err| panic_with_error!(&env, err));
        let new_funded = safe_math::add_i128(funded_so_far, amount)?;
        if exposure_cap > 0 {
            if new_funded > exposure_cap {
                return Err(LiquidityPoolError::MerchantExposureCapExceeded);
            }
            merchant_remaining = safe_math::sub_i128(exposure_cap, new_funded)?;
        }
        storage::set_merchant_funded(&env, &merchant, new_funded);

        let new_locked = safe_math::add_i128(locked_liquidity, amount)?;
        storage::set_locked_liquidity(&env, new_locked);

        // Transfer tokens from pool to merchant after accounting has been updated.
        let token = storage::get_token(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &merchant, &amount);

        events::emit_loan_funded(
            &env,
            &creditline,
            &merchant,
            amount,
            outflow_remaining,
            merchant_remaining,
        );
        Self::exit_non_reentrant(&env);
        Ok(())
    }

    /// Receive a loan repayment (principal + interest) from CreditLine.
    ///
    /// `principal` reduces locked_liquidity (loan is repaid).
    /// `interest`  is distributed via `distribute_interest` (increases pool value).
    pub fn receive_repayment(
        env: Env,
        creditline: Address,
        principal: i128,
        interest: i128,
    ) -> Result<(), LiquidityPoolError> {
        creditline.require_auth();
        Self::require_creditline(&env, &creditline);

        // Repayment Exception Policy:
        // Loan repayments intentionally bypass the contract pause check.
        // This ensures borrowers are not blocked from settling loans or penalized
        // with accrued fees during an emergency administrative pause.

        if principal < 0 || interest < 0 {
            return Err(LiquidityPoolError::InvalidAmount);
        }

        let total = safe_math::add_i128(principal, interest)?;

        if total <= 0 {
            return Err(LiquidityPoolError::InvalidAmount);
        }
        Self::enter_non_reentrant(&env);

        // Decrease locked liquidity by the principal (capped at 0)
        let locked =
            storage::get_locked_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let new_locked = 0_i128.max(safe_math::sub_i128(locked, principal)?);
        storage::set_locked_liquidity(&env, new_locked);

        // Pull funds from CreditLine after accounting changes.
        let token = storage::get_token(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&creditline, &env.current_contract_address(), &total);

        events::emit_repayment_received(&env, &creditline, principal, interest);

        if interest > 0 {
            Self::distribute_interest_internal(&env, interest)?;
        }
        Self::exit_non_reentrant(&env);
        Ok(())
    }

    /// Receive a forfeited guarantee on loan default.
    /// The amount offsets the loss: it is added back to total_liquidity
    /// and reduces locked_liquidity by the same amount (partial recovery).
    pub fn receive_guarantee(
        env: Env,
        creditline: Address,
        amount: i128,
    ) -> Result<(), LiquidityPoolError> {
        creditline.require_auth();
        Self::require_not_paused(&env);
        Self::require_creditline(&env, &creditline);

        if amount <= 0 {
            return Err(LiquidityPoolError::InvalidAmount);
        }
        Self::enter_non_reentrant(&env);

        // The defaulted loan principal stays "locked" — the guarantee partially
        // covers the loss.  We reduce locked_liquidity by the guarantee amount
        // and add it back to total_liquidity (net pool recovers that portion).
        let locked =
            storage::get_locked_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let recovered = amount.min(locked); // can't recover more than locked
        let new_locked = safe_math::sub_i128(locked, recovered)?;
        storage::set_locked_liquidity(&env, new_locked);

        let total_liquidity =
            storage::get_total_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let new_total = safe_math::add_i128(total_liquidity, recovered)?;
        storage::set_total_liquidity(&env, new_total);

        let token = storage::get_token(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&creditline, &env.current_contract_address(), &amount);

        events::emit_guarantee_received(&env, &creditline, amount);
        Self::exit_non_reentrant(&env);
        Ok(())
    }

    /// Absorb an unrecovered principal shortfall from a defaulted loan.
    /// Reduces both `locked_liquidity` and `total_liquidity` so that
    /// `get_share_price()` immediately reflects the realized loss.
    /// Only the registered CreditLine contract may call this.
    pub fn absorb_loss(
        env: Env,
        creditline: Address,
        principal_shortfall: i128,
    ) -> Result<(), LiquidityPoolError> {
        creditline.require_auth();
        Self::require_not_paused(&env);
        Self::require_creditline(&env, &creditline);

        if principal_shortfall <= 0 {
            return Err(LiquidityPoolError::InvalidAmount);
        }

        Self::enter_non_reentrant(&env);

        let locked =
            storage::get_locked_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let total_liquidity =
            storage::get_total_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));

        // Cap reductions at current values to prevent negative accounting.
        let locked_reduction = principal_shortfall.min(locked);
        let total_reduction = principal_shortfall.min(total_liquidity);

        let new_locked = safe_math::sub_i128(locked, locked_reduction)?;
        storage::set_locked_liquidity(&env, new_locked);

        let new_total = safe_math::sub_i128(total_liquidity, total_reduction)?;
        storage::set_total_liquidity(&env, new_total);

        events::emit_loss_absorbed(&env, &creditline, principal_shortfall);
        Self::exit_non_reentrant(&env);
        Ok(())
    }

    /// Distribute `interest_amount` according to the protocol fee split:
    ///   - 85 % → Liquidity Providers  (increases share value by raising `total_liquidity`)
    ///   - 10 % → Protocol Treasury
    ///   -  5 % → Merchant Incentive Fund
    ///
    /// The LP portion is NOT transferred out; it stays in the pool and inflates
    /// the share price (existing LP shares become worth more).
    ///
    /// Only the registered CreditLine contract may call this function.
    /// The caller must transfer `interest_amount` tokens into the pool before
    /// the accounting change occurs, ensuring the share price rise is backed
    /// by real tokens.
    pub fn distribute_interest(
        env: Env,
        creditline: Address,
        interest_amount: i128,
    ) -> Result<(), LiquidityPoolError> {
        creditline.require_auth();
        Self::require_not_paused(&env);
        Self::require_creditline(&env, &creditline);

        Self::enter_non_reentrant(&env);

        // Pull tokens from the creditline contract into the pool before
        // any accounting change, so share price cannot rise without backing.
        let token = storage::get_token(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&creditline, &env.current_contract_address(), &interest_amount);

        let res = Self::distribute_interest_internal(&env, interest_amount);
        Self::exit_non_reentrant(&env);
        res
    }

    /// Accrue interest into the pool, increasing share price for all holders.
    ///
    /// This is a public alias for `distribute_interest` that makes the yield
    /// mechanism explicit: calling this raises `total_liquidity` (by the LP
    /// portion after fee split), which increases the share price for every
    /// LP pro-rata.
    ///
    /// Only the registered CreditLine contract may call this function.
    /// The caller must transfer `interest_amount` tokens into the pool before
    /// the accounting change occurs.
    ///
    /// Fee split (same as `distribute_interest`):
    ///   - 85 % → Liquidity Providers (share price increase)
    ///   - 10 % → Protocol Treasury
    ///   -  5 % → Merchant Incentive Fund
    pub fn accumulate_interest(
        env: Env,
        creditline: Address,
        interest_amount: i128,
    ) -> Result<(), LiquidityPoolError> {
        creditline.require_auth();
        Self::require_not_paused(&env);
        Self::require_creditline(&env, &creditline);

        Self::enter_non_reentrant(&env);

        // Pull tokens from the creditline contract into the pool before
        // any accounting change, so share price cannot rise without backing.
        let token = storage::get_token(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&creditline, &env.current_contract_address(), &interest_amount);

        let res = Self::distribute_interest_internal(&env, interest_amount);
        Self::exit_non_reentrant(&env);
        res
    }

    fn distribute_interest_internal(
        env: &Env,
        interest_amount: i128,
    ) -> Result<(), LiquidityPoolError> {
        if interest_amount <= 0 {
            return Err(LiquidityPoolError::InvalidAmount);
        }
        debug_assert_eq!(
            types::LP_FEE_BPS + types::PROTOCOL_FEE_BPS + types::MERCHANT_FEE_BPS,
            types::TOTAL_BPS
        );

        // 85% stays in the pool ΓåÆ increases share value
        let lp_amount = safe_math::div_i128(
            safe_math::mul_i128(interest_amount, types::LP_FEE_BPS)?,
            types::TOTAL_BPS,
        )?;

        // 10% ΓåÆ treasury
        let protocol_amount = safe_math::div_i128(
            safe_math::mul_i128(interest_amount, types::PROTOCOL_FEE_BPS)?,
            types::TOTAL_BPS,
        )?;

        // 5% ΓåÆ merchant fund (use remainder to avoid rounding dust)
        let merchant_amount = safe_math::sub_i128(
            safe_math::sub_i128(interest_amount, lp_amount)?,
            protocol_amount,
        )?;

        let token = storage::get_token(env).unwrap_or_else(|err| panic_with_error!(env, err));
        let token_client = token::Client::new(env, &token);

        // Transfer protocol fee to treasury (if configured)
        if protocol_amount > 0 {
            if let Some(treasury) =
                storage::get_treasury(env).unwrap_or_else(|err| panic_with_error!(env, err))
            {
                token_client.transfer(&env.current_contract_address(), &treasury, &protocol_amount);
            }
            // If treasury not configured, protocol fee stays in pool (benefits LPs)
        }

        // Transfer merchant incentive to merchant fund (if configured)
        if merchant_amount > 0 {
            if let Some(merchant_fund) =
                storage::get_merchant_fund(env).unwrap_or_else(|err| panic_with_error!(env, err))
            {
                token_client.transfer(
                    &env.current_contract_address(),
                    &merchant_fund,
                    &merchant_amount,
                );
            }
            // If merchant fund not configured, fee stays in pool (benefits LPs)
        }

        // LP portion (lp_amount) stays in the pool ΓÇö no transfer needed.
        // Update total_liquidity to reflect the added interest (raises share price).
        let total_liquidity =
            storage::get_total_liquidity(env).unwrap_or_else(|err| panic_with_error!(env, err));
        let new_total = safe_math::add_i128(total_liquidity, lp_amount)?;
        storage::set_total_liquidity(env, new_total);

        events::emit_interest_distributed(
            env,
            interest_amount,
            lp_amount,
            protocol_amount,
            merchant_amount,
        );
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Queries
    // -------------------------------------------------------------------------

    /// Return the current share price in basis points (10000 = 1.0).
    pub fn get_share_price(env: Env) -> i128 {
        Self::calculate_share_price_internal(&env)
            .unwrap_or_else(|err| panic_with_error!(&env, err))
    }

    pub fn get_pool_stats(env: Env) -> PoolStats {
        let total_liquidity =
            storage::get_total_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let locked_liquidity =
            storage::get_locked_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let available_liquidity = total_liquidity.saturating_sub(locked_liquidity);
        let total_shares =
            storage::get_total_shares(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let share_price = Self::calculate_share_price_internal(&env)
            .unwrap_or_else(|err| panic_with_error!(&env, err));

        PoolStats {
            total_liquidity,
            locked_liquidity,
            available_liquidity,
            total_shares,
            share_price,
        }
    }

    pub fn get_lp_shares(env: Env, provider: Address) -> i128 {
        storage::get_lp_shares(&env, &provider).unwrap_or_else(|err| panic_with_error!(&env, err))
    }

    /// Return the configured per-ledger outflow cap (basis points of available
    /// liquidity; 10000 = 100%).
    pub fn get_outflow_cap_bps(env: Env) -> u32 {
        storage::get_outflow_cap_bps(&env)
    }

    /// Return the configured cumulative single-recipient exposure ceiling.
    pub fn get_merchant_exposure_cap(env: Env) -> i128 {
        storage::get_merchant_exposure_cap(&env)
    }

    /// Return the optional vendor-registry contract used for recipient checks.
    pub fn get_vendor_registry(env: Env) -> Option<Address> {
        storage::get_vendor_registry(&env).unwrap_or_else(|err| panic_with_error!(&env, err))
    }

    /// Return the cumulative amount funded to a single merchant.
    pub fn get_merchant_funded(env: Env, merchant: Address) -> i128 {
        storage::get_merchant_funded(&env, &merchant)
            .unwrap_or_else(|err| panic_with_error!(&env, err))
    }

    /// Calculate how many tokens `shares` are worth at the current share price.
    pub fn calculate_withdrawal(env: Env, shares: i128) -> i128 {
        if shares == 0 {
            return 0;
        }
        let total_shares = storage::get_total_shares(&env).unwrap_or(0);
        if total_shares == 0 {
            return 0;
        }
        let share_price = Self::calculate_share_price_internal(&env).unwrap_or(types::SHARE_PRICE_PRECISION);
        safe_math::div_i128(
            safe_math::mul_i128(shares, share_price).unwrap_or(0),
            types::SHARE_PRICE_PRECISION,
        )
        .unwrap_or(0)
    }

    // -------------------------------------------------------------------------
    // Internal helpers
    // -------------------------------------------------------------------------

    fn require_not_paused(env: &Env) {
        if storage::is_paused(env) {
            panic_with_error!(env, LiquidityPoolError::ContractPaused);
        }
    }

    fn calculate_share_price_internal(env: &Env) -> Result<i128, LiquidityPoolError> {
        let total_shares = storage::get_total_shares(env)?;
        let total_liquidity = storage::get_total_liquidity(env)?;

        // Empty pool (no shares at all) ΓÇö price defaults to 1.0.
        if total_shares == 0 {
            return Ok(types::SHARE_PRICE_PRECISION);
        }

        // Virtual backing: dead shares are backed by an equal virtual liquidity
        // amount that is NOT stored, but is added here. This keeps the initial
        // price at 1:1 ( (0+1000)*10000/1000 = 10000 ) and makes post-default
        // price proportional to real remaining liquidity without hardcoding 0
        // (which would brick the next deposit). Floor to 1 prevents truncation to 0.
        let effective_liquidity = safe_math::add_i128(total_liquidity, types::DEAD_SHARES_AMOUNT)?;

        let price = safe_math::div_i128(
            safe_math::mul_i128(effective_liquidity, types::SHARE_PRICE_PRECISION)?,
            total_shares,
        )?;
        if price == 0 {
            Ok(1)
        } else {
            Ok(price)
        }
    }

    fn require_admin(env: &Env, caller: &Address) {
        let admin = storage::get_admin(env).unwrap_or_else(|err| panic_with_error!(env, err));
        if admin != *caller {
            panic_with_error!(env, LiquidityPoolError::NotAdmin);
        }
    }

    /// Cross-check a recipient against the configured vendor registry's
    /// `is_active(merchant)`. Any invocation failure resolves to `false`
    /// (fail-closed): a broken, suspended, or uninitialized registry blocks
    /// funding rather than allowing unvalidated payouts.
    fn vendor_is_active(env: &Env, registry: &Address, merchant: &Address) -> bool {
        use soroban_sdk::IntoVal;
        if let Ok(Ok(active)) = env.try_invoke_contract::<bool, soroban_sdk::Error>(
            registry,
            &soroban_sdk::Symbol::new(env, "is_active"),
            (merchant.clone(),).into_val(env),
        ) {
            active
        } else {
            false
        }
    }

    fn require_creditline(env: &Env, caller: &Address) {
        let creditline = storage::get_creditline(env)
            .unwrap_or_else(|err| panic_with_error!(env, err))
            .unwrap_or_else(|| panic_with_error!(env, LiquidityPoolError::NotCreditLine));
        if creditline != *caller {
            panic_with_error!(env, LiquidityPoolError::NotCreditLine);
        }
    }

    fn get_upgrade_delay_seconds(env: &Env) -> u64 {
        use soroban_sdk::IntoVal;
        if let Ok(Some(params_addr)) = storage::get_parameters_contract(env) {
            if let Ok(Ok(params)) = env.try_invoke_contract::<types::ProtocolParameters, soroban_sdk::Error>(
                &params_addr,
                &soroban_sdk::Symbol::new(env, "get_parameters"),
                ().into_val(env),
            ) {
                if params.upgrade_delay_seconds > 0 {
                    return params.upgrade_delay_seconds;
                }
            }
        }
        storage::DEFAULT_UPGRADE_DELAY_SECONDS
    }

    fn enter_non_reentrant(env: &Env) {
        if storage::is_reentrancy_locked(env).unwrap_or_else(|err| panic_with_error!(env, err)) {
            panic_with_error!(env, LiquidityPoolError::ReentrancyDetected);
        }
        storage::set_reentrancy_locked(env, true);
    }

    fn exit_non_reentrant(env: &Env) {
        storage::set_reentrancy_locked(env, false);
    }
}

#[cfg(test)]
mod tests;
