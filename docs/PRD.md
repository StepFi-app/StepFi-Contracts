# StepFi — Product Requirements Document (PRD)

> **Status:** v1.1 — core decisions ratified (§13) · **Date:** 2026-09-10 · **Last updated:** 2026-09-10
> **Owner:** StepFi (`eitighis`) · **Org:** `StepFi-app`
> **This document is the single canonical source of truth for what StepFi is, what it must become, and the bar every contribution is held to.** Where any other document disagrees with this one, this one wins until amended. Amendments are made by PR to this file, approved by the owner.

---

## 0. Document control

### 0.1 What this supersedes
This PRD replaces a scattered, stale, and internally-contradictory set of "foundation" documents. On ratification, the following are demoted to *derived* or *archived* status and must be reconciled to this PRD (tracked in §14):

| Document | Repo | Problem | Action |
|---|---|---|---|
| `docs/PROJECT_CONTEXT.md` | Contracts | Stale ("Phase 2 complete, 8/20 issues"); lists deployed contracts as "planned"; describes the abandoned *generic e-commerce* BNPL model ("Maria's laptop", "merchant") | Rewrite as derived vision doc or archive |
| `docs/ROADMAP.md` | Contracts | Contract-only SC-01…20 tracker; "Merchant" naming; omits vouching + parameters; lists removed `adapter-trustless-contract` | Replace with `PROGRESSION.md` (§12) |
| `AI_CONTEXT.md` | Contracts | Mis-states status/line counts | Regenerate as pointer to this PRD |
| `context/project-overview.md` | Contracts/API/App (3 copies) | The best existing de-facto PRD, but modest | Fold into this PRD; keep as short per-repo pointer |
| `ROADMAP.md` / `PROPOSAL.md` | Contracts/API/App (copies) | 10-phase product roadmap + pitch; not reconciled with contract roadmap | PROPOSAL kept as pitch; ROADMAP superseded by `PROGRESSION.md` |
| Tier / threshold tables | Web docs, Mintlify, README | Contradict each other (min-to-borrow 40 vs 50 vs 0; two tier schemes) | Reconcile to §7.1 canonical table |

### 0.2 Related living documents
- **`docs/PROGRESSION.md`** — the phased, per-repo execution plan that turns this PRD into the epic/issue backlog seeding structured PRs (built next).
- **`stepfi-audit-bot/CODING_STANDARDS.md`** — the machine-enforced PR-quality and code-quality rubric (tightened next).
- **`context/architecture-context.md`, `code-standards.md`, `progress-tracker.md`** (per repo) — engineering foundation docs; remain authoritative for *engineering* detail and must not contradict this PRD.

### 0.3 Reading order for a new contributor
`PRD.md` (this) → repo `context/*` → `PROGRESSION.md` → the specific epic/issue → `CODING_STANDARDS.md` (before opening a PR).

---

## 1. Vision & mission

**One-liner:** *StepFi is an open-source, Stellar-native Buy-Now-Pay-Later protocol for learners, interns, and early-career developers in emerging markets — built as composable on-chain infrastructure, not a single closed app.*

**Tagline (product):** "Step into your future — pay small small."

**North star (the "tooling system" mandate):** StepFi's on-chain reputation, credit, liquidity, and vendor primitives are built as **reusable, independently-consumable infrastructure**. The learner-BNPL experience is the **flagship application** of that infrastructure — the first, not the only, consumer of it. This is a deliberate positioning decision (see §3.3) that gives the project the surface area to sustain many high-quality open-source contributions rather than the thin issue-supply of a single dApp.

**Mission:** Give people with no bank and no credit history a way to finance their own education and tooling, repay in small on-chain installments, and build a portable reputation that other protocols can trust and reuse.

---

## 2. The problem

Emerging-market learners, interns, and early-career developers cannot afford the laptops, courses, bootcamps, and tools that would let them earn — and traditional finance will not lend to them because they have no credit history, no collateral, and often no bank account. Existing crypto lending is over-collateralized (you must already have money to borrow money), which excludes exactly the people who need credit to start.

**The gap:** there is no under-collateralized, reputation-based, low-friction credit rail for education spend that (a) works from a wallet alone, (b) is cheap enough for small-ticket loans, and (c) produces a portable credit history the borrower actually owns.

---

## 3. Solution & positioning

