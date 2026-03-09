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

    // Build header row: column name + dtype on two lines.
    let header_cells: Vec<Line> = (0..app.visible_cols)
        .map(|i| {
            let ci = app.col_offset + i;
            let name = &app.columns[ci];
            let dtype = &app.dtypes[ci];
            Line::from(format!("{}\n{}", name, dtype))
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
    let title = format!(
        " tblv — {} rows × {} cols | row {}/{} col {}/{} ",
        app.total_rows(),
        app.total_cols(),
        app.cursor_row + 1,
        app.total_rows(),
        app.cursor_col + 1,
        app.total_cols(),
    );

    let constraints: Vec<Constraint> = widths.iter().map(|&w| Constraint::Length(w)).collect();

    let table = Table::new(rows, &constraints)
        .header(header)
        .block(Block::bordered().title(title))
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    // TableState tracks which row is highlighted.
    let selected = if app.cursor_row >= app.row_offset {
        Some(app.cursor_row - app.row_offset)
    } else {
        Some(0)
    };
    let mut state = TableState::default().with_selected(selected);

    frame.render_stateful_widget(table, area, &mut state);
}
