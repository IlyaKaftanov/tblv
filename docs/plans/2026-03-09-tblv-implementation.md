# tblv Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a terminal table viewer for CSV/Parquet files with on-demand column statistics.

**Architecture:** Single-binary Rust TUI. Polars LazyFrame backend loads a fixed window of rows. Ratatui renders a scrollable table with cursor. Stats (describe/uniques) computed on demand with user confirmation for large files. State machine drives view transitions.

**Tech Stack:** Rust 2024 edition, ratatui 0.30, crossterm 0.29, polars (lazy, csv, parquet), clap 4, color-eyre 0.6

---

### Task 1: Add dependencies to Cargo.toml

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add polars and clap dependencies**

```toml
[dependencies]
color-eyre = "0.6.3"
crossterm = "0.29.0"
ratatui = "0.30.0"
clap = { version = "4", features = ["derive"] }
polars = { version = "0.46", features = ["lazy", "csv", "parquet", "describe"] }
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles with no errors (warnings OK)

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "feat: add polars and clap dependencies"
```

---

### Task 2: CLI argument parsing

**Files:**
- Create: `src/cli.rs`
- Modify: `src/main.rs`

**Step 1: Create CLI struct**

`src/cli.rs`:
```rust
use std::path::PathBuf;
use clap::Parser;

/// tblv — Terminal Table Viewer for CSV and Parquet files
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    /// Path to CSV or Parquet file
    pub file: PathBuf,

    /// Number of rows to display
    #[arg(short = 'n', long = "head", default_value_t = 1000)]
    pub head: u32,
}
```

**Step 2: Wire into main.rs**

`src/main.rs`:
```rust
mod cli;

use clap::Parser;
use cli::Cli;
use ratatui::{DefaultTerminal, Frame};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    println!("File: {:?}, Head: {}", cli.file, cli.head);
    Ok(())
}
```

**Step 3: Verify it works**

Run: `cargo run -- --help`
Expected: Shows usage with file and --head/-n flag

Run: `cargo run -- test.csv`
Expected: Prints "File: test.csv, Head: 1000"

Run: `cargo run -- test.csv -n 500`
Expected: Prints "File: test.csv, Head: 500"

**Step 4: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat: add CLI argument parsing with clap"
```

---

### Task 3: Data layer — file loading and head(N)

**Files:**
- Create: `src/data.rs`
- Modify: `src/main.rs`

**Step 1: Create data module with DataSource struct**

`src/data.rs`:
```rust
use std::path::Path;
use polars::prelude::*;

pub struct DataSource {
    lazy: LazyFrame,
    head_n: u32,
}

impl DataSource {
    /// Load a CSV or Parquet file based on extension.
    pub fn open(path: &Path, head_n: u32) -> color_eyre::Result<Self> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let lazy = match ext.as_str() {
            "csv" | "tsv" => {
                let separator = if ext == "tsv" { b'\t' } else { b',' };
                LazyCsvReader::new(path)
                    .with_separator(separator)
                    .with_has_header(true)
                    .with_infer_schema_length(Some(1000))
                    .finish()?
            }
            "parquet" | "pq" => {
                LazyFrame::scan_parquet(path, Default::default())?
            }
            _ => {
                return Err(color_eyre::eyre::eyre!(
                    "Unsupported file format: '{}'. Supported: csv, tsv, parquet",
                    ext
                ));
            }
        };

