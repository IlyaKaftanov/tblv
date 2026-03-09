use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Clear, Paragraph},
};

use crate::app::App;

/// Render the help overlay.
pub fn render(frame: &mut Frame, _app: &mut App) {
    let key_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::White);
    let section_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("Navigation", section_style)),
        Line::from(vec![
            Span::styled("  h/j/k/l         ", key_style),
            Span::styled("Move cursor left/down/up/right", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl-d/Ctrl-u   ", key_style),
            Span::styled("Jump 20 rows down/up", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  g / G           ", key_style),
            Span::styled("Jump to first/last row", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl-h/Ctrl-l   ", key_style),
            Span::styled("Jump to first/last column", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Enter           ", key_style),
            Span::styled("View full cell value", desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("Sort", section_style)),
        Line::from(vec![
            Span::styled("  s               ", key_style),
            Span::styled("Sort column (None \u{2192} Asc \u{2192} Desc)", desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("Stats", section_style)),
        Line::from(vec![
            Span::styled("  d               ", key_style),
            Span::styled("Describe current column", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  u               ", key_style),
            Span::styled("Unique values for current column", desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("General", section_style)),
        Line::from(vec![
            Span::styled("  ?               ", key_style),
            Span::styled("Toggle this help", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  q / Esc         ", key_style),
            Span::styled("Quit / back to table", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl-c          ", key_style),
            Span::styled("Force quit", desc_style),
        ]),
        Line::from(""),
    ];

    let text = Text::from(lines);
    let height = text.lines.len() as u16 + 2; // +2 for borders

    let popup = Paragraph::new(text).block(
        Block::bordered()
            .title(" Help — tblv ")
            .style(Style::default().fg(Color::White)),
    );

    let area = centered_rect(60, height, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}

/// Create a centered rectangle of given percentage width and fixed height.
fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0]);
    horizontal[0]
}
