## Summary

Closes #[issue number]

Briefly describe what this PR does in 2-3 sentences.

## This repo is for Soroban smart contracts only

Before submitting, confirm your changes belong here:

- [ ] My changes are inside the `contracts/` directory
- [ ] I have NOT added any TypeScript, React, or
      frontend files
- [ ] I have NOT added package.json, vite.config.ts,
      index.html, or any Node.js files
- [ ] My changes are written in Rust

If your PR touches anything outside `contracts/`
or `scripts/`, it does not belong in this repo.
Close this PR and open it in the correct repo.

## Type of change

- [ ] Bug fix
- [ ] New contract function
- [ ] Test coverage
- [ ] Storage or type changes
- [ ] Upgrade/migration utility

## Testing

- [ ] cargo build passes with zero errors
- [ ] cargo test passes — all existing tests still pass
- [ ] New tests written for every new function
- [ ] require_auth() is first line of every
      mutating function I added or changed
- [ ] extend_ttl() called after every
      persistent storage write I added or changed
- [ ] No .unwrap() or .expect() in user-facing paths

## Context files reviewed

- [ ] context/architecture-context.md
- [ ] context/code-standards.md
- [ ] context/progress-tracker.md updated

## Mandatory before requesting review

Running these must all exit 0:
cargo build
cargo test -p [contract-name]

If either fails, fix it before opening this PR.
PRs with failing checks will be closed without review.
