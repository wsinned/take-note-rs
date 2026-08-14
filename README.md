# take-note

A Rust CLI for creating and managing weekly and daily markdown notes. Drop-in replacement for the TypeScript/Deno version.

## Why this exists

I maintain weekly log files as part of my personal knowledge management system. Every Monday I need a new note with the correct filename (e.g. `2026-06-08-Weekly-log.md`) and a date header. Doing this manually is tedious and error-prone. This tool automates it — and adds daily notes, batch creation, config profiles, and headless mode for scripting.

## Installation

### Recommended: eget (Linux/macOS)

[eget](https://github.com/zyedidia/eget) installs pre-built binaries directly from GitHub releases.

```bash
# Install take-note (Linux x86_64)
eget wsinned/take-note-rs --asset take-note_linux_x86_64 --to ~/.local/bin/take-note

# macOS Apple Silicon
eget wsinned/take-note-rs --asset take-note_darwin_aarch64 --to ~/.local/bin/take-note

# macOS Intel
eget wsinned/take-note-rs --asset take-note_darwin_x86_64 --to ~/.local/bin/take-note
```

To upgrade later, run the same command again.

> Install eget itself: see [zyedidia/eget](https://github.com/zyedidia/eget)

### Nix flakes (Linux/macOS)

The repository provides a Nix flake for Linux and macOS systems. It builds the
Rust crate from `Cargo.lock`.

```bash
# Run from the current checkout
nix run . -- --help

# Run from GitHub without installing
nix run github:wsinned/take-note-rs -- --help

# Build from the current checkout
nix build
./result/bin/take-note --help

# Build from GitHub
nix build github:wsinned/take-note-rs

# Install into your Nix profile
nix profile install github:wsinned/take-note-rs
```

### Manual download

Download the appropriate binary for your platform from the [releases page](https://github.com/wsinned/take-note-rs/releases):

| Platform | Asset |
|----------|-------|
| Linux x86_64 | `take-note_linux_x86_64` |
| Linux arm64 (e.g. Raspberry Pi) | `take-note_linux_aarch64` |
| macOS Intel | `take-note_darwin_x86_64` |
| macOS Apple Silicon | `take-note_darwin_aarch64` |
| Windows x86_64 | `take-note_windows_x86_64.exe` |

Make the binary executable and move it to your PATH:
```bash
chmod +x take-note_linux_x86_64
mv take-note_linux_x86_64 ~/.local/bin/take-note
```

### From source (requires Rust)

Source builds require Rust 1.88.0 or newer. The repository pins Rust 1.88.0
for development and application builds to enforce this minimum supported Rust
version (MSRV).

```bash
git clone https://github.com/wsinned/take-note-rs.git
cd take-note-rs
cargo build --release
# Binary at target/release/take-note
```

Pre-built releases support Linux x86_64 and arm64, macOS Intel and Apple
Silicon, and Windows x86_64, matching the assets listed above.

---

## Configuration

Reads from `~/.config/take-note/config.toml` if it exists. CLI flags always override config values.

```toml
[default]
notesFolder = "~/Documents/Notes/Weekly"
editor = "obsidian"           # obsidian | vscode | generic
template = "Templates/weekly-template.md"
batch = 1
```

### Named configs

Use multiple configs for different contexts:

```toml
[default]
notesFolder = "~/Documents/Personal/Notes"
editor = "obsidian"

[weekly]
template = "Templates/Weekly.md"
batch = 3

[daily]
template = "Templates/Daily.md"

[work]
notesFolder = "~/Documents/Work/Notes"
editor = "vscode"
batch = 2
```

Select with `--config work`.

Without `--config`, commands use their own section (`[weekly]` or `[daily]`) if it exists, then fall back to `[default]`. This lets you set command-specific defaults (e.g. different templates for weekly vs daily) without specifying `--config` every time.

> `default` is reserved as the base config and cannot be passed to `--config`. Use it only as a fallback layer in your config file.

---

## Usage

```
take-note weekly [OPTIONS] <WHEN> [APPEND]
take-note daily [OPTIONS] <WHEN> [APPEND]
take-note init
take-note --help
take-note --version
```

### Interactive setup

Run `take-note init` to create or update the configuration interactively. If
the existing config contains malformed TOML, the wizard displays the parse
error and asks whether to back it up and start fresh. A confirmed recovery:

- writes the exact original contents to
  `config.toml.YYYYMMDD-HHMMSS`, adding `.1`, `.2`, and so on rather than
  overwriting an existing backup;
- preserves the original file permissions and syncs the completed backup
  before removing the malformed config; and
- continues setup immediately with an empty configuration.

If backup creation, writing, permission preservation, or syncing fails, the
partial backup is removed and the original config is left untouched. If the
backup succeeds but removing the original fails, both files are retained and
the command reports their paths instead of continuing. Cancelling later setup
prompts leaves the completed backup in place and no active config file.

### Weekly notes

```bash
take-note weekly thisWeek
take-note weekly thisWeek --config work
take-note weekly thisWeek --editor vscode
take-note weekly thisWeek "- Shipped append mode"
take-note weekly thisWeek --insert "Weekly Log/Monday" "- Shipped insert mode"
```

### `--when` options (weekly)

| Value | Description |
|-------|-------------|
| `lastWeek` | Monday of last week |
| `thisWeek` | Monday of current week |
| `nextWeek` | Monday of next week |

### Daily notes

```bash
take-note daily today
take-note daily today "- Follow up on invoices"
take-note daily today --insert "Daily Log/Notes" "- Follow up on invoices"
```

### `--when` options (daily)

| Value | Description |
|-------|-------------|
| `yesterday` | Yesterday's date |
| `today` | Today's date |
| `tomorrow` | Tomorrow's date |

### Templates

Supply a template path relative to `notesFolder`. The placeholder `{{date}}` is replaced with the note date formatted as `Monday 28 July 2025`.

```bash
take-note weekly thisWeek --template Templates/weekly-template.md
```

### Headless / automation mode

```bash
# Text output (default)
take-note weekly thisWeek --no-open

# JSON output
take-note weekly thisWeek --no-open --format json

# Silent (exit code only)
take-note weekly thisWeek --no-open --format silent
```

Append and insert modes are always headless. They create the resolved note if missing and print only the target file path on success. Use `--insert HEADING/SUBHEADING` to place the supplied blob at the end of a matching markdown heading path instead of appending to the file.

---

## Development

After cloning, configure git to use the repo's hooks (runs fmt, clippy, and tests before each commit):

```bash
git config core.hooksPath .githooks
```

```bash
cargo test        # Run all unit tests
cargo build       # Debug build
cargo build --release  # Optimized binary
```

### Project structure

```
src/
  main.rs              # Entry point
  commands/
    weekly.rs          # Weekly notes command
    daily.rs           # Daily notes command
    init.rs            # Interactive setup wizard
  helpers/
    config.rs          # TOML config loading
    date.rs            # Date calculations
    output.rs          # Format output for --noOpen
    markdown.rs        # Markdown heading helpers
    template.rs        # Template loading & variable replacement
  handlers/
    mod.rs             # Editor integrations (obsidian, vscode, generic)
  options/
    editor.rs          # Editor option parsing
```

---

## Roadmap

- [x] Weekly notes
- [x] Daily notes
- [x] Headless mode (`--no-open`, `--format`)
- [x] Config file support
- [x] Named configs
- [x] Batch creation (`--batch N`)
- [x] `take-note init` setup wizard
- [x] Insert mode with heading locators
- [x] Append mode (`take-note daily today "text"`)
