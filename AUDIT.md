# Project Audit

Audit performed against `main` on 2026-07-11.

This document tracks inconsistencies, risks, and behavior that may surprise an
experienced Rust developer. Resolve the numbered findings one at a time and
mark their checkboxes when the corresponding change and tests are complete.

## Findings

- [x] 1. **High: append and insert silently replace file metadata**

  `src/commands/mod.rs:56-70` writes through a new temporary file and renames
  it over the note. This can reset Unix permissions, ACLs, extended attributes,
  hard-link relationships, and symlink behavior. A group-readable note may
  become mode `0600` after appending. Windows replacement semantics may also
  differ. Preserve relevant permissions explicitly and define symlink
  behavior.

  Resolution: atomic writes now resolve symlinks to update their targets and
  copy the existing file permissions to the replacement. Regression tests
  cover Unix permission bits and symlink preservation. Atomic replacement
  still cannot preserve hard-link identity or arbitrary ACLs and extended
  attributes portably.

- [x] 2. **High: note creation has a race that can truncate another writer's file**

  `src/commands/daily.rs:85-90` and `src/commands/weekly.rs:104-109` check
  `exists()` and then call `std::fs::write`. Another process can create the file
  between those operations, after which it gets truncated. Use
  `OpenOptions::create_new(true)` and handle `AlreadyExists`.

  Resolution: daily and weekly note creation now uses a shared exclusive-create
  helper. `AlreadyExists` is treated as finding an existing note, so a
  concurrent creator's contents are not overwritten. Regression tests cover
  existing-file preservation and simultaneous creation with exactly one
  winner.

- [x] 3. **High: two release workflows compete for the same release**

  `.github/workflows/ci.yml:80-213` pushes a tag, builds artifacts, and creates
  a release. That tag also triggers `.github/workflows/release.yml:3-92`, which
  independently builds and creates the same release. This can cause duplicate
  uploads, partial releases, or nondeterministic failures. Keep one
  authoritative release pipeline.

  Resolution: `.github/workflows/ci.yml` is now the sole release pipeline and
  the redundant tag-triggered workflow has been removed. Cocogitto's implicit
  post-bump pushes were also removed so CI owns one explicit atomic push of the
  version commit and tag before building and publishing the release.

- [x] 4. **High: CI repeatedly compiles unpinned release tooling**

  `.github/workflows/ci.yml` installs Cocogitto from source in both the test and
  version-bump jobs, making tool compilation one of the slowest CI steps.
  `cargo install cocogitto` and `cargo install cargo-edit` are also unpinned, so
  new tool releases or MSRV changes can break CI without a repository change.
  Remove Cocogitto from jobs that do not require it, pin required tool versions
  with `--locked`, and cache only their installed executables using keys that
  include the runner platform and exact tool version.

  Resolution: the test job no longer installs Cocogitto. The version-bump job
  pins Cocogitto and `cargo-edit`, installs them with locked dependency graphs,
  and caches only the `cog` and `cargo-set-version` executables under a key that
  includes the runner platform and both exact versions.

- [x] 5. **High: malformed-config recovery never actually starts fresh**

  `src/commands/init.rs:61-81` backs up malformed TOML but leaves the malformed
  original in place. Rerunning `init` encounters the same error indefinitely.
  The displayed and actual backup timestamps are also generated separately and
  can disagree. Generate one backup path, back up once, and replace or remove
  the invalid source atomically.

  Resolution: confirmed recovery now creates a collision-safe backup containing
  the exact original bytes and permissions, syncs it, removes the malformed
  config, and continues the same wizard run with a fresh document. Partial
  backups are cleaned up on failure, successful backups are retained if source
  removal fails, and the behavior is covered by tests and README documentation.

- [x] 6. **High: user-controlled config structure can panic `init`**

  Preflight skips non-table top-level values at
  `src/commands/init.rs:126-129`, but section selection later exposes them and
  eventually uses `expect("section must be a table")` around
  `src/commands/init.rs:463-466`. Config such as `default = 1` can therefore
  panic rather than produce a normal diagnostic.

  Resolution: preflight now rejects scalar and array top-level entries with a
  contextual error before section selection, and section writes return an
  error instead of panicking if validation is bypassed. Standard and inline
  TOML tables are both supported and editable. Regression tests cover scalar,
  array, and inline-table entries.

- [x] 7. **Medium: release-tool cache action uses deprecated Node.js 20**

  CI run `29201795355` warns that `actions/cache@v4` targets deprecated
  Node.js 20 and is being forced to run on Node.js 24. Upgrade the release-tool
  cache step in `.github/workflows/ci.yml` to the latest supported major
  (`actions/cache@v6`, currently released as v6.1.0) and confirm the cache is
  restored without runner deprecation warnings.

  Resolution: the release-tool cache now uses `actions/cache@v6`, currently
  released as v6.1.0 with the Node.js 24 runtime. Its executable paths and
  versioned cache key are unchanged, preserving existing restore behavior.

