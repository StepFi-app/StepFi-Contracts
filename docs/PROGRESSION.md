# StepFi — Progression Plan

> **Companion to [`PRD.md`](./PRD.md).** The PRD says *what StepFi is and must become*; this document says *how we get there, repo by repo, as a strictly-ordered backlog of IndigoPay-grade epics that seed the PRs*.
> **Status:** Draft v1.0 · **Date:** 2026-09-10 · **Owner:** `eitighis`
> Every epic here becomes a GitHub issue in the IndigoPay #1098 shape; every PR against it must meet the #1211 shape and pass **the gate** (CI-green-for-real + bot-approve + human review). This is the document "everything going on" is measured against.

---

## 1. How to use this document

- **Phases are gates, not suggestions.** **Phase 0 blocks all feature work.** No epic from Phase 1+ merges while any Phase 0 CRITICAL is open in that repo.
- Each epic has an ID (`<REPO>-E<n>`), a size (PR estimate), a phase, and a one-line objective. The full IndigoPay body (Objective / Problem+evidence / Scope / Implementation / Acceptance / Testing) is authored into the GitHub issue when the epic is opened; §8 is a fully-worked exemplar.
- **Evidence is mandatory.** Every "Problem" cites `file:line`. The citations below come from the current audits; a contributor re-verifies against the (synced) branch before opening the issue.
- **PR budgeting** (§7) shows how 200–400 quality PRs/campaign fall out of this backlog *without padding* — each epic decomposes into several small, reviewable PRs.

## 2. Global conventions (binding)

### 2.1 Epic shape (GitHub issue)
`Summary` (what + why-together + the unifying property) → `Labels` → per workstream: `Objective` · `Problem` (with `file:line`) · `Scope` (exact files) · `Implementation` (numbered, with config knobs + defaults) · `Acceptance Criteria` (mock/concrete scenarios) · `Testing` (unit/integration/load/chaos).

### 2.2 PR shape (matches IndigoPay #1211)
Exec summary (the unifying property) → Background (choke-point) → **file-by-file audit of the change vs the issue** (asks-for / status / action) → Design (ASCII flow + **typed error model** table) → per-file changes → failure-mode analysis → **Verification (exact CI commands + results + test matrix)** → metrics. `Closes #N`.

### 2.3 Definition of done (every PR)
Builds for real · full tests pass in bounded time · ≥80% coverage on touched code · lint+typecheck clean · no new secrets (secret-scan green) · regression test for any bug/security fix (fails pre-fix) · docs/README updated · **the gate** passed.

### 2.4 Labels
`GrantFox OSS`, campaign label, `area:<repo/domain>`, `type:{security|feature|fix|docs|test|chore|perf|ci}`, `priority:{critical|high|medium|low}`, `phase:<n>`, `epic:<id>`.

### 2.5 Size legend
`S` = 1–2 PRs · `M` = 3–5 PRs · `L` = 6–10 PRs · `XL` = 11+ PRs.

---

## 3. Phase 0 — Stop the bleeding (CRITICAL; blocks all feature work)

> These are the lapses that make CI lie and features fake. Until a repo's Phase 0 is green, nothing else merges there. Every item ships with a regression test that fails on today's code.

### 3.1 Contracts — Phase 0

