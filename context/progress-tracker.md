# Progress Tracker — StepFi-Contracts

Update this file after every completed contract change, fix, or architectural decision. Progress state must reflect the actual deployed and tested state — not the intended state.

---

## Current Phase

**Phase 1 — Contract Infrastructure & Core Fixes**

## Current Goal

`LoanType` and per-installment tracking are in. Next: per-loan grace period (Next Up #4), then vouching contract.

---

## Completed

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

---

## In Progress

### Issue #59 — Socialize Default Losses to Pool Share Price
- Added `absorb_loss(creditline, principal_shortfall)` entrypoint to `liquidity-pool-contract` restricted to the registered CreditLine
- Reduces both `locked_liquidity` and `total_liquidity` by the unrecovered principal, with independent caps to prevent negative accounting
- Added `LQLOSS` event (`emit_loss_absorbed`) to liquidity-pool events
- Updated `mark_defaulted()` to compute `principal_shortfall = principal_outstanding - guarantee_amount` and call `absorb_loss` after `receive_guarantee`
- Added 8 LP pool tests: basic absorption, share price drop, capping, partial repayment flow, unauthorized caller rejection, zero/negative amount rejection, event emission
- Added 4 creditline tests: absorb_loss called on default, zero-shortfall skip, partial repayment shortfall, end-to-end share price impact with real LP contract
- Updated MockLiquidityPool and MockLiquidityPoolEmpty with `absorb_loss` stub for test compatibility
- Fixed: `IntoVal` import moved before first usage in `test_mark_defaulted_loss_absorption_share_price_impact`

## Recently Fixed

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

- **5 contracts, not 6** — `lp-contract` was dead code, removed. `liquidity-pool-contract` is the canonical LP implementation.
- **Vendor over Merchant** — Renamed to reflect StepFi's learning-focused domain.
- **TTL approach** — Using 60-day threshold / 120-day extension constants. Off-chain indexer is responsible for bumping TTL on active loan entries.
- **Upgrade pattern** — All contracts have `upgrade()` gated by admin `require_auth()`. Admin address is set at `initialize()` and transferable via `set_admin()`.
- **Loan sharding** — 32 shards (`loan_id % 32`) in creditline-contract to distribute persistent storage keys and avoid hot-key contention.
- **Reentrancy** — Boolean `LOCKED` flag in instance storage. Cheaper than mutex, sufficient for Soroban's single-threaded execution model.

---

## Contract Deployment Status

All 5 contracts are deployed, initialized, and active on Stellar testnet
(matches `README.md` and StepFi-Web `VERIFICATION.md`). These are the IDs
live clients (StepFi-Web `constants/config.ts`) point at:

| Contract | Testnet Deployed | Contract ID | Last Deployed |
|---|---|---|---|
| `reputation-contract` | ✅ Yes | `CC3BO57ZRJGA63QJBIBSOMI25Z3X2I5CYTARYRAUXUAILX6L3OWBL5SB` | 2026-05-11 |
| `parameters-contract` | ✅ Yes | `CCAE72SKYX55C5L56DBEFIMFVXRUIJY6JYLBREHEWRFNOW7AX5NBIJ5B` | 2026-05-11 |
| `vendor-registry-contract` | ✅ Yes | `CCZ6T6NYCDNI26VGTPXKKWQDR7JCIZZ24LCEG4MMYHZJAG6BPWIVAU2L` | 2026-05-11 |
| `liquidity-pool-contract` | ✅ Yes | `CACKE7ML2BTOAGQTAAW5NEARHCFX4PXXKGEO6GMU6NHFBVYQFZRJS2BT` | 2026-05-11 |
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
