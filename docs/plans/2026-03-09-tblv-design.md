# tblv — Terminal Table Viewer Design

## Core Concept

Single-binary Rust TUI tool: `tblv <file.csv|file.parquet>`. Opens a scrollable table view with on-demand column statistics. Polars lazy backend, fixed-window data loading.

## Architecture

```
main.rs          — CLI arg parsing (clap), app entry
app.rs           — App state machine, event loop
data.rs          — Polars backend: load, head(N), describe, value_counts
ui/
  table_view.rs  — Main table rendering with cursor
  stats_view.rs  — Describe / uniques result display
  prompt.rs      — Confirmation dialog for full-scan ops
  help.rs        — Keybinding help overlay
```

**State machine:** `TableView` → (press `d`/`u`) → `ConfirmScan` → (press `y`) → `Loading` → `StatsView` → (press `Esc`) → `TableView`

## Data Layer

- Polars `LazyFrame` as the core handle — never eagerly load the full file
- `head(N)` collected into a `DataFrame` for the table view (default N=1000, configurable via flag)
- `describe()` and `value_counts()` trigger full scans — computed on demand only
- Uniques capped at top 100 by frequency
- CSV: auto-detect delimiter. Parquet: single file, metadata used for row count estimate.

## Keybindings

| Key | Action |
|-----|--------|
| `h/j/k/l` | Move cursor left/down/up/right |
| `Ctrl-d` / `Ctrl-u` | Jump N rows down/up |
| `Ctrl-l` / `Ctrl-h` | Jump to last/first column |
| `g` / `G` | Jump to first/last row |
| `d` | Describe current column (prompts if large) |
| `u` | Uniques for current column (prompts if large) |
| `y/n` | Confirm/cancel full scan |
| `Esc` | Back to table view |
| `q` | Quit |
| `?` | Help overlay |

## Dependencies

- **ratatui** + **crossterm** — TUI (already in Cargo.toml)
- **polars** — data backend (lazy feature)
- **clap** — CLI argument parsing
- **color-eyre** — error handling (already in Cargo.toml)

## Out of Scope (MVP)

- Partitioned parquet / directories
- Virtual scrolling / streaming beyond head(N)
- Filtering / sorting
- Column type overrides
- Export
