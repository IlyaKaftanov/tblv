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
    pub view: View,
    pub prompt: PromptState,
    pub should_quit: bool,
    pub data: DataFrame,
    pub columns: Vec<String>,
    pub dtypes: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub visible_rows: usize,
    pub row_offset: usize,
    pub col_offset: usize,
    pub visible_cols: usize,
    pub stats_result: Option<DataFrame>,
    pub loading: bool,
    pub stats_column: String,
}

impl App {
    /// Create a new App with defaults.
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
            visible_rows: 20,
            row_offset: 0,
            col_offset: 0,
            visible_cols: 5,
            stats_result: None,
            loading: false,
            stats_column: String::new(),
        }
    }

    /// Total number of rows in the loaded data.
    pub fn total_rows(&self) -> usize {
        self.data.height()
    }

    /// Total number of columns.
    pub fn total_cols(&self) -> usize {
        self.columns.len()
    }

    /// Move cursor down by `n` rows, clamped to bounds.
    pub fn cursor_down(&mut self, n: usize) {
        let max_row = self.total_rows().saturating_sub(1);
        self.cursor_row = (self.cursor_row + n).min(max_row);
        self.adjust_scroll();
    }

    /// Move cursor up by `n` rows, clamped to bounds.
    pub fn cursor_up(&mut self, n: usize) {
        self.cursor_row = self.cursor_row.saturating_sub(n);
        self.adjust_scroll();
    }

    /// Move cursor right by `n` columns, clamped to bounds.
    pub fn cursor_right(&mut self, n: usize) {
        let max_col = self.total_cols().saturating_sub(1);
        self.cursor_col = (self.cursor_col + n).min(max_col);
        self.adjust_scroll();
    }

    /// Move cursor left by `n` columns, clamped to bounds.
    pub fn cursor_left(&mut self, n: usize) {
        self.cursor_col = self.cursor_col.saturating_sub(n);
        self.adjust_scroll();
    }

    /// Jump cursor to the first row.
    pub fn cursor_top(&mut self) {
        self.cursor_row = 0;
        self.adjust_scroll();
    }

    /// Jump cursor to the last row.
    pub fn cursor_bottom(&mut self) {
        self.cursor_row = self.total_rows().saturating_sub(1);
        self.adjust_scroll();
    }

    /// Jump cursor to the first column.
    pub fn cursor_first_col(&mut self) {
        self.cursor_col = 0;
        self.adjust_scroll();
    }

    /// Jump cursor to the last column.
    pub fn cursor_last_col(&mut self) {
        self.cursor_col = self.total_cols().saturating_sub(1);
        self.adjust_scroll();
    }

    /// Get the name of the column under the cursor.
    pub fn current_column_name(&self) -> &str {
        &self.columns[self.cursor_col]
    }

    /// Ensure the cursor is visible within the scroll window.
    pub fn adjust_scroll(&mut self) {
        // Vertical scrolling
        if self.cursor_row < self.row_offset {
            self.row_offset = self.cursor_row;
        } else if self.cursor_row >= self.row_offset + self.visible_rows {
            self.row_offset = self.cursor_row - self.visible_rows + 1;
        }

        // Horizontal scrolling
        if self.cursor_col < self.col_offset {
            self.col_offset = self.cursor_col;
        } else if self.cursor_col >= self.col_offset + self.visible_cols {
            self.col_offset = self.cursor_col - self.visible_cols + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::df;

    fn make_test_app() -> App {
        let data = df! {
            "name" => &["alice", "bob", "carol", "dave", "eve"],
            "age"  => &[30, 25, 35, 40, 22],
            "score" => &[95.5, 87.3, 91.0, 78.2, 99.1],
        }
        .unwrap();
        let columns = vec![
            "name".to_string(),
            "age".to_string(),
            "score".to_string(),
        ];
        let dtypes = vec![
            "String".to_string(),
            "Int32".to_string(),
            "Float64".to_string(),
        ];
        App::new(data, columns, dtypes)
    }

    #[test]
    fn test_initial_state() {
        let app = make_test_app();
        assert_eq!(app.view, View::Table);
        assert_eq!(app.prompt, PromptState::None);
        assert!(!app.should_quit);
        assert_eq!(app.cursor_row, 0);
        assert_eq!(app.cursor_col, 0);
        assert_eq!(app.total_rows(), 5);
        assert_eq!(app.total_cols(), 3);
        assert!(!app.loading);
        assert!(app.stats_result.is_none());
        assert_eq!(app.stats_column, "");
    }

    #[test]
    fn test_cursor_down_and_clamp() {
        let mut app = make_test_app();
        app.cursor_down(2);
        assert_eq!(app.cursor_row, 2);

        // Move past the end — should clamp to last row (4)
        app.cursor_down(100);
        assert_eq!(app.cursor_row, 4);
    }

    #[test]
    fn test_cursor_up_and_clamp() {
        let mut app = make_test_app();
        app.cursor_row = 3;
        app.cursor_up(2);
        assert_eq!(app.cursor_row, 1);

        // Move past the start — should clamp to 0
        app.cursor_up(100);
        assert_eq!(app.cursor_row, 0);
    }

    #[test]
    fn test_cursor_right_and_clamp() {
        let mut app = make_test_app();
        app.cursor_right(1);
        assert_eq!(app.cursor_col, 1);

        // Move past the end — should clamp to last col (2)
        app.cursor_right(100);
        assert_eq!(app.cursor_col, 2);
    }

    #[test]
    fn test_cursor_left_and_clamp() {
        let mut app = make_test_app();
        app.cursor_col = 2;
        app.cursor_left(1);
        assert_eq!(app.cursor_col, 1);

        // Move past the start — should clamp to 0
        app.cursor_left(100);
        assert_eq!(app.cursor_col, 0);
    }

    #[test]
    fn test_jump_to_edges() {
        let mut app = make_test_app();

        app.cursor_bottom();
        assert_eq!(app.cursor_row, 4);

        app.cursor_top();
        assert_eq!(app.cursor_row, 0);

        app.cursor_last_col();
        assert_eq!(app.cursor_col, 2);

        app.cursor_first_col();
        assert_eq!(app.cursor_col, 0);
    }

    #[test]
    fn test_current_column_name() {
        let mut app = make_test_app();
        assert_eq!(app.current_column_name(), "name");
        app.cursor_right(1);
        assert_eq!(app.current_column_name(), "age");
        app.cursor_right(1);
        assert_eq!(app.current_column_name(), "score");
    }

    #[test]
    fn test_adjust_scroll_vertical() {
        let mut app = make_test_app();
        app.visible_rows = 3;

        // Cursor at row 0, offset 0 — no change
        app.adjust_scroll();
        assert_eq!(app.row_offset, 0);

        // Move cursor to row 4 (beyond visible window of 3)
        app.cursor_row = 4;
        app.adjust_scroll();
        assert_eq!(app.row_offset, 2); // 4 - 3 + 1 = 2

        // Move cursor back to row 0 — offset should follow
        app.cursor_row = 0;
        app.adjust_scroll();
        assert_eq!(app.row_offset, 0);
    }

    #[test]
    fn test_adjust_scroll_horizontal() {
        let mut app = make_test_app();
        app.visible_cols = 2;

        // Move cursor to col 2 (beyond visible window of 2)
        app.cursor_col = 2;
        app.adjust_scroll();
        assert_eq!(app.col_offset, 1); // 2 - 2 + 1 = 1

        // Move cursor back to col 0 — offset should follow
        app.cursor_col = 0;
        app.adjust_scroll();
        assert_eq!(app.col_offset, 0);
    }
}
