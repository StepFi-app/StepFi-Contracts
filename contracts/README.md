# Contracts — Upgrade Flow

This document describes the on-chain upgrade flow supported by StepFi contracts.

Upgrade overview
- Contracts expose an `upgrade(env: Env, new_wasm_hash: BytesN<32>)` entrypoint. Only the configured admin may call this function.
- Upgrades preserve the contract address and instance storage; only the WASM binary is swapped.
- Each contract persists a numeric `VERSION` key in instance storage (default `1`). Successful upgrades increment this value.
- On successful upgrade an event `CONTRACTUPGRADED` is emitted with `(old_version, new_version, timestamp)` to aid off-chain indexing and monitoring.

Developer notes
- `upgrade()` performs an admin auth check immediately and then bumps `VERSION` before swapping WASM.
- Use `get_version(env)` to query the current numeric version.
- Instance storage is used for admin and version keys; persistent storage TTL rules do not apply to instance storage.

Testing
- Unit tests verify that non-admin callers are rejected and that admin callers succeed (version bump + event emitted).

CI
- Run `cargo build` and `cargo test` to validate the changes.