- [ ] 8. **Medium: failed argument validation can still mutate the filesystem**

  Daily and weekly create directories and files before parsing all editor or
  format options or validating an insertion heading
  (`src/commands/daily.rs:83-123`, `src/commands/weekly.rs:98-155`). An
  unsuccessful invocation can leave new notes behind. Prefer Clap typed values
  and complete validation before mutation.

- [ ] 9. **Medium: editor launch failures return success**

  `src/handlers/mod.rs:18-44` catches spawn errors, prints a warning, and
  returns `()`. Automation receives exit code 0 even though the requested
  editor never opened. Return and propagate an `io::Result`.

- [ ] 10. **Medium: editor handling is unexpectedly limited**

  At `src/handlers/mod.rs:26-28`, a value such as `EDITOR="code --wait"` is
  treated as one executable name, `VISUAL` is ignored, and generic terminal
  editors are detached immediately. Windows uses `Command::new("start")` at
  `src/handlers/mod.rs:51-54`, but `start` is normally a `cmd.exe` built-in and
  will not execute this way.

- [ ] 11. **Medium: explicit profile typos silently select the default profile**

  At `src/helpers/config.rs:103-106`, an unknown `--config wrok` becomes an
  empty config merged over `[default]`. That can create a note in the wrong
  vault without warning. Explicitly requested missing profiles should be
  errors.

- [ ] 12. **Medium: config path behavior is not platform-native and has a dangerous fallback**

  `src/helpers/config.rs:134-137` hardcodes `~/.config`, ignoring
  `XDG_CONFIG_HOME` and Windows or macOS conventions. If no home directory is
  available, it silently uses the current directory. `expand_home` repeats
  that behavior at `src/helpers/config.rs:157-164`, potentially redirecting
  note creation into the repository.

- [ ] 13. **Medium: Markdown insertion is not sufficiently Markdown-aware**

  `src/helpers/markdown.rs:71-120` interprets heading-looking lines inside
  fenced code blocks as real headings. It also omits CommonMark details such as
  leading indentation and closing `#` characters. Content can be inserted into
  the wrong section.

- [ ] 14. **Medium: template paths can escape `notesFolder`**

  `src/helpers/template.rs:19-31` joins the configured template directly.
  Absolute paths discard the base, while `../` traverses outside it,
  contradicting the README's "relative to `notesFolder`" claim. Either
  document external templates or enforce containment.

- [ ] 15. **Medium: config accepts unknown fields silently**

  The config structs at `src/helpers/config.rs:23-37` do not use
  `#[serde(deny_unknown_fields)]`. Misspellings such as `notes_folder`,
  `noOpen`, or `templat` are ignored, which is particularly surprising for a
  CLI configuration file.

- [x] 16. **Medium: no platform or MSRV contract**

  `Cargo.toml:4` uses edition 2024 but has no `rust-version`, and no
  `rust-toolchain.toml` exists. CI uses floating stable toolchains.
  Contributors cannot determine the supported compiler, and a new stable
  release can unexpectedly alter CI or release behavior.

  Resolution: the package now declares Rust 1.85 as its MSRV, development and
  application builds are pinned to Rust 1.85.0, and CI verifies the project on
  that compiler. The version-bump job is separately pinned to Rust 1.92.0 as
  required by its existing `cargo-edit` release tool. The README documents the
  MSRV and the five supported release targets.

- [x] 17. **Low: documentation examples imply a library that does not exist**

  Rustdoc examples such as `src/handlers/mod.rs:10-17` import `take_note::...`,
  but this is a binary-only crate with private modules in `src/main.rs`. These
  are not meaningful public API examples, despite the README claiming doc
  tests are run.

  Resolution: library-style examples were removed from the binary crate's
  private modules, whose behavior remains covered by unit tests. The README now
  accurately describes `cargo test` as running unit tests, and documentation
  generation completes without implying a public library API.

- [x] 18. **Low: test dependencies are unused**

  `assert_cmd` and `predicates` are declared in `Cargo.toml:23-25`, but there
  are no integration tests. Command exit statuses, filesystem side effects,
  and headless output are precisely where these dependencies would be
  valuable.

  Resolution: CLI integration tests now use `assert_cmd` and `predicates` to
  cover help output, validation failure status and stderr without filesystem
  mutation, and successful headless JSON note creation with the reported path
  verified on disk.

## Testing Gaps

The project now has 60 unit tests and 3 CLI integration tests. Notable
omissions are:

- Broader CLI exit status and stderr behavior
- Concurrent note creation
- Permission and symlink preservation
- Malformed `init` recovery
- Unknown profiles and config fields
- Cross-platform editor launching
- Weekly partial batch failures
- Markdown fences and CommonMark heading variants

## Audit Verification

- `cargo fmt --all -- --check`: passed
- `cargo clippy --all-targets --all-features -- -D warnings`: passed
- `cargo test --all-features`: 49 passed
- `cargo tree --duplicates`: two `thiserror` major versions through
  `dialoguer`; harmless but mildly wasteful

The code is generally readable, contains no `unsafe`, and passes standard Rust
tooling. The largest risks are filesystem semantics and release automation,
rather than conventional Rust correctness or lint quality.
