# Reputation Contract

## Purpose

Manage on-chain reputation scores (0-100) for users in the StepFi BNPL system. This contract tracks user reputation based on repayment behavior and allows authorized updaters (like the CreditLine contract) to modify scores.

## Overview

The Reputation Contract provides a decentralized way to track and update user reputation scores. Reputation scores are used to determine creditworthiness and credit limits for BNPL transactions.

### Key Features

- **Score Management**: Track reputation scores between 0 and 100 for each user
- **Authorized Updaters**: Control which contracts/addresses can modify scores
- **Admin Control**: Centralized admin address for managing updaters
- **Event Emission**: Emit events for all score and permission changes
- **Authorization**: Uses Soroban's native authorization system with `require_auth()`

## Available Functions

### Version
- `get_version() -> Symbol` - Returns the contract version symbol (v1_0_0)

### Score Operations
- `get_score(user: Address) -> u32` - Get the reputation score for a user
- `increase_score(updater: Address, user: Address, amount: u32)` - Increase a user's score (requires updater auth)
- `decrease_score(updater: Address, user: Address, amount: u32)` - Decrease a user's score (requires updater auth)
- `set_score(updater: Address, user: Address, new_score: u32)` - Set a user's score to a specific value (requires updater auth)

### Admin Operations
- `initialize(admin: Address)` - Initialize the contract with an admin (requires admin auth, can only be called once)
- `set_admin(new_admin: Address)` - Transfer admin to a new address (requires current admin auth)
- `get_admin() -> Address` - Get the current admin address

### Updater Operations
- `set_updater(admin: Address, updater: Address, allowed: bool)` - Grant or revoke updater permissions (requires admin auth)
- `is_updater(addr: Address) -> bool` - Check if an address is an authorized updater

## Build Instructions

### Prerequisites

Install the `wasm32-unknown-unknown` target:

```bash
rustup target add wasm32-unknown-unknown
```

### Build

Build the contract for release:

```bash
cargo build -p reputation-contract --target wasm32-unknown-unknown --release
```

The compiled WebAssembly binary will be located at:
```
target/wasm32-unknown-unknown/release/reputation_contract.wasm
```

### Test

Run the test suite:

```bash
cargo test -p reputation-contract
```

## Implementation Status

✅ **Fully implemented and functional**

All functions are complete with:
- ✅ Contract structure and function signatures
- ✅ Type definitions and error enums
- ✅ Storage operations (Map-based)
- ✅ Access control with Soroban authorization
- ✅ Event emission
- ✅ Complete test suite
- ✅ Business logic implementation
- ✅ Validation logic

## Architecture

The contract is organized into modular components:

- `lib.rs` - Main contract implementation with authorization
- `types.rs` - Type definitions and constants
- `storage.rs` - Storage operations using Map
- `access.rs` - Access control validation
- `events.rs` - Event emission
- `errors.rs` - Error type definitions
- `tests.rs` - Complete test suite

## Authorization

This contract uses Soroban's native authorization system. Each protected function requires the caller to provide authorization via `require_auth()`:

- Admin functions require admin authorization
- Updater functions require updater authorization
- Initialization allows setting admin without prior auth