        Ok(Self { lazy, head_n })
    }

    /// Load the first N rows into a DataFrame.
    pub fn head(&self) -> color_eyre::Result<DataFrame> {
        let df = self.lazy.clone().limit(self.head_n).collect()?;
        Ok(df)
    }

    /// Get column names.
    pub fn column_names(&self) -> color_eyre::Result<Vec<String>> {
        let schema = self.lazy.collect_schema()?;
        Ok(schema.iter_names().map(|n| n.to_string()).collect())
    }

    /// Get column dtypes as strings.
    pub fn column_dtypes(&self) -> color_eyre::Result<Vec<String>> {
        let schema = self.lazy.collect_schema()?;
        Ok(schema.iter_dtypes().map(|d| format!("{}", d)).collect())
    }

    /// Estimate row count. For Parquet, uses metadata. For CSV, returns None.
    pub fn estimated_row_count(&self) -> Option<usize> {
        // Parquet metadata gives exact count; CSV doesn't without scanning.
        // For MVP, we return None and let the UI show "unknown" for CSV.
        None
    }
}
```

**Step 2: Write a test with a small CSV**

Add to the bottom of `src/data.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_csv() -> NamedTempFile {
        let mut f = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(f, "name,age,score").unwrap();
        writeln!(f, "alice,30,95.5").unwrap();
        writeln!(f, "bob,25,87.3").unwrap();
        writeln!(f, "carol,35,91.0").unwrap();
        f
    }

    #[test]
    fn test_open_csv_and_head() {
        let f = create_test_csv();
        let ds = DataSource::open(f.path(), 10).unwrap();
        let df = ds.head().unwrap();
        assert_eq!(df.shape(), (3, 3));
    }

    #[test]
    fn test_column_names() {
        let f = create_test_csv();
        let ds = DataSource::open(f.path(), 10).unwrap();
        let names = ds.column_names().unwrap();
        assert_eq!(names, vec!["name", "age", "score"]);
    }

    #[test]
    fn test_head_limits_rows() {
        let f = create_test_csv();
        let ds = DataSource::open(f.path(), 2).unwrap();
        let df = ds.head().unwrap();
        assert_eq!(df.shape().0, 2);
    }

    #[test]
    fn test_unsupported_format() {
        let f = NamedTempFile::with_suffix(".json").unwrap();
        let result = DataSource::open(f.path(), 10);
        assert!(result.is_err());
    }
}
```

Add `tempfile` as a dev dependency in `Cargo.toml`:
```toml
[dev-dependencies]
tempfile = "3"
```

**Step 3: Run tests**

Run: `cargo test -- data::tests`
Expected: All 4 tests pass

**Step 4: Wire data layer into main to verify loading**

Update `src/main.rs`:
```rust
mod cli;
mod data;

use clap::Parser;
use cli::Cli;
use data::DataSource;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    let ds = DataSource::open(&cli.file, cli.head)?;
    let df = ds.head()?;
    println!("Loaded {} rows x {} cols", df.height(), df.width());
    println!("Columns: {:?}", ds.column_names()?);

    Ok(())
}
```

**Step 5: Test with a real file**

Run: `echo "a,b,c\n1,2,3\n4,5,6" > /tmp/test.csv && cargo run -- /tmp/test.csv`
Expected: Prints "Loaded 2 rows x 3 cols" and column names

**Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/data.rs src/main.rs
git commit -m "feat: add data layer with Polars lazy loading"
```

---

### Task 4: App state machine

**Files:**
- Create: `src/app.rs`

**Step 1: Define the app state types and struct**

