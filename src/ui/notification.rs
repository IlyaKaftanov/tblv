use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Clear, Paragraph},
};

use crate::app::App;

/// Render a notification popup overlay.
pub fn render(frame: &mut Frame, app: &App) {
    let Some(ref message) = app.notification else {
        return;
    };

    let lines: Vec<Line> = message.lines().map(Line::from).collect();
    let height = (lines.len() + 2) as u16; // +2 for border

    let popup = Paragraph::new(Text::from(lines))
        .block(
            Block::bordered()
                .title(" Notice ")
                .style(Style::default().fg(Color::Yellow)),
        );

    let area = centered_rect(60, height, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
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
