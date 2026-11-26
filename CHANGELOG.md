# Changelog

All notable changes to `tomatui` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-11-26

### Fixed
- **Modal text clipping** across `ContextPicker` (`p`), `Help` (`?`), `Set Plan` (`t`), and `Data Location` modals so all hint lines and controls fit cleanly with right-side breathing room.
- **Stats `By Project` bar width** now adapts dynamically to the pane width so bars and minute totals do not clip on narrower terminals.

## [0.2.0] - 2026-11-26

### Added
- **Spaces (`Space → Project → Activity`)** as an optional, fluid grouping layer above Projects (e.g. `School`, `Work`, `Client`, `Personal`):
  - Backwards-compatible SQLite schema migrations (`v2` and `v3`) adding `projects.space` and a `spaces` table so empty spaces persist cleanly.
  - **3-Column Projects Tab (`Spaces │ Projects │ Activities`)**: add (`a`), rename/edit (`e`), filter, and remove (`d`) spaces alongside projects and activities.
  - **Context Picker (`p`) & Timer**: assign or change Space, Project, and Activity together, with `Space · Project / Activity` shown on the Timer screen.
  - **History & Stats Space Filtering**: dedicated `Space` column in History, clickable `Space: [All] ...` pill bars in both History and Stats controlled via `←`/`→` (`h`/`l`), and `space` column in CSV exports.
- **Bulk Import & Agent Discovery CLI**:
  - `tomatui import [FILE|-] [--dry-run] [--json]` to idempotently bulk-create spaces, projects, and activities in a single SQLite transaction.
  - `tomatui guide [--json]` (aliases: `agent-help`, `schema`) providing a complete machine-readable or Markdown command and schema manifest for AI agents.
  - `tomatui spaces list|add|delete` and `--space` flags across `start`, `projects add`, `sessions list`, `stats`, and `export`.

### Changed
- **Unified Interaction Model**:
  - Universal `e` (Edit) across all three Projects panes (Spaces, Projects, Activities), History, and Settings (`Data Location`).
  - Double-press `d` confirmation guard before deleting/archiving spaces, projects, activities, or sessions.
  - Scrollable lists (`ListState`) across Projects, History, and Settings, plus expanded mouse click support for Space pills and list rows.
  - Standardized modal shortcuts (`Tab`/`Shift+Tab` to move fields, `↑↓`/`←→` to cycle values) and updated the `?` Help modal.
- Upgraded dependencies (`ratatui`, `crossterm`, `rusqlite`, `directories`).

## [0.1.0] - 2026-11-25

### Added
- **Minimalist Pomodoro TUI** built with Ratatui & Crossterm:
  - Opening View with bold geometric half-block numerals, quick Project/Plan selection, and one-keystroke Start (`Enter`/`Space`).
  - Manual 3-number Plan entry (`t`) supporting focus minutes, break minutes (default `5`), and batches (default `1`), plus quick presets.
  - Smooth 8× sub-cell Unicode progress bar with phase-tinted track.
  - 18 built-in 24-bit RGB color themes and **Transparent Background** mode (on by default for Ghostty, Kitty, WezTerm, Alacritty, and iTerm2).
  - Full keyboard (`1–4`, `T/P/H/S`, `Tab`, `,`, `Esc`) and mouse click navigation.
- **Projects → Activities → Editable Sessions** backed by bundled SQLite (WAL mode):
  - Start sessions immediately (including unassigned) and edit live or historical sessions (`e`/`Enter` in History) to assign/change project, activity, status, or notes.
  - Color-coded `Type` column (`work` / `rest` / `long`) in History and tail-scrolling text inputs in all modals.
  - View or relocate the SQLite database directly from `, Settings` → **Data Location**.
- **CLI & Agent-Friendly JSON Interface**:
  - Shorthand quick-launch (`tomatui PROJECT --25 --2`) and explicit `start` subcommand.
  - Structured `--json` subcommands: `projects`, `activities`, `sessions list|edit|delete`, `stats`, `export` (JSON/CSV), and `db-path`.
- **Zero-Asset Audio & Alerts**:
  - Pure in-memory 44.1 kHz PCM16 WAV chime synthesis on a dedicated worker thread + native desktop notifications.