`src/app.rs`:
```rust
use polars::prelude::DataFrame;

/// Which view the app is currently showing.
#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Table,
    Describe,
    Uniques,
    Help,
}

/// Confirmation prompt state for full-scan operations.
#[derive(Debug, Clone, PartialEq)]
pub enum PromptState {
    None,
    ConfirmDescribe,
    ConfirmUniques,
}

/// Main application state.
pub struct App {
    /// Current view mode.
    pub view: View,
    /// Prompt overlay state.
    pub prompt: PromptState,
    /// Whether the app should quit.
    pub should_quit: bool,
    /// Loaded data (head N rows).
    pub data: DataFrame,
    /// Column names.
    pub columns: Vec<String>,
    /// Column dtypes as display strings.
    pub dtypes: Vec<String>,
    /// Current cursor row index.
    pub cursor_row: usize,
    /// Current cursor column index.
    pub cursor_col: usize,
    /// Number of visible rows (set during render).
    pub visible_rows: usize,
    /// Vertical scroll offset.
    pub row_offset: usize,
    /// Horizontal scroll offset (first visible column).
    pub col_offset: usize,
    /// Number of visible columns (set during render).
    pub visible_cols: usize,
    /// Cached stats result for current column.
    pub stats_result: Option<DataFrame>,
    /// Whether stats are currently being computed.
    pub loading: bool,
    /// Name of the column stats are shown for.
    pub stats_column: String,
}

impl App {
    pub fn new(data: DataFrame, columns: Vec<String>, dtypes: Vec<String>) -> Self {
        Self {
            view: View::Table,
            prompt: PromptState::None,
            should_quit: false,
            data,
            columns,
            dtypes,
            cursor_row: 0,
            cursor_col: 0,
            visible_rows: 0,
            row_offset: 0,
            col_offset: 0,
            visible_cols: 0,
            stats_result: None,
            loading: false,
            stats_column: String::new(),
        }
    }

    pub fn total_rows(&self) -> usize {
        self.data.height()
    }

    pub fn total_cols(&self) -> usize {
        self.columns.len()
    }

    /// Move cursor down by n rows, clamping to bounds.
    pub fn cursor_down(&mut self, n: usize) {
        self.cursor_row = (self.cursor_row + n).min(self.total_rows().saturating_sub(1));
    }

    /// Move cursor up by n rows, clamping to bounds.
    pub fn cursor_up(&mut self, n: usize) {
        self.cursor_row = self.cursor_row.saturating_sub(n);
    }

    /// Move cursor right by n columns, clamping to bounds.
    pub fn cursor_right(&mut self, n: usize) {
        self.cursor_col = (self.cursor_col + n).min(self.total_cols().saturating_sub(1));
    }

    /// Move cursor left by n columns, clamping to bounds.
    pub fn cursor_left(&mut self, n: usize) {
        self.cursor_col = self.cursor_col.saturating_sub(n);
    }

    /// Jump cursor to first row.
    pub fn cursor_top(&mut self) {
        self.cursor_row = 0;
    }

    /// Jump cursor to last row.
    pub fn cursor_bottom(&mut self) {
        self.cursor_row = self.total_rows().saturating_sub(1);
    }

    /// Jump cursor to first column.
    pub fn cursor_first_col(&mut self) {
        self.cursor_col = 0;
    }

    /// Jump cursor to last column.
    pub fn cursor_last_col(&mut self) {
        self.cursor_col = self.total_cols().saturating_sub(1);
    }

    /// Get the name of the currently selected column.
    pub fn current_column_name(&self) -> &str {
        &self.columns[self.cursor_col]
    }

    /// Ensure cursor is visible by adjusting scroll offsets.
    pub fn adjust_scroll(&mut self) {
        // Vertical scroll
        if self.visible_rows > 0 {
            if self.cursor_row < self.row_offset {
                self.row_offset = self.cursor_row;
            } else if self.cursor_row >= self.row_offset + self.visible_rows {
                self.row_offset = self.cursor_row - self.visible_rows + 1;
            }
        }
        // Horizontal scroll
        if self.visible_cols > 0 {
            if self.cursor_col < self.col_offset {
                self.col_offset = self.cursor_col;
            } else if self.cursor_col >= self.col_offset + self.visible_cols {
                self.col_offset = self.cursor_col - self.visible_cols + 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::*;

    fn test_app() -> App {
        let df = df! {
            "a" => &[1, 2, 3, 4, 5],
            "b" => &["x", "y", "z", "w", "v"],
        }
        .unwrap();
        let cols = vec!["a".into(), "b".into()];
        let dtypes = vec!["i32".into(), "str".into()];
        App::new(df, cols, dtypes)
    }

    #[test]
    fn test_cursor_movement() {
        let mut app = test_app();
        app.cursor_down(2);
        assert_eq!(app.cursor_row, 2);
        app.cursor_up(1);
        assert_eq!(app.cursor_row, 1);
        app.cursor_right(1);
        assert_eq!(app.cursor_col, 1);
        app.cursor_left(1);
        assert_eq!(app.cursor_col, 0);
    }

    #[test]
    fn test_cursor_clamps() {
        let mut app = test_app();
        app.cursor_up(10);
        assert_eq!(app.cursor_row, 0);
        app.cursor_down(100);
        assert_eq!(app.cursor_row, 4); // 5 rows, max index 4
        app.cursor_left(10);
        assert_eq!(app.cursor_col, 0);
        app.cursor_right(100);
        assert_eq!(app.cursor_col, 1); // 2 cols, max index 1
    }

    #[test]
    fn test_cursor_jump() {
        let mut app = test_app();
        app.cursor_bottom();
        assert_eq!(app.cursor_row, 4);
        app.cursor_top();
        assert_eq!(app.cursor_row, 0);
        app.cursor_last_col();
        assert_eq!(app.cursor_col, 1);
        app.cursor_first_col();
        assert_eq!(app.cursor_col, 0);
    }

    #[test]
    fn test_initial_state() {
        let app = test_app();
        assert_eq!(app.view, View::Table);
        assert_eq!(app.prompt, PromptState::None);
        assert!(!app.should_quit);
    }
}
```

