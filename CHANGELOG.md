# Changelog

All notable changes to `tomatui` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
