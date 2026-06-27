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
    /// * `admin`        – Contract administrator (can update addresses)
    /// * `token`        – SEP-41 token used by the pool (e.g. USDC)
    /// * `treasury`     – Address that receives the 10% protocol fee
    /// * `merchant_fund`– Address that receives the 5% merchant incentive fee
    pub fn initialize(
        env: Env,
        admin: Address,
        token: Address,
        treasury: Address,
        merchant_fund: Address,
    ) {
        if storage::has_admin(&env) {
            panic_with_error!(&env, LiquidityPoolError::AlreadyInitialized);
        }
        admin.require_auth();

        storage::set_admin(&env, &admin);
        storage::set_token(&env, &token);
        storage::set_treasury(&env, &treasury);
        storage::set_merchant_fund(&env, &merchant_fund);
    }

    // -------------------------------------------------------------------------
    // Admin setters
    // -------------------------------------------------------------------------

    pub fn set_creditline(env: Env, admin: Address, creditline: Address) {
        admin.require_auth();
        Self::require_admin(&env, &admin);
        storage::set_creditline(&env, &creditline);
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

    /// Upgrade the contract WASM — admin only
    pub fn upgrade(env: Env, new_wasm_hash: soroban_sdk::BytesN<32>) {
        let admin = storage::get_admin(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        admin.require_auth();
        // bump and persist version
        let old_version = storage::get_version(&env).unwrap_or(1u32);
        let new_version = old_version.checked_add(1).unwrap_or(old_version);
        storage::set_version(&env, new_version);

        env.deployer().update_current_contract_wasm(new_wasm_hash);
        events::emit_contract_upgraded(&env, old_version, new_version);
    }
    pub fn get_admin(env: Env) -> Result<Address, LiquidityPoolError> {
        storage::get_admin(&env)
    }

    pub fn get_version(env: Env) -> u32 {
        storage::get_version(&env).unwrap_or_else(|err| panic_with_error!(&env, err))
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

        if amount < types::MIN_AMOUNT {
            return Err(LiquidityPoolError::InvalidAmount);
        }

        Self::enter_non_reentrant(&env);

        let share_price = Self::calculate_share_price_internal(&env)?;
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

        // Update total shares
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

        if shares < types::MIN_AMOUNT {
            return Err(LiquidityPoolError::InvalidAmount);
        }

        Self::enter_non_reentrant(&env);

        let provider_shares = storage::get_lp_shares(&env, &provider)
            .unwrap_or_else(|err| panic_with_error!(&env, err));
        if provider_shares < shares {
            return Err(LiquidityPoolError::InsufficientShares);
        }

        let share_price = Self::calculate_share_price_internal(&env)?;
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
    pub fn fund_loan(
        env: Env,
        creditline: Address,
        merchant: Address,
        amount: i128,
    ) -> Result<(), LiquidityPoolError> {
        creditline.require_auth();
        Self::require_creditline(&env, &creditline);

        if amount <= 0 {
            return Err(LiquidityPoolError::InvalidAmount);
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

        let new_locked = safe_math::add_i128(locked_liquidity, amount)?;
        storage::set_locked_liquidity(&env, new_locked);

        // Transfer tokens from pool to merchant after accounting has been updated.
        let token = storage::get_token(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &merchant, &amount);

        events::emit_loan_funded(&env, &creditline, amount);
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

        if principal < 0 || interest < 0 {
            return Err(LiquidityPoolError::InvalidAmount);
        }

        let total = safe_math::add_i128(principal, interest)?;

        if total <= 0 {
            return Err(LiquidityPoolError::InvalidAmount);
        }
        Self::enter_non_reentrant(&env);

        // Decrease locked liquidity by the principal
        let locked =
            storage::get_locked_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let new_locked = safe_math::sub_i128(locked, principal)?;
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

    /// Distribute `interest_amount` according to the protocol fee split:
    ///   - 85 % → Liquidity Providers  (increases share value by raising `total_liquidity`)
    ///   - 10 % → Protocol Treasury
    ///   -  5 % → Merchant Incentive Fund
    ///
    /// The LP portion is NOT transferred out; it stays in the pool and inflates
    /// the share price (existing LP shares become worth more).
    ///
    /// This function is called internally by `receive_repayment`, but it is also
    /// `pub` so that the CreditLine (or admin, in edge-case) can call it directly.
    pub fn distribute_interest(env: Env, interest_amount: i128) -> Result<(), LiquidityPoolError> {
        Self::enter_non_reentrant(&env);
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
    /// Fee split (same as `distribute_interest`):
    ///   - 85 % → Liquidity Providers (share price increase)
    ///   - 10 % → Protocol Treasury
    ///   -  5 % → Merchant Incentive Fund
    pub fn accumulate_interest(env: Env, interest_amount: i128) -> Result<(), LiquidityPoolError> {
        Self::enter_non_reentrant(&env);
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

        // 85% stays in the pool → increases share value
        let lp_amount = safe_math::div_i128(
            safe_math::mul_i128(interest_amount, types::LP_FEE_BPS)?,
            types::TOTAL_BPS,
        )?;

        // 10% → treasury
        let protocol_amount = safe_math::div_i128(
            safe_math::mul_i128(interest_amount, types::PROTOCOL_FEE_BPS)?,
            types::TOTAL_BPS,
        )?;

        // 5% → merchant fund (use remainder to avoid rounding dust)
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

        // LP portion (lp_amount) stays in the pool — no transfer needed.
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

    fn calculate_share_price_internal(env: &Env) -> Result<i128, LiquidityPoolError> {
        let total_shares = storage::get_total_shares(env)?;
        let total_liquidity = storage::get_total_liquidity(env)?;
        if total_shares == 0 || total_liquidity == 0 {
            return Ok(types::SHARE_PRICE_PRECISION);
        }
        safe_math::div_i128(
            safe_math::mul_i128(total_liquidity, types::SHARE_PRICE_PRECISION)?,
            total_shares,
        )
    }

    fn require_admin(env: &Env, caller: &Address) {
        let admin = storage::get_admin(env).unwrap_or_else(|err| panic_with_error!(env, err));
        if admin != *caller {
            panic_with_error!(env, LiquidityPoolError::NotAdmin);
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