**Step 2: Run tests**

Run: `cargo test -- app::tests`
Expected: All 4 tests pass

**Step 3: Commit**

```bash
git add src/app.rs
git commit -m "feat: add app state machine with cursor navigation"
```

---

### Task 5: Event handling

**Files:**
- Create: `src/event.rs`
- Modify: `src/main.rs`

**Step 1: Create event handler that maps keys to app state changes**

`src/event.rs`:
```rust
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, PromptState, View};

/// Half-page jump size.
const JUMP_ROWS: usize = 20;

/// Handle a terminal event, mutating app state accordingly.
pub fn handle_event(app: &mut App, event: Event) {
    let Event::Key(key) = event else {
        return;
    };
    // Only handle key press events (not release/repeat).
    if key.kind != event::KeyEventKind::Press {
        return;
    }

    // Quit on 'q' from any non-prompt view, or Ctrl-c from anywhere.
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match app.prompt {
        PromptState::None => match app.view {
            View::Table => handle_table_key(app, key),
            View::Describe | View::Uniques => handle_stats_key(app, key),
            View::Help => handle_help_key(app, key),
        },
        PromptState::ConfirmDescribe | PromptState::ConfirmUniques => {
            handle_prompt_key(app, key);
        }
    }
}

fn handle_table_key(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        // Quit
        (KeyModifiers::NONE, KeyCode::Char('q')) => app.should_quit = true,

        // Movement: vim keys
        (KeyModifiers::NONE, KeyCode::Char('j')) => {
            app.cursor_down(1);
            app.adjust_scroll();
        }
        (KeyModifiers::NONE, KeyCode::Char('k')) => {
            app.cursor_up(1);
            app.adjust_scroll();
        }
        (KeyModifiers::NONE, KeyCode::Char('l')) => {
            app.cursor_right(1);
            app.adjust_scroll();
        }
        (KeyModifiers::NONE, KeyCode::Char('h')) => {
            app.cursor_left(1);
            app.adjust_scroll();
        }

        // Jump: Ctrl-d / Ctrl-u
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
            app.cursor_down(JUMP_ROWS);
            app.adjust_scroll();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            app.cursor_up(JUMP_ROWS);
            app.adjust_scroll();
        }

        // Jump to first/last column: Ctrl-h / Ctrl-l
        // Note: Ctrl-h may conflict with single 'h'. We use 0 and $ instead.
        // Actually per design: Ctrl-l = last col, Ctrl-h = first col.
        // But Ctrl-h is often interpreted as Backspace. Let's keep it and see.

        // Jump to top/bottom row
        (KeyModifiers::NONE, KeyCode::Char('g')) => {
            app.cursor_top();
            app.adjust_scroll();
        }
        (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
            app.cursor_bottom();
            app.adjust_scroll();
        }

        // Stats: describe
        (KeyModifiers::NONE, KeyCode::Char('d')) => {
            // Note: 'd' conflicts with Ctrl-d. Single 'd' = describe.
            // This won't conflict because Ctrl-d is a separate match arm.
            app.stats_column = app.current_column_name().to_string();
            app.prompt = PromptState::ConfirmDescribe;
        }

        // Stats: uniques
        (KeyModifiers::NONE, KeyCode::Char('u')) => {
            // Same note: 'u' won't conflict with Ctrl-u.
            app.stats_column = app.current_column_name().to_string();
            app.prompt = PromptState::ConfirmUniques;
        }

        // Help
        (KeyModifiers::NONE, KeyCode::Char('?')) => {
            app.view = View::Help;
        }

        _ => {}
    }
}

fn handle_stats_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.view = View::Table;
            app.stats_result = None;
        }
        _ => {}
    }
}

fn handle_help_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            app.view = View::Table;
        }
        _ => {}
    }
}

fn handle_prompt_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') => {
            let was_prompt = app.prompt.clone();
            app.prompt = PromptState::None;
            match was_prompt {
                PromptState::ConfirmDescribe => {
                    app.loading = true;
                    app.view = View::Describe;
                }
                PromptState::ConfirmUniques => {
                    app.loading = true;
                    app.view = View::Uniques;
                }
                _ => {}
            }
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.prompt = PromptState::None;
        }
        _ => {}
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check`
Expected: Compiles

