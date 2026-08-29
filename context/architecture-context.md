# Architecture Context — StepFi-Contracts

## Role In The StepFi Ecosystem

StepFi-Contracts is the on-chain truth layer. Every financial operation in StepFi — loan creation, repayment, reputation scoring, liquidity deposits — is ultimately governed by these contracts. The API reads and builds transactions against them. The App signs and submits those transactions. The contracts are the only layer that cannot be patched with a hotfix — they require an upgrade deployment.

---

## Stack

| Layer | Technology | Version | Role |
|---|---|---|---|
| Language | Rust | stable | Contract implementation |
| SDK | Soroban SDK | 22.0.0 | Storage, events, cross-contract calls, auth |
| Build target | wasm32-unknown-unknown | — | WASM output for Stellar deployment |
| Testing | soroban_sdk::Env | — | In-process unit tests |
| Deploy | Stellar CLI | latest | Testnet and mainnet deployment |
| CI | GitHub Actions | — | Build + test on push/PR to main |

---

## Contracts (6 total)

| Crate | File | Purpose |
|---|---|---|
| `reputation-contract` | `contracts/reputation-contract/` | On-chain score (0–100) per wallet. Drives credit limits and interest rates. |
| `creditline-contract` | `contracts/creditline-contract/` | Core BNPL engine — loan creation, per-installment repayment, defaults, late fees, grace periods. |
| `liquidity-pool-contract` | `contracts/liquidity-pool-contract/` | Share-based LP pool — deposits, withdrawals, loan funding, interest distribution. |
| `vendor-registry-contract` | `contracts/vendor-registry-contract/` | Admin-managed whitelist of verified learning vendors. |
| `parameters-contract` | `contracts/parameters-contract/` | Governance-tunable protocol parameters — interest BPS, grace periods, min reputation. |
| `vouching-contract` | `contracts/vouching-contract/` | Mentor vouching — verified mentors boost learner reputation scores via cross-contract calls. |

---

## Contract Dependency Graph

```
creditline-contract
    ├── reputation-contract   (validate score, update score on repay/default)
    ├── vendor-registry-contract  (validate vendor is active)
    ├── liquidity-pool-contract   (fund loan, receive repayment, receive guarantee)
    └── parameters-contract   (get protocol params — interest BPS, grace period, etc.)

liquidity-pool-contract
    └── creditline-contract   (restricted — only creditline can call fund_loan, receive_repayment)

vouching-contract
    └── reputation-contract   (add_boost / remove_boost cross-contract calls)
```

### Initialization Order

Must be initialized in this exact dependency order:

```
1. parameters-contract
2. reputation-contract
3. vendor-registry-contract
4. liquidity-pool-contract
5. vouching-contract
6. creditline-contract
```

Each contract needs the addresses of its dependencies passed to `initialize()`.

---

## File Structure Per Contract

Every contract follows this internal structure:

```
contracts/<name>-contract/
├── Cargo.toml
└── src/
    ├── lib.rs          # Public contract interface (#[contract], #[contractimpl])
    ├── storage.rs      # All env.storage() calls — reads and writes
    ├── events.rs       # All env.events().publish() calls
    ├── types.rs        # Structs, enums, contracttype definitions
    ├── errors.rs       # Error enum with contracttype
    ├── access.rs       # Auth helpers (require_admin, require_updater, etc.)
    └── tests.rs        # Unit tests using soroban_sdk::Env::default()
```

---

## Storage Model

### Storage Types Used

| Type | Where used | Lifetime |
|---|---|---|
| Instance storage | Admin keys, contract references, global counters, pool totals, reentrancy lock | Lives with contract instance |
| Persistent storage | Individual loan records, user loan indices, user active debt, LP shares, merchant/vendor info | Survives archival — needs explicit TTL extension |
| Temporary storage | Not used | N/A |

### TTL Management

All persistent storage writes must call `extend_ttl()` immediately after the write. No exceptions.

```rust
// Constants defined in storage.rs of each contract
pub const PERSISTENT_TTL_THRESHOLD: u32 = 1_036_800; // 60 days in ledgers
pub const PERSISTENT_TTL_EXTEND_TO: u32 = 2_073_600; // 120 days in ledgers

// Pattern — always after every persistent write
env.storage().persistent().set(&key, &value);
env.storage().persistent().extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
```

