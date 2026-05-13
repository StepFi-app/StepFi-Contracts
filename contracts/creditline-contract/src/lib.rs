#![no_std]
use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contract, contractimpl, panic_with_error, symbol_short, token, Address, Env, IntoVal, Symbol,
    Val, Vec,
};

mod liquidity_pool {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/liquidity_pool_contract.wasm"
    );
}
mod vendor_registry {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/vendor_registry_contract.wasm"
    );
}
use liquidity_pool::Client as LiquidityPoolContractClient;
use vendor_registry::Client as VendorRegistryContractClient;

mod access;
mod errors;
mod events;
mod storage;
mod types;

pub use errors::CreditLineError;
pub use types::{default_protocol_parameters, Loan, LoanStatus, LoanType, ProtocolParameters, RepaymentInstallment};

#[contract]
pub struct CreditLineContract;

#[contractimpl]
impl CreditLineContract {
    pub fn get_version() -> Symbol {
        symbol_short!("v1_0_0")
    }

    pub fn initialize(
        env: Env,
        admin: Address,
        reputation_contract: Address,
        vendor_registry: Address,
        liquidity_pool: Address,
        token: Address,
    ) {
        let admin_opt: Option<Address> = env.storage().instance().get(&storage::ADMIN_KEY);
        if admin_opt.is_some() {
            panic!("Already initialized");
        }

        admin.require_auth();

        storage::set_admin(&env, &admin);
        storage::set_reputation_contract(&env, &reputation_contract);
        storage::set_vendor_registry(&env, &vendor_registry);
        storage::set_liquidity_pool(&env, &liquidity_pool);
        storage::set_token(&env, &token);
    }