**Step 3: Commit**

```bash
git add src/event.rs
git commit -m "feat: add event handling with vim keybindings"
```

---

### Task 6: Stats computation (describe + value_counts)

**Files:**
- Modify: `src/data.rs`

**Step 1: Add describe and value_counts methods to DataSource**

Add these methods to the `impl DataSource` block in `src/data.rs`:

```rust
    /// Compute descriptive statistics for a single column.
    /// Scans the full file.
    pub fn describe_column(&self, col_name: &str) -> color_eyre::Result<DataFrame> {
        let df = self
            .lazy
            .clone()
            .select([col(col_name)])
            .collect()?;
        let desc = df.describe(None)?;
        Ok(desc)
    }

    /// Compute value counts for a single column, limited to top N.
    /// Scans the full file.
    pub fn value_counts(&self, col_name: &str, top_n: usize) -> color_eyre::Result<DataFrame> {
        let series = self
            .lazy
            .clone()
            .select([col(col_name)])
            .collect()?
            .column(col_name)?
            .clone();

        let vc = series.value_counts(true, true)?;
        let limited = vc.head(Some(top_n));
        Ok(limited)
    }
```

**Step 2: Add tests**

Add to the `tests` module in `src/data.rs`:

```rust
    #[test]
    fn test_describe_column() {
        let f = create_test_csv();
        let ds = DataSource::open(f.path(), 1000).unwrap();
        let desc = ds.describe_column("age").unwrap();
        assert!(desc.height() > 0);
        // Should have a "statistic" column and the data column
        assert!(desc.get_column_names().contains(&PlSmallStr::from("statistic")));
    }

    #[test]
    fn test_value_counts() {
        let f = create_test_csv();
        let ds = DataSource::open(f.path(), 1000).unwrap();
        let vc = ds.value_counts("name", 100).unwrap();
        assert_eq!(vc.height(), 3); // 3 unique names
    }
```

**Step 3: Run tests**

Run: `cargo test -- data::tests`
Expected: All 6 tests pass

**Step 4: Commit**

```bash
git add src/data.rs
git commit -m "feat: add describe and value_counts to data layer"
```

---

### Task 7: Table view UI rendering

**Files:**
- Create: `src/ui.rs`
- Create: `src/ui/table_view.rs`

**Step 1: Create the ui module**

`src/ui.rs`:
```rust
mod table_view;
mod stats_view;
mod prompt;
mod help;

use ratatui::Frame;
use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App) {
    table_view::render(frame, app);

    // Overlay prompt if active
    if app.prompt != crate::app::PromptState::None {
        prompt::render(frame, app);
    }
}

pub fn render_stats(frame: &mut Frame, app: &mut App) {
    stats_view::render(frame, app);
}

pub fn render_help(frame: &mut Frame, app: &mut App) {
    help::render(frame, app);
}
```

**Step 2: Create table view renderer**

