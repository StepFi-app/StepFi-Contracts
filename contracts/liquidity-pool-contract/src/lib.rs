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
    /// Deposit `amount` tokens and receive shares representing pool ownership.
    ///
    /// **First deposit**: shares issued == amount (1:1 ratio).
    /// **Subsequent deposits**: `shares = (amount × total_shares) / total_pool_value`
    ///
    /// Returns the number of shares issued.
    pub fn deposit(env: Env, provider: Address, amount: i128) -> Result<i128, LiquidityPoolError> {
        provider.require_auth();

        if amount < types::MIN_AMOUNT {
            return Err(LiquidityPoolError::InvalidAmount);
        }

        Self::enter_non_reentrant(&env);

        let token = storage::get_token(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let total_shares =
            storage::get_total_shares(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let total_liquidity =
            storage::get_total_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));

        // Calculate shares to issue
        let shares_issued = if total_shares == 0 || total_liquidity == 0 {
            // First deposit: 1:1 ratio
            amount
        } else {
            // Subsequent deposits: proportional to current pool value
            safe_math::div_i128(safe_math::mul_i128(amount, total_shares)?, total_liquidity)?
        };

        if shares_issued <= 0 {
            return Err(LiquidityPoolError::InvalidAmount);
        }

        // Update state
        let new_shares = safe_math::add_i128(
            storage::get_lp_shares(&env, &provider)
                .unwrap_or_else(|err| panic_with_error!(&env, err)),
            shares_issued,
        )?;
        storage::set_lp_shares(&env, &provider, new_shares);

        let new_total_shares = safe_math::add_i128(total_shares, shares_issued)?;
        storage::set_total_shares(&env, new_total_shares);

        let new_total_liquidity = safe_math::add_i128(total_liquidity, amount)?;
        storage::set_total_liquidity(&env, new_total_liquidity);

        // Transfer tokens from provider to pool contract after state effects.
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&provider, &env.current_contract_address(), &amount);

        events::emit_liquidity_deposited(&env, &provider, amount, shares_issued);
        Self::exit_non_reentrant(&env);

        Ok(shares_issued)
    }

    /// Burn `shares` and return the proportional token amount to `provider`.
    ///
    /// `amount = (shares × total_pool_value) / total_shares`
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

        let total_shares =
            storage::get_total_shares(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        if total_shares == 0 {
            return Err(LiquidityPoolError::ZeroTotalShares);
        }

        let total_liquidity =
            storage::get_total_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let locked_liquidity =
            storage::get_locked_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let available_liquidity = safe_math::sub_i128(total_liquidity, locked_liquidity)?;

        // Calculate withdrawal amount proportionally
        let amount_returned =
            safe_math::div_i128(safe_math::mul_i128(shares, total_liquidity)?, total_shares)?;

        if amount_returned > available_liquidity {
            return Err(LiquidityPoolError::InsufficientLiquidity);
        }

        // Burn shares
        let new_provider_shares = safe_math::sub_i128(provider_shares, shares)?;
        storage::set_lp_shares(&env, &provider, new_provider_shares);

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

    /// Receive a forfeited guarantee on loan default and liquidate funds.
    /// The amount offsets the loss: it is added back to total_liquidity
    /// and reduces locked_liquidity by the same amount (partial recovery).
    pub fn liquidate_funds(
        env: Env,
        creditline: Address,
        _loan_id: u64,
        amount: i128,
        _sponsor: Address,
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

        // We emit guarantee received with the same parameters as before or expanded if needed.
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

    pub fn get_pool_stats(env: Env) -> PoolStats {
        let total_liquidity =
            storage::get_total_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let locked_liquidity =
            storage::get_locked_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        let available_liquidity = total_liquidity.saturating_sub(locked_liquidity);
        let total_shares =
            storage::get_total_shares(&env).unwrap_or_else(|err| panic_with_error!(&env, err));

        // Share price in basis points: (total_liquidity × 10000) / total_shares
        let share_price = if total_shares == 0 {
            types::TOTAL_BPS // Default: 1.00 expressed as 10000 bps
        } else {
            safe_math::div_i128(
                safe_math::mul_i128(total_liquidity, types::TOTAL_BPS).unwrap_or(0),
                total_shares,
            )
            .unwrap_or(types::TOTAL_BPS)
        };

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
        let total_shares =
            storage::get_total_shares(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        if total_shares == 0 {
            return 0;
        }
        let total_liquidity =
            storage::get_total_liquidity(&env).unwrap_or_else(|err| panic_with_error!(&env, err));
        safe_math::div_i128(
            safe_math::mul_i128(shares, total_liquidity).unwrap_or(0),
            total_shares,
        )
        .unwrap_or(0)
    }

    // -------------------------------------------------------------------------
    // Internal helpers
    // -------------------------------------------------------------------------

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
