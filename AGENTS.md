# Agent Guidelines

## Project Principles

- Use stable Rust and edition 2024.
- Make the smallest correct change. Avoid speculative abstractions,
  unrelated refactors, and compatibility layers without a concrete need.
- Preserve the established project structure and CLI behavior unless the task
  explicitly changes that behavior.
- Keep the crate free of `unsafe` Rust unless the user explicitly approves an
  exception.
- Prefer the standard library and existing dependencies. Ask before adding a
  new dependency.

## Correctness

- Account for Linux, macOS, and Windows, matching the advertised release
  targets. Gate platform-specific implementations and tests appropriately.
- Do not panic on user input, configuration, process failures, or filesystem
  failures. Return useful, contextual errors instead.
- Reserve `panic!`, `unwrap`, and `expect` in production code for invariants
  that cannot be violated by external input or runtime conditions.
- Add regression tests for bug fixes and user-visible behavior changes.
- Update the README, CLI help, and configuration documentation when their
  documented behavior changes.

## Required Checks

Before work is considered complete, run and pass:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## Audit Workflow

- Treat each numbered item in `AUDIT.md` as a separate unit of work.
- Complete one audit item per commit.
- Update the corresponding checkbox and record the resolution in `AUDIT.md`
  only after implementation and verification are complete.
- Ask for approval before creating each commit.

## Commits

- Use Conventional Commits with a lowercase type and concise imperative
  summary, for example: `fix: preserve note permissions`.
- Scopes are optional.
- Use `!` or a `BREAKING CHANGE:` footer for breaking changes.
- Keep each commit focused on one logical change and include its tests and
  documentation updates.
- Do not commit automatically. Present the verified changes and wait for
  explicit approval.