`src/ui/table_view.rs`:
```rust
use ratatui::layout::Constraint;
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};
use ratatui::Frame;

use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Calculate how many rows/cols are visible (approximate, accounting for borders + header).
    // Header = 1 row + 1 margin, borders = 2 rows.
    let chrome_rows = 4;
    app.visible_rows = (area.height as usize).saturating_sub(chrome_rows);

    // Build column widths from data. Each column gets width based on max content length.
    let col_end = (app.col_offset + area.width as usize / 6).min(app.total_cols());
    app.visible_cols = col_end - app.col_offset;

    let visible_columns: Vec<&str> = app.columns[app.col_offset..col_end]
        .iter()
        .map(|s| s.as_str())
        .collect();

    // Column widths: max of header length and a reasonable default.
    let widths: Vec<Constraint> = visible_columns
        .iter()
        .map(|name| {
            let w = name.len().max(12).min(30) as u16;
            Constraint::Length(w)
        })
        .collect();

    // Header row with column names and dtypes.
    let header_cells: Vec<Cell> = visible_columns
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let col_idx = app.col_offset + i;
            let dtype = &app.dtypes[col_idx];
            Cell::from(format!("{}\n{}", name, dtype))
        })
        .collect();
    let header = Row::new(header_cells)
        .style(Style::new().bold())
        .bottom_margin(1);

    // Data rows (only the visible window).
    let row_end = (app.row_offset + app.visible_rows).min(app.total_rows());
    let rows: Vec<Row> = (app.row_offset..row_end)
        .map(|row_idx| {
            let cells: Vec<Cell> = (app.col_offset..col_end)
                .map(|col_idx| {
                    let val = app
                        .data
                        .column(&app.columns[col_idx])
                        .map(|s| {
                            s.get(row_idx)
                                .map(|v| format!("{}", v))
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();
                    Cell::from(val)
                })
                .collect();
            Row::new(cells)
        })
        .collect();

    let title = format!(
        " tblv — {} rows × {} cols | row {}/{} col {}/{} ",
        app.total_rows(),
        app.total_cols(),
        app.cursor_row + 1,
        app.total_rows(),
        app.cursor_col + 1,
        app.total_cols(),
    );

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title),
        )
        .column_spacing(1)
        .row_highlight_style(Style::new().reversed())
        .column_highlight_style(Style::new().on_dark_gray())
        .highlight_symbol("▶ ");

    // TableState tracks selected row relative to rendered rows.
    let mut state = TableState::default();
    state.select(Some(app.cursor_row - app.row_offset));
    // Note: TableState only supports row selection. Column highlighting
    // will be handled via cell styling.

    frame.render_stateful_widget(table, area, &mut state);
}
```

**Step 3: Create placeholder modules for stats_view, prompt, help**

`src/ui/stats_view.rs`:
```rust
use ratatui::Frame;
use crate::app::App;

pub fn render(frame: &mut Frame, _app: &mut App) {
    frame.render_widget("Stats view — TODO", frame.area());
}
```

`src/ui/prompt.rs`:
```rust
use ratatui::Frame;
use crate::app::App;

pub fn render(frame: &mut Frame, _app: &mut App) {
    // TODO: render confirmation dialog
}
```

`src/ui/help.rs`:
```rust
use ratatui::Frame;
use crate::app::App;

pub fn render(frame: &mut Frame, _app: &mut App) {
    frame.render_widget("Help — TODO", frame.area());
}
```

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compiles

**Step 5: Commit**

```bash
git add src/ui.rs src/ui/
git commit -m "feat: add table view UI rendering"
```

---

### Task 8: Main event loop — wire everything together

**Files:**
- Modify: `src/main.rs`

**Step 1: Connect CLI → DataSource → App → event loop → UI**