| Epic | Objective | Evidence | Size |
|---|---|---|---|
| `CON-E0.1` | Fix the broken workspace build | `no method named 'receive_guarantee'`, `creditline/src/lib.rs:543` (stale `contractimport!` wasm); builds only under CI's dep-first order | M |
| `CON-E0.2` | Make CI gate the **whole** workspace | CI runs 1 of 6 suites (~124/321 tests); build actually broken behind green | M |
| `CON-E0.3` | Reconcile deployment truth | `.env.contracts` points at orphan **Set B** (`GDL63O…`) not canonical **Set A** (`GCOYDYSE…H7BF`); reconcile every client + `.env` | M |
| `CON-E0.4` | Deploy `vouching-contract` + wire it | 6th contract implemented (Issue #4) but undeployed, absent from overviews | M |

### 3.2 API — Phase 0

| Epic | Objective | Evidence | Size |
|---|---|---|---|
| `API-E0.1` | Replace wrong-project README | `README.md` opens `# Supabase CLI`, ~90% Supabase boilerplate | S |
| `API-E0.2` | Fix nonce TOCTOU (issue #116) | check-then-act consume, `auth.service.ts:96–141`; concurrent `/auth/verify` double-mints | M |
| `API-E0.3` | Make CI gate for real + fix hanging tests | suite times out >120s, open-handle leaks; unmocked I/O at module init | L |
| `API-E0.4` | Lock CORS + apply helmet | `main.ts:53–58` `*`+credentials, `CORS_ORIGINS` never read; `helmet` in deps, 0 usages | S |

### 3.3 App — Phase 0

| Epic | Objective | Evidence | Size |
|---|---|---|---|
| `APP-E0.1` | Replace mock auth with real auth | `register.tsx:68` `setTokens('mock-access-token','mock-refresh-token')` | M |
| `APP-E0.2` | Fix mainnet hardcode → testnet | `wallet.service.ts:10` `WC_CHAIN='stellar:pubnet:'` + pubnet passphrase | S |
| `APP-E0.3` | Fix Freighter v6 break + revive tx-signing core | `getPublicKey` removed (`wallet.service.ts:1`); dead `transaction-signer.service.ts`, `hooks/useTransaction.ts` | M |
| `APP-E0.4` | Fix 17 TS errors + make CI typecheck/test | CI runs only `expo export --platform web`, hiding all of the above | M |

### 3.4 Web — Phase 0

| Epic | Objective | Evidence | Size |
|---|---|---|---|
| `WEB-E0.1` | Replace **faked mentor vouch** with a real on-chain flow | `Vouch.tsx:159` fabricates random XDR, signs it; `:167,171` submits `signedTxXdr` mislabeled as `txHash` | M |
| `WEB-E0.2` | Honor env config | `config.ts:1–4` hardcodes API/network, ignores `VITE_*` | S |
| `WEB-E0.3` | Make CI build + test + typecheck (not PR-only no-op) | PR-only CI, no deploy, no real gate | S |

### 3.5 Org-wide — Phase 0

| Epic | Objective | Size |
|---|---|---|
| `ORG-E0.1` | **Rotate the leaked `ghp_…` PAT** (owner action) + purge from every `.git/config` + move to credential helper + org-wide secret scan in CI | M |

---

## 4. Phases 1–3 — Truthful foundation → flagship E2E → gas & quality

### 4.1 Phase 1 — Truthful foundation

| Epic | Repo | Objective | Size |
|---|---|---|---|
| `ORG-E1.1` | all | Ratify PRD; reconcile all docs per PRD §14; truthful READMEs everywhere | L |
| `ORG-E1.2` | audit-bot | Tighten bot scrutiny to enforce **PR structure + code quality**; extend to all repos; wire per-repo standards (see separate `CODING_STANDARDS.md` deliverable) | L |
| `ORG-E1.3` | all | Scaffold the **typed error-code catalog** (namespaces, registry, generated reference) | M |
| `CON-E1.1` | Contracts | Enforce invariants: move inline storage out of `lib.rs` (`creditline:59`, `reputation:190`); replace raw `env.invoke_contract` with generated clients (`creditline:270,289,566,1003,1015`; `vouching:134,154`) | L |
| `CON-E1.2` | Contracts | Add `get_version`/`upgrade`/`set_admin` to `vouching`; multisig+timelock upgrade path across contracts | M |
| `CON-E1.3` | Contracts | Split `creditline/src/lib.rs` (1,099 LOC > 800) by domain | M |
| `API-E1.1` | API | Consolidate the triplicated blockchain client layer to one canonical tree | L |
| `API-E1.2` | API | Global exception filter + `{success,data,error,meta}` envelope (fills empty `common/filters/`) | M |

### 4.2 Phase 2 — Flagship end-to-end on testnet

| Epic | Repo | Objective | Size |
|---|---|---|---|
| `ORG-E2.1` | App+API+Contracts | Learner E2E: register → reputation → apply → sign → confirm | L |
| `ORG-E2.2` | App+API+Contracts | Sponsor E2E: deposit → track position → withdraw | L |
| `ORG-E2.3` | App+Web+Contracts | Mentor E2E: verify → vouch → limit rises (requires `CON-E0.4`) | L |
| `CON-E2.1` | Contracts | Migrate reputation storage off single instance `Map` to persistent per-key (**exemplar §8**) | M |
| `API-E2.1` | API | Auth freshness: re-read role, block/revocation on every request (`jwt.strategy.ts:40–51`) | M |
| `API-E2.2` | API | SEP-1 completeness: add `parameters` contract + `NETWORK_PASSPHRASE`/`SIGNING_KEY`/`VERSION` to `stellar.toml` | S |

### 4.3 Phase 3 — Gas & quality

| Epic | Repo | Objective | Size |
|---|---|---|---|
| `CON-E3.1` | Contracts | **Resource-fee benchmark harness** + `gas-benchmarks.md` per contract + CI report artifact + regression threshold | L |
| `CON-E3.2` | Contracts | Optimize hot paths toward "ultra-low" fee within invariants; document before/after | L |
| `ORG-E3.1` | all | Drive error-code catalog toward ~250 documented codes (natural output of expansion, not padding) | L |
| `ORG-E3.2` | all | Coverage floors to 80%+ enforced in CI; fill gaps | XL |

---

## 5. Phases 4–5 — Payment rails & the tooling surface *(PRD decision (b) ratified 2026-09-10; D5 ratified — anchors evaluated in Phase 4, not yet committed)*

### 5.1 Phase 4 — Multiple Stellar payment systems

| Epic | Repo | Objective | Size |
|---|---|---|---|
| `PAY-E4.1` | API+App+Web | **SEP-10 Web Authentication** — replace bespoke nonce/JWT with standard challenge/response | L |
| `PAY-E4.2` | Contracts+API | **Classic payment** rail for repayment/disbursement (XLM/asset) | M |
| `PAY-E4.3` | API+App | **Path payments** — repay in any asset, settle in the loan asset | L |
| `PAY-E4.4` | API | Harden **recurring repayment** keeper into a first-class rail | M |
| `PAY-E4.5` | API+Docs | **Evaluate SEP-24/SEP-31 anchors** for fiat on/off-ramp *(conditional on D5)* | L |

### 5.2 Phase 5 — Tooling surface (makes it a system, not a dApp)

| Epic | Repo | Objective | Size |
|---|---|---|---|
| `TOOL-E5.1` | API+Contracts | Public **reputation read-API** (query any wallet's StepFi reputation) + docs | M |
| `TOOL-E5.2` | new SDK repo | **Integration SDK** (TS) wrapping reputation/credit/events for third parties | L |
| `TOOL-E5.3` | API | Public **event/indexer access** (webhooks or stream) with signed delivery | L |
| `TOOL-E5.4` | Web | Protocol **dashboards** (pool health, loan book, reputation distribution) | L |
| `TOOL-E5.5` | new | **Reference third-party integration** proving the tooling surface end-to-end | M |

## 6. Phases 6–7 — Deploy, submit, mainnet

| Epic | Repo | Objective | Size |
|---|---|---|---|
| `DEP-E6.1` | Web+landing+Docs | **Vercel migration**: deploy, new secret, update links/READMEs, **retire old deployments** (Netlify / stale `*.vercel.app`) | L |
| `DEP-E6.2` | Web+landing | **Vercel deploy in CI/CD** (preview per PR, prod on merge) | M |
| `DEP-E6.3` | all | **Grantfox/Drips submission** with every link (GitHub, Vercel, Docs, API, live contract IDs) | S |
| `MAIN-E7.1` | Contracts | Third-party **security audit** → mainnet deploy prep | XL |
| `MAIN-E7.2` | App | Mobile store release + first learner cohort | L |

---

## 7. PR budgeting — how 200–400 quality PRs/campaign fall out (no padding)

Each epic decomposes into small, reviewable PRs (one workstream / one file-cluster / one test-layer per PR). Rough envelope per campaign:

| Bucket | Epics | Est. PRs |
|---|---|---|
| Phase 0 (stop-the-bleeding) | 15 | 35–55 |
| Phase 1 (foundation) | 8 | 40–70 |
| Phase 2 (flagship E2E) | 6 | 35–60 |
| Phase 3 (gas & quality) | 4 | 40–80 |
| Phase 4 (payment rails) | 5 | 35–65 |
| Phase 5 (tooling surface) | 5 | 35–70 |
| Phase 6 (deploy/submit) | 3 | 10–20 |
| **Total available** | **46** | **~230–420** |

The point isn't to hit a number — it's that a **6-repo tooling system with real payment rails, gas work, an error taxonomy, and a tooling SDK** *legitimately* contains 200–400 well-scoped units of work, whereas a single thin dApp does not. Volume is the by-product of scope + decomposition, gated on quality.

---

## 8. Worked exemplar epic (the shape every issue must match)

> This is `CON-E2.1` authored in full IndigoPay-#1098 form. New epics are cloned from this shape.

### EPIC: Reputation storage migration — persistent per-key scores with TTL, bounded growth, and a typed error model

**Summary.** The reputation contract is the credit engine's source of truth and a future public tooling surface (any dApp may query a wallet's score). Today all scores live in a **single instance-storage `Map<Address,u32>`**, which (a) grows unboundedly in one instance entry, (b) violates the "no unbounded collection in persistent/instance storage" standard, (c) shares one hot key across all writers, and (d) is not TTL-managed per user. This epic migrates scores to **persistent per-wallet keys** with TTL extension after every write, bounds growth, and formalizes the error model — the unifying property: *every score read/write touches exactly one bounded, TTL-managed key, and every failure is a typed, documented error.*

**Labels:** `GrantFox OSS`, `<campaign>`, `area: contracts`, `type: refactor`, `type: security`, `priority: critical`, `phase: 2`, `epic: CON-E2.1`.

**Objective.** Move reputation scores from `Map<Address,u32>` in instance storage to `DataKey::Score(Address)` in persistent storage with `extend_ttl` after every write; keep the public API stable; add before/after tests proving no data-shape regression for reads.

**Problem (evidence).** `reputation/src/storage.rs:8` defines the map key; `:26–46` read-modify-write the whole map on every score change. Instance storage is not archival-safe per-user and the map is an unbounded collection — both violate `code-standards.md` ("Storage growth must be bounded") and `architecture-context.md` invariant (persistent for per-user records + TTL). No TTL extension exists for scores.

**Scope.**
- `reputation/src/storage.rs` — new `DataKey::Score(Address)`, per-key get/set with `extend_ttl`, migration reader.
- `reputation/src/lib.rs` — remove inline storage (`:190`), delegate to `storage.rs`.
- `reputation/src/errors.rs` — typed errors (below).
- `reputation/src/tests.rs` — migration + TTL + bounds tests.

**Implementation.**
1. Add `DataKey::Score(Address)`; `read_score(env,&addr) -> Option<u32>` (persistent); `write_score(env,&addr,score)` then `extend_ttl(PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO)`.
2. One-time lazy migration: on read, if legacy map holds the addr and the per-key is empty, backfill the per-key and continue (documented, removable next upgrade).
3. Remove the `Map<Address,u32>` write path; keep a read-only legacy accessor for migration only.
4. Delegate all `lib.rs` storage calls to `storage.rs` (invariant 1).
5. Emit `SCORECHGD` unchanged.

**Acceptance criteria.**
- Set score for A and B → each stored under its own `DataKey::Score` → `extend_ttl` called for each (assert via TTL probe).
- Legacy map entry present, per-key absent → first read backfills per-key → subsequent reads hit per-key only.
- `lib.rs` contains zero `env.storage()` calls (grep gate).
- Unknown/absent score reads return `Option::None` (never panic); typed errors used on writes.
- No change to `get_score` public signature or `SCORECHGD` event.

**Typed error model.**

| `.code` | Meaning | Recoverable |
|---|---|---|
| `NotInitialized` | Contract used before `initialize()` | No |
| `Unauthorized` | Non-updater attempted a score write | No |
| `InvalidScore` | Score outside 0–100 | No |
| `ScoreOverflow` | Checked-add boost overflow | No |

**Testing.**
- Unit: per-key read/write, TTL extension called, bounds (0/100/overflow), migration backfill, auth gating.
- Integration: creditline reads migrated scores unchanged; boost/remove via vouching path.
- Regression: a test asserting the old single-map write path is gone (fails on today's code).

---

## 9. Tracking

- Live status is maintained in each repo's `context/progress-tracker.md` (kept honest — reflects deployed/tested state, not intended state) and rolled up here per phase.
- An epic is "done" only when its issue is closed by a merged PR that passed **the gate** and its acceptance criteria are demonstrably met.

*End of Progression Plan v1.0 (draft). Next: tightened `stepfi-audit-bot/CODING_STANDARDS.md` + epic/PR templates that mechanically enforce §2.*
