# Implement On-Chain Vouch Expiry & Baseline-Clamped Boost Removal

closes #87

## Summary

Vouches in the `vouching-contract` previously had **no on-chain expiry and no
expiry enforcement**: once a mentor vouched for a learner, the reputation boost
it minted lived forever, even if the real-world relationship ended. This PR
adds a permissionless `expire_vouch` entrypoint that enforces a 30-day TTL and
removes the boost, mirroring the creditline contract's existing
`apply_late_fees` pattern.

It also hardens boost accounting so revocation/expiry can never push a learner
**below their pre-vouch reputation baseline**, including when a mid-life
boost-config change or external penalty has already reduced their score.

> **Note:** the `vouching-contract` is `PENDING_DEPLOYMENT`, so adding the
> `baseline` field to `VouchRecord` and the new `VouchNotExpired` error variant
> is safe and will not break already-deployed storage.

## Problem

1. **No expiry** — vouches were stored as active indefinitely; nothing
   deactivated them and nothing enforced a lifetime. A stale mentor–learner
   relationship kept inflating the learner's reputation permanently.
2. **Mismatched boost math** — `vouch()` minted using the *current* config
   boost, but `revoke_vouch()` subtracted the historical `record.boost_amount`.
   If the admin changed the global vouch boost while a vouch was live, removal
   removed the wrong amount.
3. **Below-baseline removal** — `remove_boost` subtracted the raw boost amount
   without checking the learner's current score. Combined with penalties or a
   boost-config change, this could drive the score below what the learner had
   *before* the vouch, effectively punishing them for a relationship they no
   longer benefit from.

## Changes

### `types.rs`
- Added `pub const VOUCH_DURATION: u64 = 2_592_000;` (30 days, seconds).
- Added `baseline: u32` to `VouchRecord` (learner reputation score immediately
  before this vouch's boost was applied).

### `errors.rs`
- Added `VouchNotExpired = 13`.

### `events.rs`
- Added `emit_vouch_expired` → `VOUCHEXPIRED` event.

### `lib.rs`
- Re-exported `VOUCH_DURATION` from `types`.
- `vouch` now captures `baseline = get_reputation_score(learner)` before adding
  the boost.
- Added permissionless `expire_vouch(env, mentor, learner)`:
  - No `require_auth` — anyone may trigger it (same trust model as
    `apply_late_fees`).
  - Validates `record.ts + VOUCH_DURATION < now`, else panics `VouchNotExpired`.
  - Idempotent: re-expiring an already-inactive record is a no-op.
  - Deactivates the record and removes the (clamped) boost, emitting
    `VOUCHEXPIRED`.
- `get_vouches` now returns **expired** records with `active = false` regardless
  of the stored flag, so off-chain readers never see a stale active boost
  without having to call `expire_vouch` first.
- `revoke_vouch` now uses `remove_reputation_boost_clamped` instead of the raw
  `remove_boost`.
- Added helpers:
  - `get_reputation_score` — cross-contract `get_score` read.
  - `remove_reputation_boost_clamped` — removes only
    `min(boost_amount, max(0, current_score - baseline))`, guaranteeing the
    learner can never drop below their pre-vouch baseline.

### `tests.rs`
- Added `decrease_score` to `MockReputationContract` to simulate external
  penalties.
- 6 new tests:
  - `test_expire_vouch_removes_boost_permissionless`
  - `test_expire_vouch_before_ttl_fails` (panics `VouchNotExpired`)
  - `test_expire_vouch_idempotent`
  - `test_revoke_after_expiry_fails` (panics `VouchNotActive`)
  - `test_get_vouches_marks_expired_inactive_without_expire`
  - `test_boost_removal_clamped_to_baseline` (pre-existing reputation + penalty
    scenario)

## Test Plan

```sh
cargo test -p vouching-contract
```

Result: **24 passed, 0 failed** (18 pre-existing + 6 new).

## Risk / Migration

- `VouchRecord` layout changed (`baseline` field added). Safe because the
  contract is not yet deployed (`PENDING_DEPLOYMENT`).
- New error code `13` appended; existing codes unchanged.
- `repuation-contract` ABI unchanged (only a new `decrease_score` on the mock
  for tests; the real reputation contract already exposes `decrease_score`).