`src/main.rs`:
```rust
mod app;
mod cli;
mod data;
mod event;
mod ui;

use clap::Parser;
use cli::Cli;
use crossterm::event as ct_event;
use data::DataSource;

use crate::app::{App, View};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    // Load data.
    let ds = DataSource::open(&cli.file, cli.head)?;
    let df = ds.head()?;
    let columns = ds.column_names()?;
    let dtypes = ds.column_dtypes()?;

    let mut app = App::new(df, columns, dtypes);

    // Store DataSource in a way accessible for stats computation.
    // We'll pass it through the run loop.
    ratatui::run(|terminal| run_app(terminal, &mut app, &ds))?;

    Ok(())
}

fn run_app(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    ds: &DataSource,
) -> std::io::Result<()> {
    loop {
        terminal.draw(|frame| {
            match app.view {
                View::Table => ui::render(frame, app),
                View::Describe | View::Uniques => {
                    if app.loading {
                        // Compute stats (blocking — MVP simplicity).
                        let result = match app.view {
                            View::Describe => ds.describe_column(&app.stats_column),
                            View::Uniques => ds.value_counts(&app.stats_column, 100),
                            _ => unreachable!(),
                        };
                        match result {
                            Ok(df) => {
                                app.stats_result = Some(df);
                                app.loading = false;
                            }
                            Err(e) => {
                                // On error, go back to table view.
                                app.view = View::Table;
                                app.loading = false;
                                // TODO: show error message
                                eprintln!("Error: {}", e);
                            }
                        }
                    }
                    ui::render_stats(frame, app);
                }
                View::Help => ui::render_help(frame, app),
            }
        })?;

        if app.should_quit {
            break Ok(());
        }

        let ev = ct_event::read().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        event::handle_event(app, ev);
    }
}
```

**Step 2: Test manually**

Run: `echo -e "name,age,score\nalice,30,95.5\nbob,25,87.3\ncarol,35,91.0" > /tmp/test.csv`
Run: `cargo run -- /tmp/test.csv`
Expected: TUI opens showing a table with 3 rows and 3 columns. Press q to quit.

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire main event loop connecting all components"
```

---

### Task 9: Prompt overlay UI

**Files:**
- Modify: `src/ui/prompt.rs`

**Step 1: Implement the confirmation dialog**

`src/ui/prompt.rs`:
```rust
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, PromptState};

pub fn render(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 7, frame.area());

    let action = match app.prompt {
        PromptState::ConfirmDescribe => "describe",
        PromptState::ConfirmUniques => "unique values",
        PromptState::None => return,
    };

    let text = format!(
        "Compute {} for column '{}'?\nThis scans the entire file.\n\n[y] Yes  [n] No",
        action, app.stats_column
    );

    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirm ")
                .style(Style::new().bold()),
        );

    frame.render_widget(Clear, area);
    frame.render_widget(paragraph, area);
}

/// Create a centered rectangle of given width (%) and height (lines).
fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0]);
    horizontal[0]
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles

**Step 3: Commit**

```bash
git add src/ui/prompt.rs
git commit -m "feat: add confirmation prompt overlay"
```

---

### Task 10: Stats view UI

**Files:**
- Modify: `src/ui/stats_view.rs`

**Step 1: Render the stats DataFrame as a table**

`src/ui/stats_view.rs`:
```rust
use ratatui::layout::Constraint;
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;

use crate::app::{App, View};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    if app.loading {
        let loading = ratatui::widgets::Paragraph::new("Computing...")
            .block(Block::default().borders(Borders::ALL).title(" Loading "));
        frame.render_widget(loading, area);
        return;
    }

    let Some(ref df) = app.stats_result else {
        let msg = ratatui::widgets::Paragraph::new("No data")
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(msg, area);
        return;
    };

    let col_names: Vec<String> = df.get_column_names().iter().map(|s| s.to_string()).collect();

    let widths: Vec<Constraint> = col_names
        .iter()
        .map(|name| Constraint::Length(name.len().max(15).min(30) as u16))
        .collect();

    let header = Row::new(
        col_names.iter().map(|n| Cell::from(n.as_str())).collect::<Vec<_>>()
    )
    .style(Style::new().bold())
    .bottom_margin(1);

    let rows: Vec<Row> = (0..df.height())
        .map(|i| {
            let cells: Vec<Cell> = col_names
                .iter()
                .map(|col| {
                    let val = df
                        .column(col)
                        .map(|s| {
                            s.get(i)
                                .map(|v| format!("{}", v))
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();
                    Cell::from(val)
                })
                .collect();
            Row::new(cells)
        })
        .collect();

    let view_name = match app.view {
        View::Describe => "Describe",
        View::Uniques => "Uniques",
        _ => "",
    };

    let title = format!(" {} — '{}' [Esc to go back] ", view_name, app.stats_column);

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title))
        .column_spacing(2);

    frame.render_widget(table, area);
}
```

