use ratatui::{
    Frame,
    layout::Constraint,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Row, Table, TableState},
};

use crate::app::App;

/// Render the main data table view.
pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Chrome: 2 lines for top/bottom border + 2 lines for header (col name + dtype).
    let chrome = 4_u16;
    let available_rows = area.height.saturating_sub(chrome) as usize;
    app.visible_rows = available_rows.max(1);

    // Calculate visible columns and their widths.
    let total_cols = app.total_cols();
    let col_end = total_cols.min(app.col_offset + 50); // upper bound scan
    let mut widths: Vec<u16> = Vec::new();
    let mut used_width: u16 = 1; // 1 for left border
    let mut vis_cols = 0;

    for ci in app.col_offset..col_end {
        let name_len = app.columns[ci].len() as u16;
        let w = name_len.clamp(12, 30);
        // +1 for column gap
        let needed = w + 1;
        if used_width + needed > area.width && vis_cols > 0 {
            break;
        }
        widths.push(w);
        used_width += needed;
        vis_cols += 1;
    }
    app.visible_cols = vis_cols.max(1);

    // Adjust scroll after updating visible dimensions.
    app.adjust_scroll();

    // Build header row: column name + dtype on two lines, with sort/filter indicators.
    let header_cells: Vec<Line> = (0..app.visible_cols)
        .map(|i| {
            let ci = app.col_offset + i;
            let name = &app.columns[ci];
            let dtype = &app.dtypes[ci];

            let sort_indicator = match app.sort_col {
                Some(sc) if sc == ci => {
                    if app.sort_desc { " \u{25bc}" } else { " \u{25b2}" }
                }
                _ => "",
            };

            let filter_indicator = if app.active_filter_for_col(name).is_some() {
                " [F]"
            } else {
                ""
            };

            Line::from(format!("{}{}{}\n{}", name, sort_indicator, filter_indicator, dtype))
        })
        .collect();

    let header = Row::new(header_cells)
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .height(2);

    // Build data rows.
    let row_start = app.row_offset;
    let row_end = (app.row_offset + app.visible_rows).min(app.total_rows());

    let rows: Vec<Row> = (row_start..row_end)
        .map(|ri| {
            let cells: Vec<String> = (0..app.visible_cols)
                .map(|i| {
                    let ci = app.col_offset + i;
                    let col_name = &app.columns[ci];
                    match app.data.column(col_name.as_str()) {
                        Ok(series) => {
                            let val = series.get(ri);
                            match val {
                                Ok(v) => format!("{}", v),
                                Err(_) => "ERR".to_string(),
                            }
                        }
                        Err(_) => "?".to_string(),
                    }
                })
                .collect();
            Row::new(cells)
        })
        .collect();

    // Title bar.
    let total_label = match app.total_file_rows {
        Some(total) => format!(" (of {} total)", total),
        None => String::new(),
    };
    let filter_label = if app.filters.is_empty() {
        String::new()
    } else {
        format!(" | {} filter(s) active", app.filters.len())
    };
    let sort_label = match app.sort_col {
        Some(sc) => {
            let dir = if app.sort_desc { "desc" } else { "asc" };
            format!(" | sort: {} {}", app.columns[sc], dir)
        }
        None => String::new(),
    };
    let title = format!(
        " tblv — {} rows{} × {} cols | row {}/{} col {}/{} «{}»{}{} ",
        app.total_rows(),
        total_label,
        app.total_cols(),
        app.cursor_row + 1,
        app.total_rows(),
        app.cursor_col + 1,
        app.total_cols(),
        app.current_column_name(),
        sort_label,
        filter_label,
    );

    let constraints: Vec<Constraint> = widths.iter().map(|&w| Constraint::Length(w)).collect();

    let table = Table::new(rows, &constraints)
        .header(header)
        .block(Block::bordered().title(title))
        // Subtle row highlight — just underline, no background
        .row_highlight_style(Style::default().add_modifier(Modifier::UNDERLINED))
        // No column highlight — let cell highlight do the work
        .column_highlight_style(Style::default())
        // Active cell: bright and reversed so it pops
        .cell_highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );

    // TableState tracks both row and column selection.
    let selected_row = if app.cursor_row >= app.row_offset {
        Some(app.cursor_row - app.row_offset)
    } else {
        Some(0)
    };
    let selected_col = if app.cursor_col >= app.col_offset {
        Some(app.cursor_col - app.col_offset)
    } else {
        Some(0)
    };
    let mut state = TableState::default()
        .with_selected(selected_row)
        .with_selected_column(selected_col);

    frame.render_stateful_widget(table, area, &mut state);
}
