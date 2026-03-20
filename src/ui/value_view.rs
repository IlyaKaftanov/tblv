use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Clear, Paragraph, Wrap},
};

use crate::app::App;

/// Render a popup showing the full value of the current cell.
pub fn render(frame: &mut Frame, app: &App) {
    let col_name = app.current_column_name();
    let dtype = &app.dtypes[app.cursor_col];

    // Split value into lines for proper display.
    let mut lines = vec![
        Line::from(format!("Column: {} ({})", col_name, dtype)).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(format!("Row: {}", app.cursor_row + 1)),
        Line::from(""),
    ];

    for line in app.cell_value.lines() {
        lines.push(Line::from(line.to_string()));
    }
    // Handle single-line values that may not contain newlines.
    if app.cell_value.lines().count() == 0 {
        lines.push(Line::from(app.cell_value.as_str()));
    }

    let text = Text::from(lines);

    // Use most of the screen height for the popup.
    let max_height = frame.area().height.saturating_sub(4);
    let height = max_height.clamp(8, max_height);

    let popup = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .scroll((app.value_scroll, 0))
        .block(
            Block::bordered()
                .title(" Value [j/k scroll, Esc close] ")
                .style(Style::default().fg(Color::White)),
        );

    let area = centered_rect(80, height, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}

/// Render a small error popup overlay (e.g. for map parse errors).
pub fn render_error(frame: &mut Frame, msg: &str) {
    let text = format!(" {} — press any key ", msg);
    let width = (text.len() as u16 + 2).min(frame.area().width);

    let vertical = Layout::vertical([Constraint::Length(3)])
        .flex(Flex::Center)
        .split(frame.area());
    let area = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .split(vertical[0])[0];

    let para = Paragraph::new(text).block(
        Block::bordered()
            .title(" Error ")
            .style(Style::default().fg(Color::Red)),
    );
    frame.render_widget(Clear, area);
    frame.render_widget(para, area);
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
