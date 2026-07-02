# Contributing to StepFi-Contracts

This repo contains Soroban smart contracts
written in Rust. Nothing else belongs here.

## What belongs in this repo

- Rust source files inside contracts/
- Contract tests inside each contract's tests.rs
- Build scripts inside scripts/
- Context and documentation inside docs/ and context/

## What does NOT belong here

- TypeScript, JavaScript, React, or Vue files
- package.json, vite.config.ts, or any Node.js config
- Frontend components or pages
- HTML files

If you are building a web dashboard or UI feature,
open your PR against StepFi-Web instead.

## Before you start

1. Read context/architecture-context.md
2. Read context/code-standards.md
3. Make sure cargo build passes on your branch
4. Make sure cargo test passes on your branch

## Standards that are non-negotiable

- require_auth() must be the first line of every
  mutating contract function
- extend_ttl() must be called after every
  persistent storage write
- No .unwrap() or .expect() in user-facing paths
- New tests required for every new function
- All existing tests must still pass

## PR process

1. Fork the repo
2. Create a branch: feat/your-feature or fix/your-fix
3. Write code and tests
4. Run cargo build and cargo test locally
5. Open a PR referencing the issue number
6. Fill out the PR template completely
7. Wait for CI to pass before requesting review

PRs that fail CI will be closed without review.
PRs that do not reference an issue will be closed.
PRs that add non-Rust files will be closed immediately.
