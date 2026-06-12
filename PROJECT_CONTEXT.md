# PROJECT_CONTEXT.md — take-note-rs

## Project
Rust rewrite of take-note-cli (migrated from Deno/TypeScript).

## Current State
- **Release:** v2.0.0 (2026-06-12)
- **Main branch:** `main`
- **Last merged:** `feature/command-named-config-defaults` — command-specific config sections for weekly/daily defaults
- **Remote:** https://github.com/wsinned/take-note-rs.git

## Architecture
- Rust CLI using `clap` for argument parsing
- Config file with command-specific sections (`[weekly]`, `[daily]`)
- Template placeholders: `{{date}}` (replaced `HEADER_DATE`)

## CI/CD
- GitHub Actions with Node 24 compatible versions
- Clippy + formatting checks via git hooks

## Next
- Test build pipeline and platform-specific builds
