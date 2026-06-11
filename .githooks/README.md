# Git Hooks

This repo includes custom hooks in `.githooks/`.

## Setup

Git doesn't use `.githooks/` by default. Configure git to use this directory:

```bash
git config core.hooksPath .githooks
```

Or set globally for all repos (if you keep the same convention):

```bash
git config --global core.hooksPath .githooks
```

## Hooks

### pre-commit

Runs before each commit:
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test --all-features`

### pre-push

Runs before each push:
- Same as pre-commit, plus `cargo build --release`

## Bypassing

In emergencies, use `--no-verify`:

```bash
git commit --no-verify -m "WIP"
git push --no-verify
```
