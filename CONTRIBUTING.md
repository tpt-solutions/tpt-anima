# Contributing to TPT Anima

Thanks for your interest in contributing to TPT Anima! This document explains how
to get set up and what we expect from contributions.

## Code of Conduct

By participating you agree to abide by our
[Code of Conduct](CODE_OF_CONDUCT.md). Be respectful, constructive, and assume
good intent.

## Getting started

1. Install a recent Rust toolchain (1.80+):
   ```sh
   rustup toolchain install stable
   ```
2. Build the workspace:
   ```sh
   cargo build --workspace
   ```
3. Run the test suite:
   ```sh
   cargo test --workspace
   ```

## Before opening a pull request

We run CI on every PR. To avoid round-trips, please run locally first:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

PRs that do not pass `cargo fmt --check` or clippy with `-D warnings` will not be
merged.

## Commit and PR style

- Keep commits focused; one logical change per commit.
- Write clear commit messages describing *why*, not just *what*.
- For substantial changes, open an issue first to discuss the design.
- Add or update tests for any behavior you change.

## Licensing

TPT Anima is dual-licensed under MIT or Apache-2.0. By contributing you agree
that your contributions are submitted under the same dual license.

## Reporting security issues

Do **not** open public issues for security vulnerabilities. See the security
policy (or contact the maintainers privately) so we can coordinate a fix and
disclosure.
