use ratatui::{
    Frame,
    layout::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Block, Paragraph, Row, Table},
};

use crate::app::{App, View};

/// Render the stats view (Describe or Uniques).
pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let view_label = match app.view {
        View::Describe => "Describe",
        View::Uniques => "Uniques",
        _ => "Stats",
    };

    if app.loading {
        let msg = Paragraph::new("Computing...")
            .block(
                Block::bordered().title(format!(
                    " {} — '{}' [Esc to go back] ",
                    view_label, app.stats_column
                )),
            );
        frame.render_widget(msg, area);
        return;
    }

    let Some(df) = &app.stats_result else {
        let msg = Paragraph::new("No data")
            .block(Block::bordered().title(format!(" {} ", view_label)));
        frame.render_widget(msg, area);
        return;
    };

    let col_names: Vec<String> = df.get_column_names().iter().map(|s| s.to_string()).collect();

    // Header row.
    let header = Row::new(col_names.clone())
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    // Data rows.
    let rows: Vec<Row> = (0..df.height())
        .map(|ri| {
            let cells: Vec<String> = col_names
                .iter()
                .map(|cn| {
                    match df.column(cn.as_str()) {
                        Ok(series) => match series.get(ri) {
                            Ok(v) => format!("{}", v),
                            Err(_) => "ERR".to_string(),
                        },
                        Err(_) => "?".to_string(),
                    }
                })
                .collect();
            Row::new(cells)
        })
        .collect();

    let constraints: Vec<Constraint> = col_names
        .iter()
        .map(|name| {
            let w = (name.len() as u16).max(12).min(30);
            Constraint::Length(w)
        })
        .collect();

    let title = format!(
        " {} — '{}' [Esc to go back] ",
        view_label, app.stats_column
    );

    let table = Table::new(rows, &constraints)
        .header(header)
        .block(Block::bordered().title(title));

    frame.render_widget(table, area);
}