### 3.1 What StepFi does
1. A **learner** connects a Stellar wallet, builds an on-chain **reputation score**, and borrows against it to buy approved learning products from whitelisted **vendors**.
2. **Sponsors** deposit into a **liquidity pool** and earn yield from loan interest (share-price appreciation).
3. **Mentors** **vouch** for learners, raising their credit limits and staking reputation on the outcome.
4. Repayment is in small installments; on-time repayment raises reputation and unlocks better terms; default socializes losses to the pool and damages reputation.

### 3.2 Why Stellar/Soroban
Sub-cent fees make small-ticket installment loans economically viable; fast finality suits a mobile-first repayment UX; Soroban gives the protocol programmable, upgradeable, auditable on-chain logic; the Stellar Asset Contract (SAC) provides standard token rails.

### 3.3 Positioning decision — "tooling system, not just a dApp" (RATIFY)
Three dials were considered:
- **(a) Product-only** — a single BNPL dApp; harden what exists. *Rejected:* cannot honestly support "capacity for many open-source issues" or 200–400 quality PRs/campaign.
- **(b) Product + infrastructure** *(ADOPTED as north star)* — ship the BNPL flagship **and** expose the reputation/credit/liquidity/vendor primitives, the indexer, an integration SDK, dashboards, and developer tooling as independently-usable pieces. Lowest-regret: keeps everything already built, and creates real, non-padded work surface.
- **(c) Full platform pivot** — reposition primarily as generic on-chain credit infrastructure with BNPL as a demo. *Deferred:* higher risk, larger rewrite; can be reached later from (b).

**The existing hook:** `docs/PROJECT_CONTEXT.md` already contains a "Reputation Portability" section ("any dApp can query StepFi reputation", "DAOs can use it for governance weight", "Merchant SDK for easy integration"). (b) formalizes that thread as a first-class product surface.

> **Decision needed from owner:** confirm (b), or redirect to (a)/(c). Everything downstream (architecture surface, PROGRESSION backlog size, grant narrative) assumes **(b)**.

---

## 4. Users & personas

| Persona | Needs | Primary surfaces |
|---|---|---|
| **Learner** (borrower) | Register with wallet, see reputation, apply for a loan, sign the Soroban tx, repay in installments, watch reputation grow | App (mobile-first), Web |
| **Sponsor** (LP) | Deposit into the pool, track position and yield, withdraw | App, Web |
| **Mentor** | Get verified, vouch for a learner, see the effect on their limit, stake reputation | App, Web |
| **Vendor** | Register, get approved, receive loan-funded payments for learning products | Web, API |
| **Integrator / third-party dApp** *(tooling-system persona)* | Query a wallet's StepFi reputation; consume events; integrate credit primitives via SDK | Contracts (read APIs), API, SDK, Docs |
| **Contributor** (OSS dev) | Find a well-specified issue, build to standard, open a PR that passes the gate, get paid via Grantfox | GitHub issues, `CODING_STANDARDS.md`, all repos |
| **Operator / admin** | Deploy, upgrade (multisig+timelock), tune parameters, monitor health, replay/reconcile | Contracts admin, API admin, monitoring |

---

## 5. Scope

### 5.1 In scope
- Six Soroban contracts (§7) on **testnet**, verified, with matching client references everywhere.
- **API** (NestJS/Fastify): auth, indexer, tx-building, reconciliation, health, OpenAPI, admin.
- **App** (React Native/Expo): the flagship mobile learner/sponsor/mentor experience.
- **Web** (Vite/React): browser experience + protocol dashboards. *(Reconciliation: the Web app is IN scope — it is built and live. The old "mobile-first only, web out-of-scope" line in `project-overview.md` is corrected here.)*
- **Docs** (Mintlify) + **landing** (Next.js) + **org profile** — public truth surface.
- **stepfi-audit-bot** — the PR-quality/code-quality enforcement tool (a first-class part of the "tooling system").
- **Tooling-system surface:** reputation read-APIs, an integration SDK, event/indexer access, contributor tooling.
- **Multiple Stellar payment systems** — staged expansion beyond Soroban-SAC (§8).
- **Gas/resource-fee benchmarking + documentation** per transaction (§9.2).
- **Vercel migration** for the appropriate web surfaces + CI/CD deploy, new secret, retire old deployments (§9.7).

