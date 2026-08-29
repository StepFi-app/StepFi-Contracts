# Progress Tracker — StepFi-Contracts

Update this file after every completed contract change, fix, or architectural decision. Progress state must reflect the actual deployed and tested state — not the intended state.

---

## Current Phase

**Phase 1 — Contract Infrastructure & Core Fixes**

## Current Goal

`LoanType` and per-installment tracking are in. Next: per-loan grace period (Next Up #4), then vouching contract.

---

## Completed

### Security: Close Unauthenticated Admin-Claim Branch in Reputation `set_admin()`
- **Problem:** `set_admin()` in `contracts/reputation-contract/src/lib.rs` contained an unauthenticated fallback branch when no admin was stored, allowing anyone to claim admin of the reputation contract without authorization.
- **Fix (`reputation-contract`):**
  - Added explicit, one-time `initialize(env, admin) -> Result<(), ReputationError>` function that requires `admin.require_auth()` and checks `storage::has_admin(&env)` (rejects re-initialization with `AlreadyInitialized = 11`).
  - Updated `set_admin(env, new_admin) -> Result<(), ReputationError>` to fetch `old_admin = storage::get_admin(&env)?` (panics/returns `NotInitialized` when uninitialized) and enforce `old_admin.require_auth()` + `access::require_admin(&env, &old_admin)`.
  - Added `has_admin(env)` in `storage.rs`.
  - Added `AlreadyInitialized` error variant in `errors.rs`.
  - Added new unit tests covering: unauthenticated first-time `set_admin` rejection (`NotInitialized`), single authorized `initialize`, double initialization rejection (`AlreadyInitialized`), unauthenticated `initialize` rejection, and end-to-end updater flow preservation.
  - Updated test setup in `reputation-contract/src/tests.rs` and `creditline-contract/src/tests.rs` (`RealIntegrationCtx`), deployment script (`scripts/deploy-testnet.sh`), deployment metadata (`contracts/deployed-testnet.json`), and contract documentation (`contracts/reputation-contract/README.md`).

### `approve_loan` Pending Loan Funding & Re-Validation Fix
- **Problem:** `approve_loan()` previously activated pending loans without validating vendor status, checking reputation score, or checking available pool liquidity, and without calling `fund_loan()` on the liquidity pool to lock contribution funds or transferring funds to the vendor.
- **Fix (`creditline-contract`):**
  - Refactored shared pool funding logic into internal helper `fund_loan_from_pool(&env, borrower, vendor, guarantee_amount, pool_contribution, pull_guarantee)` used by both `create_loan` (`pull_guarantee = true`) and `approve_loan` (`pull_guarantee = false`).
  - Extended `approve_loan()` to re-validate vendor status (`validate_vendor`), borrower reputation (`validate_reputation`), and pool liquidity (`validate_liquidity`) at approval time before mutating state.
  - Extended `approve_loan()` to set `funded_at`, increment `user_active_debt`, lock pool funds via `fund_loan_from_pool`, write persistent loan record with TTL extension, and emit both `LOANAPPROVED` and `LOANFNDD` (`LoanFunded`) events.
  - Added new unit tests covering: vendor funding and liquidity locking on approval, insufficient liquidity rejection (`InsufficientLiquidity`), suspended vendor approval rejection (`VendorNotActive`), decayed reputation approval rejection (`InsufficientReputation`), and double-funding prevention (`InvalidLoanStatus`).

### Issue #82 — First-Depositor Share-Price Inflation Attack Mitigation (revised after audit)
- **Dead shares with virtual backing**: On first deposit (`total_shares == 0`), mints `DEAD_SHARES_AMOUNT = 1_000` dead shares to `env.current_contract_address()` (unclaimable — `withdraw` requires `provider.require_auth()`). Dead shares are **backed by virtual liquidity**: `calculate_share_price_internal()` computes `(total_liquidity + DEAD_SHARES_AMOUNT) * PRECISION / total_shares` (floor `1`). This preserves honest 1:1 `deposit 1_000 → shares 1_000 → withdraw 1_000` while still preventing a dust depositor from owning 100% of yield (audit gap 1 fixed).
- **First-deposit share price**: No hardcoded branch. With virtual backing `(0+1_000)*10000/1_000 = 10_000`, the generic formula yields `PRECISION` for the first honest depositor, so share-price semantics are unchanged on the honest path.
- **`MIN_AMOUNT = 1_000`**: Deposits below `1_000` fail with `InvalidAmount` (#4). Enforced on all deposits; secondary dust defense. Honest `1_000` minimum depositor retains full principal (no 50% tax).
- **`calculate_share_price_internal` fix**: When `total_shares > 0` but `total_liquidity == 0` (fully-absorbed default), returns **near-zero proportional price** `(0+1_000)*10000/total_shares` (e.g. `909` for `10_000` shares + dead) floored to `1`, **not** hardcoded `0` (which bricked the next `deposit` via divide-by-zero) and **not** `PRECISION` (which hid losses). Coordinated with `absorb_loss()` caps (audit gap 3 fixed).
- **Withdraw guard**: `shares <= 0` rejected; floor-division in both `deposit` (`shares = amount*PRECISION/price` floored) and `withdraw` (`amount = shares*price/PRECISION` floored) via `safe_math`, never rounding up.
- **New constants**: `DEAD_SHARES_AMOUNT = 1_000`, `MIN_AMOUNT = 1_000`, `SHARE_PRICE_PRECISION = 10_000` in `types.rs`.
- **Tests — 107 pass**: Updated existing tests for virtual-backed math; `5` security tests corrected to `1:1`/`9_454`; plus **2 new regression**: `test_honest_path_regression_one_to_one` (proves `1_000→1_000` parity and second depositor `1:1`) and `test_post_default_deposit_does_not_brick` (full `absorb_loss` → price `909` → next `deposit 1_000` succeeds with `>1_000` shares, no divide-by-zero). Creditline integration `test_mark_defaulted_loss_absorption_share_price_impact` updated to `9_454`. Total `cargo test` 107 (liquidity-pool) + 128 (creditline) + others = 362 passing.
- **Honest-path reconciliation (audit gap)**: Strict `identical results` for *interest-bearing* yield is intentionally not preserved — dead shares (1_000) take a pro-rata `DEAD/(total+DEAD)` share of distributed interest as the cost of preventing the inflation attack. For the minimal `1_000` pool with `85` interest, honest withdraw `1085→1042` (−4.0%) and deposit-after-interest `921→959` (+4.1% shares for same 1_000) reflect this dilution. **Principal is strictly preserved** (`1_000→1_000` 1:1, second depositor `1:1` at same price, full drain leaves `0` real liquidity). For larger pools dilution is negligible (`10_000` pool, `85` interest → `1085` vs `1077` ≈0.7%). This bounded, documented trade-off is accepted: without dead shares an attacker could steal 100% of yield via dust-deposit + donation inflation. The alternative of smaller `DEAD` (e.g. 100) would reduce dilution to <1% but weaken dust protection; `1_000` equals `MIN_AMOUNT` for simplicity and is documented as deliberate.

### Timelocked Contract Upgrades & Version Overflow Safety
- Routed WASM upgrades through a mandatory two-step timelock (`propose_upgrade` → delay → `execute_upgrade`) across `liquidity-pool-contract`, `creditline-contract`, `reputation-contract`, and `vendor-registry-contract`.
- Parameterized upgrade delay via `ProtocolParameters` in `parameters-contract` (field `upgrade_delay_seconds: u64`, defaulting to 86,400 seconds / 1 day).
- Enforced exact committed `wasm_hash` matching and unlock timestamp verification prior to WASM update.
- Replaced saturating `unwrap_or(old_version)` version increment with `checked_add(1).ok_or(Overflow)`.
- Emitted `UPGDPRP` (upgrade proposed) and `CONTRACTUPGRADED` events at both steps.
- Direct `upgrade()` function restricted to call `execute_upgrade()`, enforcing timelocked commit requirements on existing callers.
- Added comprehensive unit tests in each contract asserting unproposed upgrade fails (`UpgradeNotProposed`), early execution fails (`UpgradeTimelockNotMet`), hash mismatch fails (`UpgradeHashMismatch`), and elapsed execution succeeds and bumps version monotonically.

### Issue #58 — Principal-Interest-Fee Repayment Waterfall
- Added `RepaymentAllocation` struct and `apply_waterfall()` helper in `lib.rs` with correct priority: late fees → interest → service fee → principal
- Fixed `repay_loan()` to use the corrected waterfall order (was principal-first, now late-fees-first)
- Rewrote `repay_installment()` to: accrue late fees, apply waterfall, transfer tokens, call pool's `receive_repayment()`, return guarantee on full repayment, update reputation
- Each `*_outstanding` bucket decremented correctly per payment
- `remaining_balance == sum(all outstanding buckets)` invariant asserted in tests
- Added 8 new tests: waterfall order verification, bucket invariant for both repay_loan and repay_installment, partial/full bucket decrementation, full repayment via repay_installment, active debt tracking
- Updated `test_repay_loan_auto_accrues_late_fees` for new waterfall behavior (late fees paid first, not last)

### Issue #7 — Vendor Approval Flow
- Added `VendorStatus` enum (`Pending`, `Approved`, `Suspended`, `Rejected`) to `types.rs`
- Replaced `active: bool` with `status: VendorStatus` in `VendorInfo`
- `register_vendor()` now sets `status = VendorStatus::Pending` instead of immediately active
- Added `approve_vendor()` (admin-only, requires Pending → Approved)
- Added `suspend_vendor()` (admin-only, any status → Suspended)
- `is_active()` returns `true` only for `Approved` vendors — automatically prevents unapproved/suspended vendors from receiving loans in `creditline-contract`
- Legacy functions (`activate_vendor`, `deactivate_vendor`, `set_vendor_status`) updated to map to new enum
- New error: `VendorNotPending = 10` in `vendor-registry-contract`
- Updated `publish_vendor_status` event to emit `VendorStatus` instead of `bool`
- All vendor-registry tests updated; 7 new tests added (approval flow, non-pending rejection, suspension, re-approval, reentrancy guards for approve/suspend)
- Creditline tests updated to approve vendors after registration
- No changes needed to `creditline-contract/src/lib.rs` — `validate_vendor()` already uses `is_active()` which now checks for `Approved`

### Workspace Cleanup
- Removed dead code: `lp-contract` (superseded by `liquidity-pool-contract`)
- Removed empty placeholder: `adapter-trustless-contract`
- Updated `Cargo.toml` workspace members to reflect 5 active contracts
- Removed `[profile]` sections from individual contract `Cargo.toml` files (profiles belong in workspace root only)

### Renaming
- Renamed `merchant-registry-contract` → `vendor-registry-contract`
- Updated all Rust source references: `merchant_registry_contract` → `vendor_registry_contract`
- Updated all struct names: `MerchantRegistry*` → `VendorRegistry*`
- Updated `Cargo.toml` dependency paths in `creditline-contract`

### Critical Fixes
- Added TTL constants (`PERSISTENT_TTL_THRESHOLD`, `PERSISTENT_TTL_EXTEND_TO`) to `creditline-contract/src/storage.rs`
- Added `upgrade()` function to all 5 contracts: reputation, creditline, liquidity-pool, vendor-registry, parameters
- All 5 contracts build cleanly: `cargo build` passes with zero errors (3 minor unused constant warnings — acceptable)
 - Added numeric `VERSION` instance key, `get_version()` API, and `CONTRACTUPGRADED` event across contracts; added unit tests asserting admin gating and version bump on upgrade

### Deployment
- Created `scripts/deploy-testnet.sh` — full deployment script covering all 5 contracts in correct dependency order
- Script outputs contract IDs and saves to `.env.contracts`

### Documentation
- `README.md` fully rewritten as StepFi-Contracts 

### LoanType + Per-Installment Tracking (creditline-contract)
- Added `LoanType` enum (`Standard`, `LearnerInstallment`) to `types.rs`
- Added `paid: bool` and `paid_at: u64` to `RepaymentInstallment`
- Added `loan_type: LoanType` to `Loan`
- Threaded `loan_type` through `create_loan`, `request_loan`, `build_loan`
- New `repay_installment(borrower, loan_id, installment_index, amount) -> i128`: bounds-checks index, rejects already-paid slots, decrements `remaining_balance`, marks `paid`/`paid_at`, persists, emits `INSTPAID`
- New errors: `InvalidInstallmentIndex = 23`, `InstallmentAlreadyPaid = 24`
- New event: `INSTPAID` via `emit_installment_paid`
- All 93 existing tests updated and passing; 0 failing

### repay_installment Unit Tests
- Added `setup_loan_with_schedule` helper that creates a loan with N equal installments
- `test_repay_installment_happy_path`: pays installment 0, verifies `paid`/`paid_at`, balance decremented, second installment untouched
- `test_repay_installment_double_pay_rejected`: asserts `InstallmentAlreadyPaid` (#24) on second payment of same slot
- `test_repay_installment_out_of_bounds`: asserts `InvalidInstallmentIndex` (#23) for index >= schedule length
- `test_repay_installment_non_borrower_rejected`: asserts `UnauthorizedRepayer` (#14) when caller is not the borrower
- `test_repay_installment_zero_amount_rejected`: asserts `InvalidRepaymentAmount` (#13) for zero payment
- Total tests: 98 (93 existing + 5 new) — all passing

### Issue #6 — Typed Storage Errors
- Removed all `.expect(...)` and bare `.unwrap()` matches from `contracts/*/src/storage.rs`
- Converted storage getters/readers to typed `Result<T, ContractError>` paths while preserving intentional zero/false/default semantics
- Added TTL extension after persistent writes for creditline user indexes/active debt, liquidity-pool LP shares, and vendor-registry vendor/count records
- Added missing `NotInitialized` variants to creditline, parameters, and reputation errors without renumbering existing variants
- Added before-initialize regression coverage across all 5 active contracts using generated `try_*` clients
- Verified with `cargo check --offline`, `cargo build --offline`, `cargo test --offline`, and `cargo clippy --offline -- -D warnings` — 230 passed, 0 failed, 4 ignored

### Issue #4 — Mentor Vouching Contract
- Added `vouching-contract` workspace member with `vouch`, `revoke_vouch`, `get_vouches`, `set_mentor`, and initialization APIs
- Stored verified mentors and mentor/learner vouch records in persistent storage with TTL extension after every persistent write
- Added learner-to-mentor indexing so `get_vouches(learner)` avoids global scans
- Added `MENTORVOUCHED`, `VOUCHREVOKED`, and `MENTORVERIFIED` event helpers using short Soroban event symbols
- Added reputation `add_boost` and `remove_boost` updater-gated APIs for vouching cross-contract calls
- Added mock reputation cross-contract tests covering mentor verification, vouching, revocation, duplicate rejection, unverified mentor rejection, admin rejection, and event emission
- Added `get_version()` and `upgrade()` functions following the same pattern as all other contracts
- Added `CONTRACTUPGRADED` event emission on upgrade
- Added version and upgrade unit tests
- Removed unused `safe_math` functions (replaced with comment placeholder for future use)

### Issue #87 — Vouch expiry & boost-accounting clamp
- **Problem:** Vouches never expired on-chain (no `expire_vouch`, no enforcement), so a mentor's boost inflated a learner's reputation permanently. Secondary: `revoke_vouch()` subtracted the historical `boost_amount` while `vouch()` re-minted with the *current* config boost, and `remove_boost` could push a learner's score below their pre-vouch baseline when combined with penalties or a mid-life boost-config change.
- **Fix (vouching-contract, not yet deployed — `PENDING_DEPLOYMENT`, so the `VouchRecord` layout change is safe):**
  - Added `VOUCH_DURATION: u64 = 2_592_000` (30 days) in `types.rs`.
  - Added permissionless `expire_vouch(mentor, learner)` (no `require_auth`, mirrors creditline's `apply_late_fees`): validates `record.ts + VOUCH_DURATION < now` (else `VouchNotExpired = 13`), deactivates the record, and removes the boost (clamped). Idempotent — re-expiring an already-inactive record is a no-op.
  - `get_vouches()` now returns expired records with `active = false` regardless of stored state, so readers never see a stale active boost.
  - Added `baseline: u32` to `VouchRecord` (learner score before this vouch's boost) and a `get_reputation_score()` cross-contract read. New `remove_reputation_boost_clamped()` removes only `min(boost_amount, current_score - baseline)`, so removal can never drop the learner below their pre-vouch baseline even after penalties/config changes.
  - `revoke_vouch()` now uses the clamped removal instead of subtracting the raw `boost_amount`.
  - New event `VOUCHEXPIRED`; new error `VouchNotExpired = 13`.
- **Tests (`tests.rs`):** added `decrease_score` to the mock reputation contract and 6 new tests — permissionless expiry removes boost, expiry-before-TTL rejected (`VouchNotExpired`), idempotent expiry, revoke-after-expiry fails cleanly (`VouchNotActive`), `get_vouches` marks expired inactive without an explicit expire, and boost removal clamped to baseline (pre-existing reputation + penalty scenario).
- **Verification (initial):** `cargo test -p vouching-contract` → 24 passed, 0 failed.

### Issue #87 — Revision (automated audit follow-up)
- **Audit findings addressed:**
  - **Order-dependent drift on multiple overlapping vouches.** The initial per-record `baseline` (captured at each vouch's own time) made `remove_reputation_boost_clamped` non-additive: with two concurrent vouches across a boost-config change (boost 10 then 5), expiring the older/larger vouch first clamped the successor's removal to zero, permanently leaving a residual boost. Replaced per-record baseline with a **shared learner baseline** (score before ANY active vouch, captured on the first vouch and shared by all overlapping vouches) and an **aggregate `total_vouch_boost`** per learner. Removal is now `min(boost_amount, current - baseline)`, which is exact and order-independent.
  - **Missing required test:** added `test_boost_config_change_exact_older_larger_expired_first` and `test_boost_config_change_exact_newer_smaller_expired_first` covering issue acceptance criterion 2 (boost-config change between vouch and expiry keeps accounting exact), in both expiry orderings.
  - **`expire_vouch` idempotency / doc mismatch:** the TTL check previously preceded the active check, so calling within TTL on a revoked (inactive) record panicked `VouchNotExpired` instead of no-op. Reordered so an already-inactive record returns immediately (no-op) regardless of TTL; the TTL check only applies to still-active records. Added `test_expire_vouch_on_revoked_record_is_noop_within_ttl`.
  - **`storage.rs` left unchanged:** the aggregate baseline/total helpers now live in `storage.rs` (previously the TTL logic was only in `lib.rs`), satisfying the original files-to-touch note.
- **Storage changes:** `DataKey` gained `LearnerBaseline(Address)` and `LearnerTotalBoost(Address)`; `VouchRecord.baseline` field removed (no longer needed). `storage.rs` gained `get/set/clear_learner_baseline` and `get/set_total_vouch_boost`.
- **Verification (revised):** `cargo test -p vouching-contract` → 27 passed, 0 failed (3 new audit-follow-up tests).

---

## In Progress

### Issue #59 — Socialize Default Losses to Pool Share Price
- Added `absorb_loss(creditline, principal_shortfall)` entrypoint to `liquidity-pool-contract` restricted to the registered CreditLine

### Issue #79 — Admin Auth in Vendor Registry Initialize
- Added `admin.require_auth()` as the literal first line of `vendor_registry::initialize()` to prevent admin-hijack front-run attacks
- Reordered guard order to: `require_auth()` → `has_admin` check → state writes, consistent with `parameters-contract` and `liquidity-pool` patterns
- Wrote test proving unauthorized caller cannot complete initialization (auth failure, not just state rejection)
- Wrote test proving second `initialize()` call returns `AlreadyInitialized`
- All existing tests still pass; new test count has not decreased
- Fixed: No `.unwrap()` or `.expect()` introduced in user-facing paths
- Reduces both `locked_liquidity` and `total_liquidity` by the unrecovered principal, with independent caps to prevent negative accounting
- Added `LQLOSS` event (`emit_loss_absorbed`) to liquidity-pool events
- Updated `mark_defaulted()` to compute `principal_shortfall = principal_outstanding - guarantee_amount` and call `absorb_loss` after `receive_guarantee`
- Added 8 LP pool tests: basic absorption, share price drop, capping, partial repayment flow, unauthorized caller rejection, zero/negative amount rejection, event emission
- Added 4 creditline tests: absorb_loss called on default, zero-shortfall skip, partial repayment shortfall, end-to-end share price impact with real LP contract
- Updated MockLiquidityPool and MockLiquidityPoolEmpty with `absorb_loss` stub for test compatibility
- Fixed: `IntoVal` import moved before first usage in `test_mark_defaulted_loss_absorption_share_price_impact`

## Recently Fixed

### Security: Strict Error Handling & Atomic Invariant for Reputation Contract Calls
- **Problem:** `mark_defaulted()` and `handle_reputation_increase()` in `creditline-contract` used fire-and-forget `let _ = env.try_invoke_contract(...)` calls, discarding reputation contract errors. A reverting reputation contract (e.g. revoked updater status, TTL expiry, or WASM mismatch) allowed loans to settle/default while score mutations silently failed, creating state-reputation divergence.
- **Fix (Option a - Propagate Failure):**
  - Updated both `mark_defaulted()` and `handle_reputation_increase()` to strictly evaluate `try_invoke_contract` invocation & execution results.
  - If a reputation call fails when a reputation contract address is configured, `emit_score_update_failed(&env, borrower, is_increase, amount)` is emitted and the call panics with `panic_with_error!(&env, CreditLineError::ReputationCallFailed)`.
  - Atomically reverts the entire transaction, guaranteeing that loan status (Defaulted/Paid) and borrower reputation scores never diverge.
  - Documented Policy Option (a) in doc comments for both call sites. If no reputation contract is configured (`None`), score updates are intentionally skipped as the protocol is operating without on-chain reputation integration.
  - Added `ReputationCallFailed = 32` error variant in `contracts/creditline-contract/src/errors.rs`.
  - Added `emit_score_update_failed` helper in `contracts/creditline-contract/src/events.rs`.
- **Files:** `contracts/creditline-contract/src/lib.rs`, `contracts/creditline-contract/src/errors.rs`, `contracts/creditline-contract/src/events.rs`, `contracts/creditline-contract/src/tests.rs`, `context/progress-tracker.md`
- **New tests:** Updated `test_reputation_call_failure_reverts_repayment` and added `test_reputation_call_failure_reverts_default` verifying `ReputationCallFailed` error, `ScoreUpdateFailed` event, and atomic loan status revert on both score increase and decrease paths.
- **Verification:** All 141 tests in `creditline-contract` and all 389 tests across the entire workspace passed with zero failures.

### Security: `cancel_loan()` Ordering Discipline, Reentrancy Guard & Pre-Flight Check
- **Problem:** `cancel_loan()` in `creditline-contract` violated contract ordering discipline by omitting `enter_non_reentrant`/`exit_non_reentrant` guards, executing outbound token transfers before mutating status to `Cancelled` and persisting state, and missing token balance pre-flight validation (causing raw token panic on underfunded contract balance).
- **Fix:**
  - Wrapped mutation section in `enter_non_reentrant(&env)` and `exit_non_reentrant(&env)` matching sibling functions.
  - Reordered execution flow: validate parameters/auth → pre-flight contract token balance check → enter non-reentrant guard → mutate status to `Cancelled` and persist to storage → execute outbound guarantee token refund transfer → emit `LOANCNCL` event → exit non-reentrant guard.
  - Pre-flight check: queries creditline contract token balance against `loan.guarantee_amount`. If underfunded, returns `Err(CreditLineError::InsufficientRefundBalance)` leaving loan state in `Pending` without panic or state corruption.
  - Added new error variant `InsufficientRefundBalance = 31` to `CreditLineError`.
  - Signature updated to `pub fn cancel_loan(env: Env, caller: Address, loan_id: u64) -> Result<(), CreditLineError>`.
- **Files:** `contracts/creditline-contract/src/lib.rs`, `contracts/creditline-contract/src/errors.rs`, `contracts/creditline-contract/src/tests.rs`
- **New tests:** 7 comprehensive security & order proof unit tests (happy-case refund, admin cancellation, reentrancy lock rejection, unauthorized caller rejection, double-cancel rejection, underfunded contract typed error, state persistence ordering proof).

### Security: Unauthorized `distribute_interest` / `accumulate_interest` (SC-17)
- **Problem:** `distribute_interest()` and `accumulate_interest()` were public mutating functions with no `require_auth()` and no caller restriction. Any funded account could call them with an arbitrary amount, draining the pool's token balance to treasury and merchant fund addresses and inflating the share price so the caller could redeem LP shares for more than deposited.
- **Fix:** Changed both function signatures to accept `creditline: Address` as the first parameter. Added `creditline.require_auth()` as the literal first line and `Self::require_creditline(&env, &creditline)` as the second, matching `receive_repayment()` exactly. Both functions now pull `interest_amount` tokens into the pool via `token_client.transfer()` before any accounting change. Updated doc comments to remove the admin edge-case mention.
- **Internal call site preserved:** `receive_repayment()` still calls `distribute_interest_internal()` directly — it has already pulled funds and validated the caller, so it must not go through the newly guarded public wrappers.
- **Pre-existing bug fixed:** `calculate_withdrawal()` now returns 0 when the pool has no shares, fixing two pre-existing test failures.
- **Files:** `contracts/liquidity-pool-contract/src/lib.rs`, `contracts/liquidity-pool-contract/src/tests.rs`
- **New tests:** 8 new tests (unauthorized caller rejection for both functions, token pull + distribution for both, receive_repayment no-regression, receive_repayment single-distribution regression)
- **Verification:** `cargo check`, `cargo test -p liquidity-pool-contract` (86 passed, 0 failed), `cargo clippy -p liquidity-pool-contract -- -D warnings` (0 warnings)

### Issue #7 — Follow-up: Missing `approve_vendor` in `RealIntegrationCtx::register_vendor`
- Discovered second `register_vendor` helper in `RealIntegrationCtx` (integration test struct, ~line 2390) that only called `register_vendor` without `approve_vendor`
- All integration tests using `RealIntegrationCtx` created loans with `Pending` vendors → `validate_vendor` → `is_active` returned `false` → `VendorNotActive` (#3)
- Added `self.vendor_registry.approve_vendor(&self.admin, vendor)` after registration in `RealIntegrationCtx::register_vendor`

---

## Next Up (In Order)

1. **Learner grace period** — Make `grace_period_seconds` per-loan (not just global via parameters)
2. **Reputation rules** — Update `creditline-contract` to call different reputation adjustments for `LoanType::LearnerInstallment`
3. **Testnet deployment** ✅ — All 5 contracts deployed and initialized (see Contract Deployment Status below); IDs in StepFi-API env config
4. **End-to-end validation** — Verify loan lifecycle on testnet via Stellar CLI

---

## Open Questions

- What token is used for loans — native XLM or a USDC anchor? (Affects token contract address in `initialize()`)
- What is the correct `grace_period_seconds` for learner installment loans? (Longer than standard BNPL — possibly 7-14 days per installment)
- Should sponsor pool deposits go through `liquidity-pool-contract` or a new `sponsor-pool-contract`?

---

## Architecture Decisions

- **6 contracts, not 5** — `vouching-contract` added for mentor-based reputation boosting. `lp-contract` was dead code, removed. `liquidity-pool-contract` is the canonical LP implementation.
- **Vendor over Merchant** — Renamed to reflect StepFi's learning-focused domain.
- **TTL approach** — Using 60-day threshold / 120-day extension constants. Off-chain indexer is responsible for bumping TTL on active loan entries.
- **Upgrade pattern** — All contracts have `upgrade()` gated by admin `require_auth()`. Admin address is set at `initialize()` and transferable via `set_admin()`.
- **Loan sharding** — 32 shards (`loan_id % 32`) in creditline-contract to distribute persistent storage keys and avoid hot-key contention.
- **Reentrancy** — Boolean `LOCKED` flag in instance storage. Cheaper than mutex, sufficient for Soroban's single-threaded execution model.

---

## Contract Deployment Status

All 6 contracts are deployed, initialized, and active on Stellar testnet
(matches `README.md` and StepFi-Web `VERIFICATION.md`). These are the IDs
live clients (StepFi-Web `constants/config.ts`) point at:

| Contract | Testnet Deployed | Contract ID | Last Deployed |
|---|---|---|---|
| `reputation-contract` | ✅ Yes | `CC3BO57ZRJGA63QJBIBSOMI25Z3X2I5CYTARYRAUXUAILX6L3OWBL5SB` | 2026-05-11 |
| `parameters-contract` | ✅ Yes | `CCAE72SKYX55C5L56DBEFIMFVXRUIJY6JYLBREHEWRFNOW7AX5NBIJ5B` | 2026-05-11 |
| `vendor-registry-contract` | ✅ Yes | `CCZ6T6NYCDNI26VGTPXKKWQDR7JCIZZ24LCEG4MMYHZJAG6BPWIVAU2L` | 2026-05-11 |
| `liquidity-pool-contract` | ✅ Yes | `CACKE7ML2BTOAGQTAAW5NEARHCFX4PXXKGEO6GMU6NHFBVYQFZRJS2BT` | 2026-05-11 |
| `vouching-contract` | ⏳ Pending | `PENDING_DEPLOYMENT` | — |
| `creditline-contract` | ✅ Yes | `CAQDHYG3TALPNXG466SZUMJEPOI7VYV732LPFF3GHE4ASPBCNMIQBS3X` | 2026-05-12 (redeployed) |

Deployer: `GCOYDYSEHRCFWGXUCMPSQ3ODEY2LGMBSVKKCOFH4NRIK4DEEDSETH7BF`

> ✅ Resolved 2026-07-17: The 2026-05-11 set above (deployer `GCOYDYSE...H7BF`,
> = `stepfi-deployer` on the maintainer machine) is confirmed **live and correct**.
> A reproducible `stellar contract build` of current `main` (multi-sig admin
> included, commit `44a8c00`) produces bytecode whose SHA256 hashes match the
> on-chain wasm of all five contracts above exactly — the contracts were created
> in May and upgraded in place via their `upgrade()` functions as the source
> evolved. All clients (web, landing, docs, live API `.well-known/stellar.toml`)
> reference this set.
>
> The **second** deployment recorded on 2026-06-23 (deployer `GDL63O...Q4LH`) is
> identified as an **orphaned experimental deploy**: its key is not recognized on
> the maintainer machine, appears in no deploy script/env/shell-history, its
> account was funded by testnet Friendbot immediately before deploy (no memo), and
> its on-chain wasm matches no build of any branch in this repo. No client ever
> referenced it. It is now recorded under `orphanedDeployment` in
> `deployed-testnet.json` and marked DO NOT USE. Investigation into the origin of
> the `GDL63O...` key is **ongoing**.

> Update this table after running `scripts/deploy-testnet.sh`

---

## Session Notes

- Always run `cargo build` after any contract change before committing.
- Always run `cargo test` before marking any contract feature complete.
- Never modify storage key structures of a contract that has been deployed — it breaks existing data. Use a migration pattern or deploy a new contract.
- The `creditline-contract` depends on all other contracts — it must be initialized last.
- Do not add new workspace members to `Cargo.toml` without creating the full contract file structure first.