    pub fn create_loan(
        env: Env,
        user: Address,
        vendor: Address,
        total_amount: i128,
        guarantee_amount: i128,
        repayment_schedule: Vec<RepaymentInstallment>,
        loan_type: LoanType,
    ) -> u64 {
        user.require_auth();

        Self::validate_guarantee(&env, total_amount, guarantee_amount);
        Self::validate_vendor(&env, &vendor);
        let score = Self::validate_reputation(&env, &user);
        Self::validate_liquidity(&env, total_amount, guarantee_amount);
        Self::enter_non_reentrant(&env);

        let mut loan = Self::build_loan(
            &env,
            user.clone(),
            vendor.clone(),
            total_amount,
            guarantee_amount,
            repayment_schedule.clone(),
            score,
            LoanStatus::Active,
            loan_type,
        );
        loan.funded_at = env.ledger().timestamp();

        storage::increase_user_active_debt(&env, &user, loan.remaining_balance);
        let loan_id = loan.loan_id;
        storage::write_loan(&env, &loan);

        let pool_contribution = total_amount
            .checked_sub(guarantee_amount)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::Underflow));
        Self::fund_loan_from_pool(&env, &user, &vendor, guarantee_amount, pool_contribution);

        events::emit_loan_created(
            &env,
            &user,
            &vendor,
            loan_id,
            total_amount,
            guarantee_amount,
            &repayment_schedule,
        );

        Self::exit_non_reentrant(&env);
        loan_id
    }

    pub fn request_loan(
        env: Env,
        user: Address,
        vendor: Address,
        total_amount: i128,
        guarantee_amount: i128,
        repayment_schedule: Vec<RepaymentInstallment>,
        loan_type: LoanType,
    ) -> u64 {
        user.require_auth();

        Self::validate_guarantee(&env, total_amount, guarantee_amount);
        let score = Self::validate_reputation(&env, &user);
        let loan = Self::build_loan(
            &env,
            user.clone(),
            vendor.clone(),
            total_amount,
            guarantee_amount,
            repayment_schedule.clone(),
            score,
            LoanStatus::Pending,
            loan_type,
        );

        let token_address = storage::get_token(&env)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::TokenNotConfigured));
        let token_client = token::Client::new(&env, &token_address);
        token_client.transfer(&user, &env.current_contract_address(), &guarantee_amount);

        let loan_id = loan.loan_id;
        storage::write_loan(&env, &loan);

        events::emit_loan_requested(
            &env,
            &user,
            &vendor,
            loan_id,
            total_amount,
            guarantee_amount,
            &repayment_schedule,
        );

        loan_id
    }

    pub fn get_user_loans(env: Env, borrower: Address, start: u64, limit: u32) -> Vec<Loan> {
        storage::get_user_loans_paginated(&env, &borrower, start, limit)
    }

    pub fn get_user_loan_count(env: Env, borrower: Address) -> u64 {
        storage::get_user_loan_count(&env, &borrower)
    }

    pub fn get_user_active_debt(env: Env, borrower: Address) -> i128 {
        storage::get_user_active_debt(&env, &borrower)
    }

    pub fn get_loan(env: Env, loan_id: u64) -> Loan {
        storage::read_loan(&env, loan_id)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::LoanNotFound))
    }

    pub fn set_admin(env: Env, new_admin: Address) {
        let old_admin = storage::get_admin(&env);
        old_admin.require_auth();
        access::require_admin(&env, &old_admin);

        storage::set_admin(&env, &new_admin);
    }


    /// Upgrade the contract WASM — admin only
    pub fn upgrade(env: Env, new_wasm_hash: soroban_sdk::BytesN<32>) {
        let admin = storage::get_admin(&env);
        admin.require_auth();
        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }
    pub fn get_admin(env: Env) -> Address {
        storage::get_admin(&env)
    }

    pub fn set_reputation_contract(env: Env, admin: Address, address: Address) {
        admin.require_auth();
        access::require_admin(&env, &admin);
        storage::set_reputation_contract(&env, &address);
    }

    pub fn set_vendor_registry(env: Env, admin: Address, address: Address) {
        admin.require_auth();
        access::require_admin(&env, &admin);
        storage::set_vendor_registry(&env, &address);
    }

    pub fn set_liquidity_pool(env: Env, admin: Address, address: Address) {
        admin.require_auth();
        access::require_admin(&env, &admin);
        storage::set_liquidity_pool(&env, &address);
    }

    pub fn set_parameters_contract(env: Env, admin: Address, address: Address) {
        admin.require_auth();
        access::require_admin(&env, &admin);
        storage::set_parameters_contract(&env, &address);
    }

    fn validate_guarantee(env: &Env, total_amount: i128, guarantee_amount: i128) {
        if total_amount <= 0 || guarantee_amount <= 0 {
            panic_with_error!(env, CreditLineError::InvalidAmount);
        }

        if guarantee_amount > total_amount {
            panic_with_error!(env, CreditLineError::InvalidAmount);
        }

        let params = Self::get_protocol_parameters(env);
        let min_guarantee = total_amount
            .checked_mul(params.min_guarantee_percent as i128)
            .and_then(|v| v.checked_div(100))
            .unwrap_or_else(|| panic_with_error!(env, CreditLineError::Overflow));

        if guarantee_amount < min_guarantee {
            panic_with_error!(env, CreditLineError::InsufficientGuarantee);
        }
    }

    fn validate_vendor(env: &Env, vendor: &Address) {
        let vendor_registry = storage::get_vendor_registry(env)
            .unwrap_or_else(|| panic_with_error!(env, CreditLineError::InvalidVendor));

        let registry_client = VendorRegistryContractClient::new(env, &vendor_registry);
        let is_active = env
            .try_invoke_contract::<bool, soroban_sdk::Error>(
                &registry_client.address,
                &symbol_short!("is_active"),
                (vendor,).into_val(env),
            )
            .unwrap_or_else(|_| panic_with_error!(env, CreditLineError::VendorValidationFailed))
            .unwrap_or_else(|_| panic_with_error!(env, CreditLineError::VendorValidationFailed));

        if !is_active {
            panic_with_error!(env, CreditLineError::VendorNotActive);
        }
    }

    fn validate_reputation(env: &Env, user: &Address) -> u32 {
        let reputation_contract = storage::get_reputation_contract(env)
            .unwrap_or_else(|| panic!("Reputation contract not configured"));

        let score: u32 = env.invoke_contract(
            &reputation_contract,
            &symbol_short!("get_score"),
            (user,).into_val(env),
        );

        let params = Self::get_protocol_parameters(env);
        if score < params.min_reputation_threshold {
            panic_with_error!(env, CreditLineError::InsufficientReputation);
        }

        score
    }

    fn validate_liquidity(env: &Env, total_amount: i128, guarantee_amount: i128) {
        let liquidity_pool = storage::get_liquidity_pool(env)
            .unwrap_or_else(|| panic_with_error!(env, CreditLineError::InsufficientLiquidity));

        let required_from_pool = total_amount
            .checked_sub(guarantee_amount)
            .unwrap_or_else(|| panic_with_error!(env, CreditLineError::Underflow));

        if required_from_pool == 0 {
            return;
        }

        let lp_client = LiquidityPoolContractClient::new(env, &liquidity_pool);
        let stats = lp_client.get_pool_stats();

        if stats.available_liquidity < required_from_pool {
            panic_with_error!(env, CreditLineError::InsufficientLiquidity);
        }
    }

    fn fund_loan_from_pool(
        env: &Env,
        borrower: &Address,
        vendor: &Address,
        guarantee_amount: i128,
        pool_contribution: i128,
    ) {
        let liquidity_pool = storage::get_liquidity_pool(env)
            .unwrap_or_else(|| panic_with_error!(env, CreditLineError::InsufficientLiquidity));

        let token_address = storage::get_token(env)
            .unwrap_or_else(|| panic_with_error!(env, CreditLineError::TokenNotConfigured));

        let token_client = token::Client::new(env, &token_address);
        token_client.transfer(borrower, &env.current_contract_address(), &guarantee_amount);

        if pool_contribution > 0 {
            let lp_client = LiquidityPoolContractClient::new(env, &liquidity_pool);
            lp_client.fund_loan(
                &env.current_contract_address(),
                vendor,
                &pool_contribution,
            );
        }
    }

    fn build_loan(
        env: &Env,
        user: Address,
        vendor: Address,
        total_amount: i128,
        guarantee_amount: i128,
        repayment_schedule: Vec<RepaymentInstallment>,
        score: u32,
        status: LoanStatus,
        loan_type: LoanType,
    ) -> Loan {
        Self::validate_guarantee(env, total_amount, guarantee_amount);
        Self::validate_vendor(env, &vendor);

        let interest_rate_bps = Self::interest_rate_bps(env, score);
        let interest_amount =
            Self::calculate_bps_amount(env, total_amount, interest_rate_bps as i128);
        let service_fee_amount =
            Self::calculate_bps_amount(env, total_amount, types::SERVICE_FEE_BPS);
        let remaining_balance = total_amount
            .checked_add(interest_amount)
            .and_then(|v| v.checked_add(service_fee_amount))
            .unwrap_or_else(|| panic_with_error!(env, CreditLineError::Overflow));

        let credit_limit = Self::credit_limit(score);
        let active_debt = storage::get_user_active_debt(env, &user);
        let next_debt = active_debt
            .checked_add(remaining_balance)
            .unwrap_or_else(|| panic_with_error!(env, CreditLineError::Overflow));
        if next_debt > credit_limit {
            panic_with_error!(env, CreditLineError::ExposureLimitExceeded);
        }

        let loan_id = storage::increment_loan_counter(env);
        Loan {
            loan_id,
            borrower: user,
            vendor,
            total_amount,
            guarantee_amount,
            interest_rate_bps,
            interest_amount,
            service_fee_amount,
            principal_outstanding: total_amount,
            interest_outstanding: interest_amount,
            service_fee_outstanding: service_fee_amount,
            remaining_balance,
            repayment_schedule,
            status,
            loan_type,
            created_at: env.ledger().timestamp(),
            funded_at: 0,
            late_fees_outstanding: 0,
            late_fee_accrual_timestamp: 0,
        }
    }

    fn calculate_bps_amount(env: &Env, base: i128, bps: i128) -> i128 {
        base.checked_mul(bps)
            .and_then(|v| v.checked_div(types::BPS_DENOMINATOR))
            .unwrap_or_else(|| panic_with_error!(env, CreditLineError::Overflow))
    }

    fn interest_rate_bps(env: &Env, score: u32) -> u32 {
        let base_interest_bps = Self::get_protocol_parameters(env).base_interest_bps;
        if base_interest_bps == 0 {
            return match score {
                90..=u32::MAX => 400,
                75..=89 => 600,
                60..=74 => 800,
                _ => 1_000,
            };
        }

        match score {
            90..=u32::MAX => base_interest_bps.saturating_sub(600),
            75..=89 => base_interest_bps.saturating_sub(400),
            60..=74 => base_interest_bps.saturating_sub(200),
            _ => base_interest_bps,
        }
    }

    fn credit_limit(score: u32) -> i128 {
        match score {
            90..=u32::MAX => 10_000,
            75..=89 => 5_000,
            60..=74 => 2_500,
            _ => 1_000,
        }
    }

    fn calculate_default_penalty(env: &Env, loan: &Loan) -> u32 {
        let params = Self::get_protocol_parameters(env);
        if loan.total_amount > params.large_loan_threshold {
            params.large_loan_default_penalty
        } else {
            params.default_penalty
        }
    }

    /// Warn that a loan is past due but still within the grace period.
    /// Emits a `LOANGRC` event so off-chain services and borrowers can be notified.
    /// Returns `LoanNotOverdue` if the loan is not yet past its due date, and
    /// `LoanNotActive` if the loan is not active.  Returns `Ok(())` when the
    /// warning event was successfully emitted (i.e. the loan is in the grace window).
    pub fn warn_grace_period(env: Env, loan_id: u64) -> Result<(), CreditLineError> {
        let loan = storage::read_loan(&env, loan_id).ok_or(CreditLineError::LoanNotFound)?;

        if loan.status != LoanStatus::Active {
            return Err(CreditLineError::LoanNotActive);
        }

        let last_installment = loan
            .repayment_schedule
            .last()
            .ok_or(CreditLineError::Overflow)?;

        let now = env.ledger().timestamp();
        if now <= last_installment.due_date {
            return Err(CreditLineError::LoanNotOverdue);
        }

        let params = Self::get_protocol_parameters(&env);
        let grace_ends_at = last_installment
            .due_date
            .checked_add(params.grace_period_seconds)
            .ok_or(CreditLineError::Overflow)?;

        if now > grace_ends_at {
            // Grace period already expired — not in grace period anymore.
            return Err(CreditLineError::LoanNotOverdue);
        }

        events::emit_loan_in_grace_period(
            &env,
            &loan.borrower,
            loan_id,
            loan.remaining_balance,
            grace_ends_at,
        );

        Ok(())
    }

    pub fn mark_defaulted(env: Env, loan_id: u64) -> Result<(), CreditLineError> {
        let mut loan = storage::read_loan(&env, loan_id).ok_or(CreditLineError::LoanNotFound)?;

        if loan.status != LoanStatus::Active {
            return Err(CreditLineError::LoanNotActive);
        }

        let last_installment = loan
            .repayment_schedule
            .last()
            .ok_or(CreditLineError::Overflow)?;

        let now = env.ledger().timestamp();
        if now <= last_installment.due_date {
            return Err(CreditLineError::LoanNotOverdue);
        }

        let params = Self::get_protocol_parameters(&env);
        let grace_ends_at = last_installment
            .due_date
            .checked_add(params.grace_period_seconds)
            .ok_or(CreditLineError::Overflow)?;

        if now <= grace_ends_at {
            // Still within the grace window — emit a warning and block hard default.
            events::emit_loan_in_grace_period(
                &env,
                &loan.borrower,
                loan_id,
                loan.remaining_balance,
                grace_ends_at,
            );
            return Err(CreditLineError::LoanInGracePeriod);
        }

        let lp_address =
            storage::get_liquidity_pool(&env).ok_or(CreditLineError::InsufficientLiquidity)?;

        Self::enter_non_reentrant(&env);

        loan.status = LoanStatus::Defaulted;
        storage::decrease_user_active_debt(&env, &loan.borrower, loan.remaining_balance);
        storage::write_loan(&env, &loan);

        let token_address = storage::get_token(&env).ok_or(CreditLineError::TokenNotConfigured)?;
        Self::authorize_token_transfer(&env, &token_address, &lp_address, loan.guarantee_amount);

        let lp_client = LiquidityPoolContractClient::new(&env, &lp_address);
        lp_client.receive_guarantee(&env.current_contract_address(), &loan.guarantee_amount);

        events::emit_loan_defaulted(
            &env,
            loan.borrower.clone(),
            loan_id,
            loan.total_amount,
            loan.remaining_balance,
            loan.guarantee_amount,
        );

        if let Some(reputation_contract) = storage::get_reputation_contract(&env) {
            let penalty = Self::calculate_default_penalty(&env, &loan);
            let updater = env.current_contract_address();
            let _ = env.try_invoke_contract::<(), soroban_sdk::Error>(
                &reputation_contract,
                &Symbol::new(&env, "decrease_score"),
                (updater, loan.borrower, penalty).into_val(&env),
            );
        }

        Self::exit_non_reentrant(&env);
        Ok(())
    }

    pub fn approve_loan(env: Env, loan_id: u64) -> Loan {
        // 1. Admin auth - must be first
        let admin = storage::get_admin(&env);
        admin.require_auth();

        // 2. Load loan — panic if not found
        let mut loan = storage::read_loan(&env, loan_id)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::LoanNotFound));

        // 3. Validate status is Pending
        if loan.status != LoanStatus::Pending {
            panic_with_error!(&env, CreditLineError::InvalidLoanStatus);
        }

        // 4. Transition to Active
        loan.status = LoanStatus::Active;

        // 5. Write back with TTL extension
        storage::write_loan(&env, &loan);

        // 6. Emit event
        events::emit_loan_approved(&env, loan_id);

        loan
    }

    pub fn cancel_loan(env: Env, caller: Address, loan_id: u64) {
        caller.require_auth();

        let mut loan = storage::read_loan(&env, loan_id)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::LoanNotFound));

        if loan.status != LoanStatus::Pending {
            panic_with_error!(&env, CreditLineError::LoanNotCancellable);
        }

        let admin = storage::get_admin(&env);
        if caller != loan.borrower && caller != admin {
            panic_with_error!(&env, CreditLineError::UnauthorizedRepayer);
        }

        let token_address = storage::get_token(&env)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::TokenNotConfigured));
        let token_client = token::Client::new(&env, &token_address);
        token_client.transfer(
            &env.current_contract_address(),
            &loan.borrower,
            &loan.guarantee_amount,
        );

        loan.status = LoanStatus::Cancelled;
        storage::write_loan(&env, &loan);
        events::emit_loan_cancelled(&env, &loan.borrower, loan_id, loan.guarantee_amount);
    }

    pub fn repay_loan(env: Env, borrower: Address, loan_id: u64, amount: i128) -> i128 {
        borrower.require_auth();

        let mut loan = storage::read_loan(&env, loan_id)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::LoanNotFound));

        if loan.borrower != borrower {
            panic_with_error!(&env, CreditLineError::UnauthorizedRepayer);
        }

        if loan.status != LoanStatus::Active {
            panic_with_error!(&env, CreditLineError::LoanNotActive);
        }

        // Accrue any outstanding late fees before validating the payment amount so
        // the borrower repays the true current balance (principal + interest + fees + late fees).
        let accrued_fee = Self::accrue_late_fees_internal(&env, &mut loan);
        if accrued_fee > 0 {
            storage::increase_user_active_debt(&env, &borrower, accrued_fee);
            events::emit_late_fee_accrued(&env, &borrower, loan_id, accrued_fee, loan.remaining_balance);
        }

        if amount <= 0 || amount > loan.remaining_balance {
            panic_with_error!(&env, CreditLineError::InvalidRepaymentAmount);
        }

        Self::enter_non_reentrant(&env);

        // Payment priority: principal → interest → service fee → late fees
        let principal_paid = amount.min(loan.principal_outstanding);
        let after_principal = amount
            .checked_sub(principal_paid)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::Underflow));
        let interest_paid = after_principal.min(loan.interest_outstanding);
        let after_interest = after_principal
            .checked_sub(interest_paid)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::Underflow));
        let fee_paid = after_interest.min(loan.service_fee_outstanding);
        let after_fee = after_interest
            .checked_sub(fee_paid)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::Underflow));
        let late_fee_paid = after_fee.min(loan.late_fees_outstanding);

        loan.principal_outstanding = loan
            .principal_outstanding
            .checked_sub(principal_paid)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::Underflow));
        loan.interest_outstanding = loan
            .interest_outstanding
            .checked_sub(interest_paid)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::Underflow));
        loan.service_fee_outstanding = loan
            .service_fee_outstanding
            .checked_sub(fee_paid)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::Underflow));
        loan.late_fees_outstanding = loan
            .late_fees_outstanding
            .checked_sub(late_fee_paid)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::Underflow));

        let new_balance = loan
            .remaining_balance
            .checked_sub(amount)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::Underflow));

        loan.remaining_balance = new_balance;
        let is_fully_repaid = new_balance == 0;
        if is_fully_repaid {
            loan.status = LoanStatus::Paid;
        }

        storage::decrease_user_active_debt(&env, &borrower, amount);
        storage::write_loan(&env, &loan);

        let lp_address = storage::get_liquidity_pool(&env)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::InsufficientLiquidity));
        let token_address = storage::get_token(&env)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::TokenNotConfigured));

        let token_client = token::Client::new(&env, &token_address);
        token_client.transfer(&borrower, &env.current_contract_address(), &amount);
        Self::authorize_token_transfer(&env, &token_address, &lp_address, amount);

        let lp_client = LiquidityPoolContractClient::new(&env, &lp_address);
        lp_client.receive_repayment(
            &env.current_contract_address(),
            &principal_paid,
            &interest_paid
                .checked_add(fee_paid)
                .and_then(|v| v.checked_add(late_fee_paid))
                .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::Overflow)),
        );

        if is_fully_repaid {
            token_client.transfer(
                &env.current_contract_address(),
                &borrower,
                &loan.guarantee_amount,
            );
        }

        events::emit_loan_repaid(
            &env,
            &borrower,
            loan_id,
            amount,
            new_balance,
            is_fully_repaid,
        );

        if is_fully_repaid {
            if let Some(reputation_contract) = storage::get_reputation_contract(&env) {
                let updater = env.current_contract_address();
                let payment_date = env.ledger().timestamp();
                let due_date = loan
                    .repayment_schedule
                    .last()
                    .map(|i| i.due_date)
                    .unwrap_or(0);
                Self::handle_reputation_increase(
                    &env,
                    &reputation_contract,
                    &updater,
                    &borrower,
                    payment_date,
                    due_date,
                );
            }
        }

        Self::exit_non_reentrant(&env);
        new_balance
    }

    /// Mark a single installment paid and reduce the loan's outstanding balance by `amount`.
    ///
    /// Borrower-only. Validates the installment index, ensures the slot is unpaid, debits the
    /// remaining balance, sets `paid`/`paid_at`, persists the loan, and emits `INSTPAID`.
    pub fn repay_installment(
        env: Env,
        borrower: Address,
        loan_id: u64,
        installment_index: u32,
        amount: i128,
    ) -> i128 {
        borrower.require_auth();

        let mut loan = storage::read_loan(&env, loan_id)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::LoanNotFound));

        if loan.borrower != borrower {
            panic_with_error!(&env, CreditLineError::UnauthorizedRepayer);
        }

        if loan.status != LoanStatus::Active {
            panic_with_error!(&env, CreditLineError::LoanNotActive);
        }

        if installment_index >= loan.repayment_schedule.len() {
            panic_with_error!(&env, CreditLineError::InvalidInstallmentIndex);
        }

        let mut installment = loan
            .repayment_schedule
            .get(installment_index)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::InvalidInstallmentIndex));

        if installment.paid {
            panic_with_error!(&env, CreditLineError::InstallmentAlreadyPaid);
        }

        if amount <= 0 || amount > loan.remaining_balance {
            panic_with_error!(&env, CreditLineError::InvalidRepaymentAmount);
        }

        Self::enter_non_reentrant(&env);

        let new_balance = loan
            .remaining_balance
            .checked_sub(amount)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::Underflow));
        loan.remaining_balance = new_balance;

        installment.paid = true;
        installment.paid_at = env.ledger().timestamp();
        loan.repayment_schedule.set(installment_index, installment);

        if new_balance == 0 {
            loan.status = LoanStatus::Paid;
        }

        storage::decrease_user_active_debt(&env, &borrower, amount);
        storage::write_loan(&env, &loan);

        events::emit_installment_paid(&env, loan_id, installment_index, amount, new_balance);

        Self::exit_non_reentrant(&env);
        new_balance
    }

    /// Accrue late fees for a loan and update the caller-supplied `loan` in place.
    ///
    /// Fees are calculated as `remaining_balance × LATE_FEE_BPS_PER_DAY × days_overdue`
    /// starting from the earliest overdue installment due date (or the previous accrual
    /// timestamp, whichever is later). Only complete days are counted; any partial day
    /// carries over to the next accrual.
    ///
    /// Returns the newly accrued fee amount (0 if nothing was due).
    fn accrue_late_fees_internal(env: &Env, loan: &mut Loan) -> i128 {
        let now = env.ledger().timestamp();

        // Find the earliest overdue installment due date.
        let mut overdue_since: Option<u64> = None;
        for installment in loan.repayment_schedule.iter() {
            if installment.due_date < now {
                overdue_since = Some(match overdue_since {
                    None => installment.due_date,
                    Some(d) => {
                        if installment.due_date < d {
                            installment.due_date
                        } else {
                            d
                        }
                    }
                });
            }
        }

        let overdue_since = match overdue_since {
            Some(d) => d,
            None => return 0, // no overdue installments
        };

        // Accrue from the later of (first overdue date, last accrual timestamp).
        let accrual_start = if loan.late_fee_accrual_timestamp == 0 {
            overdue_since
        } else if loan.late_fee_accrual_timestamp > overdue_since {
            loan.late_fee_accrual_timestamp
        } else {
            overdue_since
        };

        if now <= accrual_start {
            return 0;
        }

        let seconds_elapsed = now - accrual_start;
        let days_elapsed = (seconds_elapsed / types::SECONDS_PER_DAY) as i128;

        if days_elapsed == 0 {
            return 0; // less than one full day has passed since last accrual
        }

        let fee = loan
            .remaining_balance
            .checked_mul(types::LATE_FEE_BPS_PER_DAY)
            .and_then(|v| v.checked_mul(days_elapsed))
            .and_then(|v| v.checked_div(types::BPS_DENOMINATOR))
            .unwrap_or(0);

        if fee == 0 {
            return 0;
        }

        // Advance the accrual cursor by only complete days to avoid losing fractions.
        loan.late_fee_accrual_timestamp =
            accrual_start + (days_elapsed as u64) * types::SECONDS_PER_DAY;

        loan.late_fees_outstanding = loan
            .late_fees_outstanding
            .checked_add(fee)
            .unwrap_or(loan.late_fees_outstanding);
        loan.remaining_balance = loan
            .remaining_balance
            .checked_add(fee)
            .unwrap_or(loan.remaining_balance);

        fee
    }

    /// Apply late fees to an active loan without requiring a repayment.
    ///
    /// Anyone may call this to trigger fee accrual on an overdue loan.  Emits a
    /// `LOANLTFE` event when fees are accrued; is a no-op when no full day has
    /// elapsed since the last accrual or when no installment is overdue.
    pub fn apply_late_fees(env: Env, loan_id: u64) {
        let mut loan = storage::read_loan(&env, loan_id)
            .unwrap_or_else(|| panic_with_error!(&env, CreditLineError::LoanNotFound));

        if loan.status != LoanStatus::Active {
            panic_with_error!(&env, CreditLineError::LoanNotActive);
        }

        let accrued_fee = Self::accrue_late_fees_internal(&env, &mut loan);

        if accrued_fee == 0 {
            return;
        }

        storage::increase_user_active_debt(&env, &loan.borrower, accrued_fee);
        storage::write_loan(&env, &loan);
        events::emit_late_fee_accrued(&env, &loan.borrower, loan_id, accrued_fee, loan.remaining_balance);
    }

    fn handle_reputation_increase(
        env: &Env,
        reputation_contract: &Address,
        updater: &Address,
        borrower: &Address,
        payment_date: u64,
        due_date: u64,
    ) {
        let score_increase: u32 = if payment_date < due_date { 15 } else { 10 };
        let _ = env.try_invoke_contract::<(), soroban_sdk::Error>(
            reputation_contract,
            &Symbol::new(env, "increase_score"),
            (updater, borrower, score_increase).into_val(env),
        );
    }

    fn get_protocol_parameters(env: &Env) -> ProtocolParameters {
        match storage::get_parameters_contract(env) {
            Some(address) => env
                .try_invoke_contract::<ProtocolParameters, soroban_sdk::Error>(
                    &address,
                    &Symbol::new(env, "get_parameters"),
                    ().into_val(env),
                )
                .unwrap_or_else(|_| panic_with_error!(env, CreditLineError::ParametersUnavailable))
                .unwrap_or_else(|_| panic_with_error!(env, CreditLineError::ParametersUnavailable)),
            None => default_protocol_parameters(),
        }
    }

    fn enter_non_reentrant(env: &Env) {
        if storage::is_reentrancy_locked(env) {
            panic_with_error!(env, CreditLineError::ReentrancyDetected);
        }
        storage::set_reentrancy_locked(env, true);
    }

    fn exit_non_reentrant(env: &Env) {
        storage::set_reentrancy_locked(env, false);
    }

    fn authorize_token_transfer(env: &Env, token_address: &Address, to: &Address, amount: i128) {
        let args: Vec<Val> = (env.current_contract_address(), to.clone(), amount).into_val(env);
        let context = ContractContext {
            contract: token_address.clone(),
            fn_name: Symbol::new(env, "transfer"),
            args,
        };
        let invocation = SubContractInvocation {
            context,
            sub_invocations: Vec::new(env),
        };
        let mut auth_entries = Vec::new(env);
        auth_entries.push_back(InvokerContractAuthEntry::Contract(invocation));
        env.authorize_as_current_contract(auth_entries);
    }
}

#[cfg(test)]
mod tests;
