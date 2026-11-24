# tomatui

A clean, offline-first Pomodoro TUI & agent-friendly CLI written in Rust, backed by SQLite (WAL mode).

## Features

- **Minimalist TUI**: Bold geometric half-block numerals, smooth 8× sub-cell progress bar, mouse + keyboard navigation (`1–4` or `T/P/H/S` for tabs, `,` for Settings, `Esc` to return), and 18 built-in 24-bit RGB themes.
- **Projects → Activities → Editable Sessions**: Organize work by Project and Activity, start immediately (even unassigned), and edit any live or historical session later to assign/change project, activity, status, or notes.
- **Flexible Plans**: Quick presets (`25m × 1`, `25m × 2`, `50m × 2`) or manual 3-number entry (`t` → focus mins, break mins default `5`, batches default `1`).
- **Quick-Launch CLI & Agent JSON Interface**: Launch batches directly from your shell (`tomatui backend-api --25 --2`) and let AI agents query or update sessions via structured `--json` output or direct SQLite reads.
- **Zero-Asset Audio & Alerts**: Pure in-memory synthesized PCM16 chimes + native desktop notifications.

## Quick Start

```bash
# Build and run the Opening View
cargo run --release

# Quick-launch 2 x 25m focus sessions on "backend-api" (5m default break)
cargo run --release -- backend-api --25 --2

# Install globally to ~/.cargo/bin/tomatui
cargo install --path .
```

## CLI & Agent Usage

```bash
# Quick-launch shorthand
tomatui backend-api --25 --2
tomatui backend-api --25 --2 --activity "API schema" --break 10
tomatui --50 --2

# Projects & Activities
tomatui projects list --json
tomatui projects add backend-api --json
tomatui activities add backend-api "API schema" --est 4 --json

# Sessions (inspect & edit post-hoc)
tomatui sessions list --unassigned --json
tomatui sessions edit <ID_PREFIX> --project backend-api --activity "API schema" --notes "Done" --json

# Analytics & Export
tomatui stats --range week --json
tomatui export --format csv
tomatui db-path
```

## Data Location

By default, data is stored in a single SQLite (WAL) database:
- **macOS**: `~/Library/Application Support/com.tomatui.tomatui/tomatui.db`
- **Linux**: `~/.local/share/tomatui/tomatui.db`

You can view or relocate the database anytime from `, Settings` → **Data Location (SQLite)**, or override it via `--db <PATH>` / `TOMATUI_DB`.

## License

MIT