### 5.2 Out of scope (for now)
- **Mainnet** — gated behind a completed third-party security audit (roadmap Phase "Mainnet & Growth").
- **Custodial funds / holding user keys** — StepFi is non-custodial by design.
- **Fiat on/off-ramp built in-house** — may be delivered *via SEP-24/SEP-31 anchors* (§8) rather than custom; **ratified (§13, 2026-09-10): deferred — evaluated in Phase 4, not yet committed**.
- Native token / StepFi coin — not planned.

### 5.3 Contradictions explicitly reconciled by this PRD
1. **Contract count: SIX, not five.** The `vouching-contract` is implemented (Issue #4) and is a first-class contract; it is currently **undeployed and absent from public overviews**. This PRD treats StepFi as a **6-contract** system and makes vouching deployment a tracked deliverable (§7, §13).
2. **Web app is IN scope** (see §5.1).
3. **Tier/threshold tables** are unified in §7.1; all other copies are derived and must match.
4. **Stack facts:** API is **NestJS + Fastify** (not "Express"); App is **React Native + Expo**; Web is **Vite + React**. All docs claiming otherwise are wrong and tracked in §14.

---

## 6. System architecture (product-level)

### 6.1 Repository map

| Repo | Role | Stack | Deploy target | Status |
|---|---|---|---|---|
| **StepFi-Contracts** | On-chain truth layer (6 contracts) | Rust / Soroban SDK 22 / wasm32 | Stellar testnet (mainnet later) | ✅ in org |
| **StepFi-API** | Reads/builds txs, indexes events, auth, admin | NestJS 11 / Fastify / Postgres / Redis / Stellar SDK | Render (live) | ✅ in org |
| **StepFi-App** | Flagship mobile experience | React Native / Expo SDK 54 | Expo / stores (later) | ✅ in org |
| **StepFi-Web** | Browser experience + dashboards | Vite / React 19 | **Vercel** (migrating from Netlify) | ✅ in org |
| **StepFi-Docs** | Product & protocol docs | Mintlify | Vercel/hosted | ✅ in org |
| **stepfi-landing** | Marketing landing + Grantfox surface | Next.js | Vercel | ▢ planned (not yet in org) |
| **stepfi-audit-bot** | PR-quality/code-quality gate + stale-issue nudger | Python | Local/host runner | ▢ exists locally; not yet pushed to org |
| **stepfi-github-profile** | Org profile / link hub | Markdown | GitHub | ▢ planned (not yet in org) |

> **Repo reality (2026-09-10):** the org `StepFi-app` currently holds **five code/docs repos + `.github`** (the five marked ✅). `stepfi-landing`, `stepfi-audit-bot`, and `stepfi-github-profile` are planned surfaces that are **not yet pushed to the org**; creating/pushing them is tracked work (audit-bot → `ORG-E1.2`; landing → `DEP-E6.1`; profile → `ORG-E0.1`/docs). Tooling that enumerates repos (e.g. the audit bot's `GITHUB_REPOS`) must list only repos that exist, or it will error polling the missing ones.

### 6.2 Composition

```
                         ┌─────────────────────────────────────────────┐
                         │              STELLAR TESTNET                 │
                         │  ┌────────────┐ reads/updates ┌───────────┐  │
   signs & submits ─────▶│  │ creditline │──────────────▶│reputation │  │
        (App/Web)        │  │  (core)    │──┐            └───────────┘  │
                         │  └────────────┘  │  ┌───────────┐            │
                         │        │         ├─▶│ liquidity │            │
                         │        │         │  │   pool    │            │
                         │        │         │  └───────────┘            │
                         │        │         ├─▶┌───────────┐            │
                         │        │         │  │  vendor-  │            │
                         │        │         │  │ registry  │            │
                         │        │         │  └───────────┘            │
                         │        │         └─▶┌───────────┐            │
                         │        │            │parameters │            │
                         │        ▼            └───────────┘            │
                         │  ┌────────────┐  (reputation boost)          │
                         │  │  vouching  │──────────────▶ reputation    │
                         │  └────────────┘   [NOT YET DEPLOYED]         │
                         └───────────────▲──────────────────┬──────────┘
                                         │ events            │ read/build tx
                                         │                   │
                                   ┌─────┴───────────────────▼─────┐
                                   │        StepFi-API (Render)     │
                                   │  indexer · tx-build · auth ·   │
                                   │  admin · reconciler · health · │
                                   │  reputation read-API (tooling) │
                                   └─────┬───────────────────┬──────┘
                                         │                   │
                              ┌──────────▼─────┐    ┌────────▼────────┐   ┌──────────────┐
                              │  StepFi-App    │    │   StepFi-Web    │   │  Integrators │
                              │  (flagship)    │    │  + dashboards   │   │  / SDK (tool)│
                              └────────────────┘    └─────────────────┘   └──────────────┘
```

### 6.3 Layering rules
- **Contracts** are the only source of financial truth and the only layer that cannot be hotfixed — changes require an upgrade deployment (multisig + timelock).
- **API** never holds keys or funds; it reads chain state, builds unsigned transactions, indexes events, and serves reputation/credit data (including to third parties — the tooling surface).
- **App/Web** sign and submit; never trust client state for authorization.
- **SDK/integrators** consume read-APIs and events; never get privileged write access.

---

## 7. Protocol mechanics (canonical)

### 7.1 Reputation → credit tiers (CANONICAL — all other tables derive from this)

| Score | Tier | Interest (BPS) | Credit limit |
|---|---|---|---|
| 90–100 | Gold | 400 (4%) | 10,000 |
| 75–89 | Silver | 600 (6%) | 5,000 |
| 60–74 | Bronze | 800 (8%) | 2,500 |
| 0–59 | Starter | 1,000 (10%) | 1,000 |

- Score range: **0–100**. New wallets start at the contract default (currently `0` → Starter terms).
- **Open decision (§13):** whether there is a hard *minimum-to-borrow* floor (docs variously say 40 / 50) or whether score-0 wallets borrow at Starter terms. **Ratified (2026-09-10): no hard floor — score-0 wallets borrow at Starter terms.**

### 7.2 Loan lifecycle

```
create_loan()/request_loan() ─▶ [Active] ─ repay (partial) ─▶ [Active]
     │                               │
     │                               ├─ repay (balance=0) ─▶ [Paid]  (+reputation)
     │                               ├─ apply_late_fees() [permissionless]
     │                               ├─ warn_grace_period() ─▶ emit LOANGRC
     │                               └─ mark_defaulted() ─▶ [Defaulted] (−reputation, socialize loss)
[Pending] ─ cancel_loan() ─▶ [Cancelled]
```
- **Repayment waterfall (ratified, implemented):** late fees → interest → service fee → principal.
- **Installments:** `LoanType::LearnerInstallment` supports per-installment tracking (`paid`, `paid_at`).

### 7.3 Liquidity-pool economics
On each `receive_repayment(principal, interest)`:
- **85%** stays in pool → `total_liquidity` grows → LP share price appreciates.
- **10%** → protocol treasury.
- **5%** → vendor fund.

On default: `absorb_loss(principal_shortfall)` reduces `locked_liquidity` and `total_liquidity` (loss socialized to share price), where `shortfall = principal_outstanding − guarantee_amount`.

### 7.4 Vouching
Verified mentors vouch for learners → reputation boost (`add_boost`/`remove_boost`, updater-gated) → higher effective limit. Vouches are revocable and indexed learner→mentor. **Contract exists; must be deployed and wired end-to-end.**

### 7.5 Parameters / governance
Governance-tunable protocol parameters (interest BPS, grace periods, min reputation) live in `parameters-contract`, admin-gated. Upgrades go through **multisig + timelock** (no single-key instant upgrade).

### 7.6 Loan asset (RATIFY)
Docs commit to **USDC**; the contract tracker still lists "XLM vs USDC?" as open. **Ratified (§13, 2026-09-10): USDC (via SAC) as the loan/repayment asset**, with the token address injected at `initialize()`.

---

## 8. Stellar payment systems (current + expansion)

The grant requirement is *"incorporate multiple Stellar payment systems."* Today StepFi is **Soroban-invoke + custom auth only**. This is the committed staged plan to satisfy the requirement honestly.

| Rail | Status today | Plan |
|---|---|---|
| **Soroban SAC transfers** (deposits, loans, repayments) | ✅ Implemented (real simulate→assemble→XDR) | Keep as core |
| **Custom wallet-signature auth** (Ed25519 / SEP-0043 prefixed message → JWT) | ✅ Implemented | Keep short-term; migrate to SEP-10 |
| **`.well-known/stellar.toml`** (SEP-1 metadata) | ⚠️ Partial (missing `parameters` contract + SEP-1 fields) | Complete SEP-1 fields |
| **SEP-10 Web Authentication** | ❌ None | **Add** — replace bespoke nonce/JWT with standard challenge/response |
| **Classic payments** (XLM / asset) | ❌ None | **Add** — direct classic repayment/disbursement path |
| **Path payments** | ❌ None | **Add** — let a learner repay in any asset, settle in the loan asset |
| **SEP-24 hosted deposit/withdraw** (anchor) | ❌ None | **Evaluate** — fiat on/off-ramp for repayment (decision §13) |
| **SEP-31 cross-border** | ❌ None | **Evaluate** — sponsor/remittance flows (decision §13) |
| **Recurring payments** | Partial (keeper concept in API) | Harden into a first-class recurring-repayment rail |

Each rail becomes one or more epics in `PROGRESSION.md` with the full IndigoPay treatment.

---

## 9. Non-functional requirements

### 9.1 Security (non-negotiable)
- The **10 contract invariants** (auth-first, storage/events only in their modules, TTL-extend after every persistent write, checked arithmetic, reentrancy guard, generated-client-only cross-contract calls, `get_version`/`upgrade` on every contract, no modifying deployed storage layouts, `mock_all_auths` in tests) are binding.
- API: role re-verified server-side (never trust JWT claims alone), atomic single-use nonces, domain-bound signature challenges, helmet applied, strict CORS (no `*`+credentials), secrets only via config.
- **No secrets in source or automation.** The leaked `ghp_…` PAT found in multiple repos' `.git/config` must be rotated by the owner and purged (§13).
- Every security fix ships with a regression test that fails pre-fix and passes post-fix.

### 9.2 Gas / resource-fee (grant requirement)
- **Benchmark and document resource-fee cost per public transaction** for every contract (a `gas-benchmarks.md` per contract + a workspace summary), measured reproducibly (script in CI).
- Optimize toward "ultra-low" cost: minimize persistent writes, batch TTL extensions where sound, avoid unbounded storage, prefer instance storage for hot small values — **without** violating invariants.
- CI publishes a resource-fee report artifact; regressions beyond a threshold fail the build.

### 9.3 Testing
- **80%+ coverage**, TDD (RED→GREEN→REFACTOR). Unit + integration + E2E on critical flows.
- Contracts: `Env::default()` + `mock_all_auths()`; every public function has ≥1 test.
- Test suites must **pass in bounded time** — the API suite's open-handle leaks/hangs are a defect to fix, not tolerate (CI must be trustworthy).

### 9.4 Error-code taxonomy (grant requirement: ~250)
- A **typed, namespaced, catalogued** error model across the system (contracts + API + app + web), each code stable and documented. Current inventory: Contracts ~84, API ~86 (uncatalogued literals), App ~7 (dead), Web ~0 typed.
- Target **~250 distinct, documented codes** system-wide, delivered as a real catalog (enums/registries + a generated reference), **not** padding. Growth is a natural output of the expansion work (payment rails, tooling APIs), not invented codes.

### 9.5 CI that actually gates
- **Every repo's CI must build the real thing, run the real tests, and fail on real failure.** The current pattern — green CI that runs 1 of 6 suites, or only a web export, hiding broken builds, mock auth, and faked features — is the root cause of the stall and is prohibited going forward.
- CI gates: build + full tests + lint + typecheck + coverage floor + secret scan + (Contracts) resource-fee report + (Web/landing) Vercel deploy.

### 9.6 Observability
- Structured logging, health/readiness probes, and metrics (Prometheus where a server exists). Indexer lag, RPC/circuit-breaker state, and money-path counters are first-class (IndigoPay-style).

### 9.7 Deployment & Vercel migration (grant requirement)
- Migrate the appropriate web surfaces to **Vercel**; add **Vercel deploy to CI/CD**; store a **new Vercel secret**; update project + READMEs with the new link; **retire the old deployments** (Netlify / stale `*.vercel.app`) after cutover.
- API stays on Render (documented) unless a move is ratified.

### 9.8 Documentation truth
- Every README describes the **actual** feature set and stack. No wrong-project READMEs (the API's `# Supabase CLI` README is a CRITICAL fix). Docs↔code parity is checked in CI where feasible.

---

## 10. Success criteria & grant KPIs

| # | Criterion (definition of done) | Measure | Status |
|---|---|---|---|
| 1 | Learner registers, signs in, sees reputation end-to-end on testnet | E2E green | Partial (App auth mocked) |
| 2 | Learner applies, signs Soroban tx, sees confirmation | E2E green | Partial |
| 3 | Sponsor deposits & tracks position | E2E green | Partial |
| 4 | Mentor vouch raises limit end-to-end | E2E green + **vouching deployed** | Blocked (undeployed) |
| 5 | Background jobs index events + send reminders | Indexer live near tip | ✅ (API indexer live) |
| 6 | All contracts deployed on testnet with **verified** IDs, reconciled everywhere | 6/6 verified + client parity | Partial (5/6, vouching pending) |
| 7 | API live with health + Swagger | Live probes green | ✅ |
| 8 | App has working Expo preview | Build + preview green | Partial (TS errors) |
| 9 | ≥20 well-labeled issues | Issue count | ✅ |
| 10 | Submitted to Drips/Grantfox with live URLs (GitHub, Vercel, Docs, API) | Submission complete | Pending |
| 11 | **Gas benchmarked & documented** per tx | Report artifact exists | ❌ |
| 12 | **~250 documented error codes** | Catalog count | ❌ (~177 raw, uncatalogued) |
| 13 | **Multiple Stellar payment systems** live | ≥3 rails beyond SAC | ❌ |
| 14 | **CI gates everything**, all repos green *for real* | No false-green | ❌ |
| 15 | **Tooling surface** usable by a third party (reputation read + SDK + docs) | Demo integration | ❌ (net-new) |
| 16 | **200–400 quality PRs / campaign** merged across repos through the gate | Merged-PR count | Mechanism to build |

---

## 11. Funding model & the PR pipeline

**How StepFi gets paid:** Grantfox is a **per-merged-PR bounty** program, not a milestone contract (campaign 1: "59 PRs merged, 26 contributors"; campaign 2 open). Rewards flow when **quality PRs merge**. Therefore:

> The stall ("3 months, unpaid") maps directly onto: **false-green CI → low-quality/faked PRs → nothing merges cleanly → no rewards.** The fix is not cosmetic — a trustworthy CI plus a strict-but-*passable* PR pipeline that actually lands quality PRs **is the revenue mechanism.**

**The pipeline (to be documented and strictly followed):**
1. **Well-specified epics/issues** (IndigoPay #1098 shape): Objective → Problem (with evidence) → Scope → Implementation → Acceptance Criteria → Testing.
2. **Contributor builds to `CODING_STANDARDS.md`.**
3. **PR must match the IndigoPay #1211 shape** (exec summary → file-by-file audit vs the issue → design + typed errors → verification with exact CI commands + test matrix).
4. **stepfi-audit-bot** audits every PR against the linked issue + standards; posts APPROVE / REQUEST_CHANGES + a Telegram verdict; nags stale assignees. Its scrutiny is tightened (next deliverable) to enforce PR *structure* and *code quality*, and extended beyond API+Contracts to all repos.
5. **CI must be green for real.** Merge only on green + bot-approve + human review.

**Target throughput:** 200–400 merged, gate-passing PRs collectively per campaign across all repos — sustainable because the tooling-system surface (§3.3) supplies genuine, non-padded work.

---

## 12. Roadmap (north-star phases)

Detailed, per-repo, issue-level execution lives in **`docs/PROGRESSION.md`** (built next). High-level phases:

- **Phase 0 — Stop the bleeding (precondition for everything).** Fix all CRITICAL lapses: rotate the leaked PAT; make every CI gate real; fix the API's wrong-project README + nonce TOCTOU; fix the App's mock-auth + mainnet-hardcode + TS errors; fix the Web faked vouch; reconcile the Contracts `.env`/orphan-deployment; deploy `vouching`. **No feature work merges until Phase 0 is green.**
- **Phase 1 — Truthful foundation.** This PRD ratified; docs reconciled (§14); READMEs truthful; `PROGRESSION.md` + tightened bot live; error-code catalog scaffolded.
- **Phase 2 — Flagship end-to-end on testnet.** All 6 success-flow E2Es (learner, sponsor, mentor) green on testnet; 6/6 contracts verified with client parity.
- **Phase 3 — Gas & quality.** Resource-fee benchmarking + docs per tx; coverage floors; error-code taxonomy to target.
- **Phase 4 — Payment rails.** SEP-10, classic + path payments; SEP-24/31 evaluation; recurring repayment.
- **Phase 5 — Tooling surface.** Reputation read-API + integration SDK + dashboards + a reference third-party integration.
- **Phase 6 — Deploy & submit.** Vercel migration + CI/CD + secret rotation + retire old deploys; Grantfox/Drips submission with all links.
- **Phase 7 — Mainnet & growth.** Third-party security audit → mainnet → mobile release → first learner cohort.

---

## 13. Risks & open decisions

> **Decision log — ratified by owner (`eitighis`) on 2026-09-10:** D1 **(b)**, D2 **USDC**, D3 **no hard floor**, D5 **anchors deferred to Phase-4 evaluation**, D6 **grace period ≈14 days/installment (governance-tunable)**. These are now binding on the PRD and flow into `PROGRESSION.md`. **Still open:** D4 (sponsor-pool structure). **Outstanding owner action:** R1 (rotate the leaked PAT).

| # | Decision / risk | Ratified position | Status |
|---|---|---|---|
| D1 | **Tooling-system depth** (a/b/c, §3.3) | **(b)** product + reusable infrastructure | ✅ ratified 2026-09-10 |
| D2 | **Loan asset** XLM vs USDC (§7.6) | **USDC via SAC**, token address injected at `initialize()` | ✅ ratified 2026-09-10 |
| D3 | **Min-to-borrow floor** (40 / 50 / none, §7.1) | **No hard floor** — score-0 wallets borrow at Starter terms | ✅ ratified 2026-09-10 |
| D4 | **Sponsor pool** — separate contract vs single LP | Single LP now; separate later if needed | ▢ open (not in 2026-09-10 batch) |
| D5 | **Anchor/fiat rails** (SEP-24/31) in scope? | **Deferred** — evaluate in Phase 4, not committed | ✅ ratified 2026-09-10 |
| D6 | **Learner grace period** value (per-loan) | **≈14 days/installment**, governance-tunable via `parameters-contract` | ✅ ratified 2026-09-10 |
| R1 | **Leaked `ghp_…` PAT** in multiple `.git/config` | **Rotate now** (owner-only), then purge | ⚠ action outstanding |
| R2 | **Orphan "Set B" deployment** (`GDL63O…`) origin unresolved | Keep DO-NOT-USE; document; close investigation | ▢ |
| R3 | **Local checkouts stale** vs live branches (API ~5 weeks behind) | Sync + reconcile before Phase 0 audit sign-off | ▢ |
| R4 | **Vercel target** — which surfaces (Web only? + Docs + landing?) | Web + landing + Docs to Vercel; API stays Render | ▢ |

---

## 14. Document reconciliation actions

| Doc | Action | Phase |
|---|---|---|
| `docs/PROJECT_CONTEXT.md` | Rewrite as derived vision (learner-BNPL, 6 contracts, current phase) or archive | 1 |
| `docs/ROADMAP.md`, per-repo `ROADMAP.md` | Supersede with `PROGRESSION.md` | 1 |
| `AI_CONTEXT.md` | Regenerate as pointer to this PRD | 1 |
| `context/project-overview.md` (×3) | Trim to per-repo pointer to this PRD | 1 |
| Contracts `README.md` | Add vouching (6 contracts); confirm IDs/deployer | 0/1 |
| API `README.md` | **Replace wrong-project (`# Supabase CLI`) README** | 0 |
| Web/Mintlify tier & threshold tables | Match §7.1 canonical | 1 |
| Web `introduction.md` | Fix "Express" → NestJS/Fastify | 1 |
| Mintlify `contracts/overview.mdx` | Add vouching + parameters | 1 |
| All READMEs | Truthful feature/stack + live links | 1/6 |

---

## 15. Glossary

- **BNPL** — Buy Now, Pay Later.
- **SAC** — Stellar Asset Contract (token rail used by contracts).
- **Reputation boost** — mentor-vouch-driven additive score component.
- **Guarantee** — collateral/guarantee amount held against a loan; consumed on default before loss socialization.
- **Loss socialization** — reducing LP share price to absorb an unrecovered default.
- **The gate** — the combination of CI-green + bot-approve + human review that a PR must pass to merge (and thus to be paid).
- **Tooling surface** — the reusable primitives (reputation read-API, SDK, indexer/events, dashboards) that make StepFi a system, not a single dApp.

---

*End of PRD v1.0 (draft for ratification). Next: `docs/PROGRESSION.md` (per-repo execution → PR backlog) and the tightened `stepfi-audit-bot/CODING_STANDARDS.md`.*