### Loan Sharding (creditline-contract)

Loans are sharded into 32 buckets to avoid hot-key contention:

```rust
fn loan_shard(loan_id: u64) -> u32 {
    (loan_id % 32) as u32
}
// Key: DataKey::Loan(shard, loan_id)
```

---

## Reputation → Credit Tiers

| Score | Tier | Interest Rate (BPS) | Credit Limit |
|---|---|---|---|
| 90+ | Gold | 400 (4%) | 10,000 |
| 75–89 | Silver | 600 (6%) | 5,000 |
| 60–74 | Bronze | 800 (8%) | 2,500 |
| < 60 | Starter | 1,000 (10%) | 1,000 |

---

## Loan Lifecycle

```
create_loan() / request_loan()
        │
        ▼
    [Active] ◀──────── repay_loan() [partial]
        │
        ├── repay_loan() [full, balance = 0] ──▶ [Paid]
        │
        ├── apply_late_fees() [permissionless]
        │
        ├── warn_grace_period() ──▶ emit LOANGRC
        │
        └── mark_defaulted() ──▶ [Defaulted]

[Pending] ──▶ cancel_loan() ──▶ [Cancelled]
```

---

## Interest Distribution (liquidity-pool-contract)

On every `receive_repayment(principal, interest)`:
- **85%** stays in pool → inflates `total_liquidity` → LP share price appreciation
- **10%** → transferred to protocol treasury
- **5%** → transferred to vendor fund

---

## Events

### creditline-contract

| Symbol | Trigger |
|---|---|
| `LOANCRTD` | `create_loan()` |
| `LOANRQST` | `request_loan()` |
| `LOANRPD` | `repay_loan()` |
| `LOANDFLT` | `mark_defaulted()` |
| `LOANCNCL` | `cancel_loan()` |
| `LOANLTFE` | `apply_late_fees()` |
| `LOANGRC` | `warn_grace_period()` |

### liquidity-pool-contract

| Symbol | Trigger |
|---|---|
| `LQDEPST` | `deposit()` |
| `LQWTHDR` | `withdraw()` |
| `LQFUND` | `fund_loan()` |
| `LQREPAY` | `receive_repayment()` |
| `LQGUART` | `receive_guarantee()` |

### reputation-contract

| Symbol | Trigger |
|---|---|
| `SCORECHGD` | Any score mutation |
| `UPDCHGD` | Updater grant/revoke |

---

## Security Patterns

Every contract enforces these patterns:

- **Auth-first** — `require_auth()` is the first line of every mutating function
- **Reentrancy guard** — `LOCKED` boolean in instance storage, set before and cleared after every mutating function that touches shared state
- **Checked arithmetic** — `checked_add`, `checked_sub` everywhere. No unwrapped arithmetic
- **TTL extension** — on every persistent storage write
- **Role-based access** — admin, updaters (reputation), creditline-only (liquidity pool functions)

---

## Required Functions On Every Contract

```rust
pub fn get_version(env: Env) -> Symbol  // e.g., symbol_short!("1_0_0")
pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>)  // admin only
pub fn get_admin(env: Env) -> Address
pub fn set_admin(env: Env, new_admin: Address)
```

---

## Deployment

### Testnet

```bash
chmod +x scripts/deploy-testnet.sh
./scripts/deploy-testnet.sh
# Outputs contract IDs → add to StepFi-API .env
```

### Mainnet (Phase 10 — not yet)

Same script with `NETWORK=mainnet`. Requires funded mainnet account and security audit first.

---

## Invariants

1. **All storage reads and writes go through `storage.rs`** — no inline `env.storage()` in `lib.rs`
2. **All events go through `events.rs`** — no inline `env.events()` in `lib.rs`
3. **Every mutating function calls `require_auth()` first**
4. **Every persistent storage write calls `extend_ttl()` immediately after**
5. **Every contract has `get_version()` and `upgrade()`**
6. **Reentrancy guard is set before and cleared after every mutating function touching shared state**
7. **No unwrapped arithmetic anywhere** — use `checked_add`, `checked_sub`
8. **Cross-contract calls use generated client interfaces only** — no raw `invoke_contract` in production code
9. **Tests use `mock_all_auths()`** — no real key signing in unit tests
10. **Deployed contracts must not be modified** — use `upgrade()` with a new WASM hash instead.
