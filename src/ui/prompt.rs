use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Clear, Paragraph},
};

use crate::app::{App, PromptState};

/// Render a confirmation prompt overlay.
pub fn render(frame: &mut Frame, app: &mut App) {
    let action = match app.prompt {
        PromptState::ConfirmDescribe => "describe",
        PromptState::ConfirmUniques => "unique values",
        PromptState::None => return,
    };

    let text = Text::from(vec![
        Line::from(""),
        Line::from(format!(
            "Compute {} for column '{}'?",
            action, app.stats_column
        )),
        Line::from("This scans the entire file."),
        Line::from(""),
        Line::from("[y] Yes  [n] No"),
    ]);

    let popup = Paragraph::new(text)
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::bordered()
                .title(" Confirm ")
                .style(Style::default().fg(Color::White)),
        );

    let area = centered_rect(50, 7, frame.area());
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