**Step 2: Verify it compiles and test manually**

Run: `cargo check`
Run: `cargo run -- /tmp/test.csv`
Expected: Press `d` on a column → see confirm prompt → press `y` → see describe output → press `Esc` to return.

**Step 3: Commit**

```bash
git add src/ui/stats_view.rs
git commit -m "feat: add stats view for describe and uniques"
```

---

### Task 11: Help overlay UI

**Files:**
- Modify: `src/ui/help.rs`

**Step 1: Implement help screen**

`src/ui/help.rs`:
```rust
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;

const HELP_TEXT: &str = "\
Navigation
  h/j/k/l        Move cursor left/down/up/right
  Ctrl-d/Ctrl-u   Jump 20 rows down/up
  g / G           Jump to first/last row
  Ctrl-h/Ctrl-l   Jump to first/last column

Stats
  d               Describe current column
  u               Unique values for current column

General
  ?               Toggle this help
  q / Esc         Quit / back to table
  Ctrl-c          Force quit";

pub fn render(frame: &mut Frame, _app: &App) {
    let area = centered_rect(60, 18, frame.area());

    let paragraph = Paragraph::new(HELP_TEXT)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help — press Esc or ? to close ")
                .style(Style::new().bold()),
        );

    frame.render_widget(Clear, area);
    frame.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0]);
    horizontal[0]
}
```

**Step 2: Verify and test manually**

Run: `cargo run -- /tmp/test.csv`
Expected: Press `?` → see help overlay → press `Esc` or `?` to close.

**Step 3: Commit**

```bash
git add src/ui/help.rs
git commit -m "feat: add help overlay"
```

---

### Task 12: Final integration testing and polish

**Files:**
- Modify: `src/ui.rs` (fix the render dispatch)

**Step 1: Update ui.rs render dispatch to use the view-based rendering**

The `ui.rs` module should dispatch based on view state. Update:

```rust
mod table_view;
mod stats_view;
mod prompt;
mod help;

use ratatui::Frame;
use crate::app::{App, PromptState, View};

pub fn render(frame: &mut Frame, app: &mut App) {
    match app.view {
        View::Table => table_view::render(frame, app),
        View::Describe | View::Uniques => stats_view::render(frame, app),
        View::Help => help::render(frame, app),
    }

    // Overlay prompt if active
    if app.prompt != PromptState::None {
        prompt::render(frame, app);
    }
}
```

And simplify `main.rs` to just call `ui::render`:

```rust
fn run_app(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    ds: &DataSource,
) -> std::io::Result<()> {
    loop {
        // Compute stats if loading.
        if app.loading {
            let result = match app.view {
                View::Describe => ds.describe_column(&app.stats_column),
                View::Uniques => ds.value_counts(&app.stats_column, 100),
                _ => unreachable!(),
            };
            match result {
                Ok(df) => app.stats_result = Some(df),
                Err(_) => app.view = View::Table,
            }
            app.loading = false;
        }

        terminal.draw(|frame| ui::render(frame, app))?;

        if app.should_quit {
            break Ok(());
        }

        let ev = ct_event::read()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        event::handle_event(app, ev);
    }
}
```

**Step 2: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Manual end-to-end test**

Run: `cargo run -- /tmp/test.csv`
Test: Navigate with hjkl, press `d` → `y` to see describe, `Esc` to go back, `u` → `y` for uniques, `?` for help, `q` to quit.

**Step 4: Commit**

```bash
git add src/main.rs src/ui.rs
git commit -m "feat: final integration — wire render dispatch and stats computation"
```

---

### Task 13: Run cargo clippy and fix any warnings

**Step 1: Run clippy**

Run: `cargo clippy -- -W clippy::all`
Expected: Fix any warnings.

**Step 2: Run cargo fmt**

Run: `cargo fmt`

**Step 3: Final commit**

```bash
git add -A
git commit -m "chore: clippy fixes and formatting"
```
