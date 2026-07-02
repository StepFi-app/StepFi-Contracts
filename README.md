# StepFi Contracts

> Soroban smart contracts powering the StepFi BNPL protocol on Stellar — open-source, auditable, and built for learners.

## Live on Stellar Testnet ✅

All 5 contracts are deployed, initialized, and active on Stellar testnet.

| Contract | Contract ID | Explorer |
|---|---|---|
| Creditline | `CAQDHYG3TALPNXG466SZUMJEPOI7VYV732LPFF3GHE4ASPBCNMIQBS3X` | [View ↗](https://stellar.expert/explorer/testnet/contract/CAQDHYG3TALPNXG466SZUMJEPOI7VYV732LPFF3GHE4ASPBCNMIQBS3X) |
| Reputation | `CC3BO57ZRJGA63QJBIBSOMI25Z3X2I5CYTARYRAUXUAILX6L3OWBL5SB` | [View ↗](https://stellar.expert/explorer/testnet/contract/CC3BO57ZRJGA63QJBIBSOMI25Z3X2I5CYTARYRAUXUAILX6L3OWBL5SB) |
| Liquidity Pool | `CACKE7ML2BTOAGQTAAW5NEARHCFX4PXXKGEO6GMU6NHFBVYQFZRJS2BT` | [View ↗](https://stellar.expert/explorer/testnet/contract/CACKE7ML2BTOAGQTAAW5NEARHCFX4PXXKGEO6GMU6NHFBVYQFZRJS2BT) |
| Vendor Registry | `CCZ6T6NYCDNI26VGTPXKKWQDR7JCIZZ24LCEG4MMYHZJAG6BPWIVAU2L` | [View ↗](https://stellar.expert/explorer/testnet/contract/CCZ6T6NYCDNI26VGTPXKKWQDR7JCIZZ24LCEG4MMYHZJAG6BPWIVAU2L) |
| Parameters | `CCAE72SKYX55C5L56DBEFIMFVXRUIJY6JYLBREHEWRFNOW7AX5NBIJ5B` | [View ↗](https://stellar.expert/explorer/testnet/contract/CCAE72SKYX55C5L56DBEFIMFVXRUIJY6JYLBREHEWRFNOW7AX5NBIJ5B) |

Deployer: `GCOYDYSEHRCFWGXUCMPSQ3ODEY2LGMBSVKKCOFH4NRIK4DEEDSETH7BF`
Deployed: 2026-05-11 (Creditline redeployed 2026-05-12)
Full deployment details: [`contracts/deployed-testnet.json`](./contracts/deployed-testnet.json)

---

## Architecture

StepFi uses 5 Soroban smart contracts that work together:

```
┌─────────────────────────────────────────────────┐
│                 StepFi Protocol                  │
├──────────────┬──────────────┬────────────────────┤
│  Creditline  │  Reputation  │  Liquidity Pool    │
│  (core BNPL) │  (scoring)   │  (sponsor capital) │
├──────────────┴──────────────┴────────────────────┤
│      Vendor Registry   │   Parameters            │
│      (vendor data)     │   (protocol config)     │
└─────────────────────────────────────────────────┘
```

### Contract Responsibilities

**Creditline** — The core lending contract:
- `create_loan()` — initiates a new BNPL loan
- `repay_installment()` — processes individual installment payments
- `approve_loan()` — transitions loan from Pending → Active
- Tracks LoanType (Standard, LearnerInstallment)
- Per-installment paid/unpaid tracking with timestamps
- Reentrancy guard on all mutating functions

**Reputation** — On-chain credit scoring:
- `get_score()` — returns borrower score (0-100)
- `update_score()` — updates score after payment events
- Score determines interest rate and credit limit:
  - 0-59 (Starter): 10% APR, $1,000 limit
  - 60-74 (Bronze): 8% APR, $2,500 limit
  - 75-89 (Silver): 6% APR, $5,000 limit
  - 90-100 (Gold): 4% APR, $10,000 limit

**Liquidity Pool** — Sponsor capital management:
- `deposit()` — sponsors add capital to the pool
- `withdraw()` — sponsors withdraw with yield
- `get_pool_info()` — returns pool stats

**Vendor Registry** — Learning vendor management:
- Stores verified vendor profiles
- Tracks vendor categories (School, Bootcamp, Electronics)
- Admin-controlled vendor approval

**Parameters** — Protocol governance:
- Base interest rates, penalty amounts
- Minimum guarantee percent (20%)
- Minimum reputation threshold (50)
- Grace period and large loan thresholds

---

## Getting Started

### Prerequisites
- Rust + wasm32-unknown-unknown target
- Stellar CLI v22+

```bash
# Install Rust target
rustup target add wasm32-unknown-unknown

# Install Stellar CLI
curl -L https://github.com/stellar/stellar-cli/releases/download/v22.8.1/stellar-cli-x86_64-unknown-linux-gnu.tar.gz \
  | tar -xz -C ~/.cargo/bin/
```

### Build

```bash
# Build all contracts
cargo build --target wasm32-unknown-unknown --release

# Run all tests
cargo test --manifest-path contracts/creditline-contract/Cargo.toml
```

### Test Results

```
test result: ok. 93 passed; 0 failed; 4 ignored
```

### Deploy to Testnet

```bash
# Generate deployer keypair
stellar keys generate stepfi-deployer --network testnet
stellar keys fund stepfi-deployer --network testnet

# Run deploy script
chmod +x scripts/deploy-testnet.sh
./scripts/deploy-testnet.sh
```

---

## Contract Security

- All mutating functions require `require_auth()`
- Reentrancy guard on loan operations
- TTL extension on every persistent storage write
- Checked arithmetic on all balance operations
- No `unwrap()` on user-facing paths (hardening in progress)

---

## Contributing

Read [`context/code-standards.md`](./context/code-standards.md) before contributing.

Checklist for every PR:
- [ ] `cargo build` passes with zero errors
- [ ] `cargo test` — all 93 existing tests still pass
- [ ] `require_auth()` is first line of every mutating function
- [ ] `extend_ttl()` called after every persistent storage write
- [ ] New tests written for any new function

Browse open issues: [StepFi-app/StepFi-Contracts/issues](https://github.com/StepFi-app/StepFi-Contracts/issues)

---

## Contributors

<!-- LEADERBOARD_START -->
## 🏆 Top 3 Contributors

<div align="center">

<table>
<tr>

<td align="center">
  <a href="https://github.com/EmeditWeb">
    <img src="https://avatars.githubusercontent.com/u/77761768?v=4" width="100px;" style="border-radius:50%;" alt="EmeditWeb"/><br />
    <sub><b>🥇 @EmeditWeb</b></sub><br />
    <sub>24 contributions</sub>
  </a>
</td>

<td align="center">
  <a href="https://github.com/Dopezapha">
    <img src="https://avatars.githubusercontent.com/u/141345379?v=4" width="100px;" style="border-radius:50%;" alt="Dopezapha"/><br />
    <sub><b>🥈 @Dopezapha</b></sub><br />
    <sub>3 contributions</sub>
  </a>
</td>

<td align="center">
  <a href="https://github.com/KingFRANKHOOD">
    <img src="https://avatars.githubusercontent.com/u/168771603?v=4" width="100px;" style="border-radius:50%;" alt="KingFRANKHOOD"/><br />
    <sub><b>🥉 @KingFRANKHOOD</b></sub><br />
    <sub>2 contributions</sub>
  </a>
</td>

</tr>
</table>
</div>

<!-- LEADERBOARD_END -->

---

## License

MIT — see [LICENSE](./LICENSE)

Part of the [StepFi Protocol](https://github.com/StepFi-app) · Built on [Stellar](https://stellar.org) · Powered by [Soroban](https://soroban.stellar.org)
