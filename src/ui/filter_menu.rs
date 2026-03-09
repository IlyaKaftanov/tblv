use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Clear, Paragraph},
};

use crate::app::App;

/// Render the filter menu overlay.
pub fn render(frame: &mut Frame, app: &mut App) {
    let col_name = &app.stats_column;

    if app.filter_items.is_empty() {
        // Still loading or no values
        let text = Text::from(vec![
            Line::from(""),
            Line::from("Loading unique values..."),
            Line::from(""),
        ]);
        let popup = Paragraph::new(text)
            .alignment(ratatui::layout::Alignment::Center)
            .block(
                Block::bordered()
                    .title(format!(" Filter: {} ", col_name))
                    .style(Style::default().fg(Color::White)),
            );
        let area = centered_rect(50, 5, frame.area());
        frame.render_widget(Clear, area);
        frame.render_widget(popup, area);
        return;
    }

    let max_popup_height = frame.area().height.saturating_sub(6);
    // 4 lines of chrome: header hint line, blank, footer hint, blank
    let chrome_lines: u16 = 4;
    let item_area = max_popup_height.saturating_sub(chrome_lines + 2); // +2 for borders
    let visible_items = item_area as usize;

    // Adjust scroll so cursor is visible.
    if app.filter_menu_cursor < app.filter_menu_scroll {
        app.filter_menu_scroll = app.filter_menu_cursor;
    } else if app.filter_menu_cursor >= app.filter_menu_scroll + visible_items {
        app.filter_menu_scroll = app.filter_menu_cursor - visible_items + 1;
    }

    let selected_count = app.filter_items.iter().filter(|(_, s)| *s).count();

    let hint_style = Style::default().fg(Color::DarkGray);
    let cursor_style = Style::default()
        .bg(Color::Cyan)
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD);
    let selected_style = Style::default().fg(Color::Green);
    let normal_style = Style::default().fg(Color::White);

    let mut lines: Vec<Line> = Vec::new();

    // Header hints
    lines.push(Line::from(vec![
        Span::styled("  [a]", Style::default().fg(Color::Cyan)),
        Span::styled(" Select all  ", hint_style),
        Span::styled("[n]", Style::default().fg(Color::Cyan)),
        Span::styled(" Select none  ", hint_style),
        Span::styled("[Space]", Style::default().fg(Color::Cyan)),
        Span::styled(" Toggle", hint_style),
    ]));
    lines.push(Line::from(""));

    // Items
    let scroll_end = (app.filter_menu_scroll + visible_items).min(app.filter_items.len());
    for idx in app.filter_menu_scroll..scroll_end {
        let (value, is_selected) = &app.filter_items[idx];
        let checkbox = if *is_selected { "[x]" } else { "[ ]" };
        let text = format!("  {} {}", checkbox, value);

        let style = if idx == app.filter_menu_cursor {
            cursor_style
        } else if *is_selected {
            selected_style
        } else {
            normal_style
        };

        lines.push(Line::from(Span::styled(text, style)));
    }

    // Pad remaining lines if fewer items than visible area
    let rendered_items = scroll_end - app.filter_menu_scroll;
    for _ in rendered_items..visible_items {
        lines.push(Line::from(""));
    }

    // Footer
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  [Enter]", Style::default().fg(Color::Cyan)),
        Span::styled(
            format!(" Apply ({} selected)  ", selected_count),
            hint_style,
        ),
        Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
        Span::styled(" Cancel", hint_style),
    ]));

    let text = Text::from(lines);
    let height = (chrome_lines + visible_items as u16 + 2).min(max_popup_height + 2);

    let popup = Paragraph::new(text).block(
        Block::bordered()
            .title(format!(" Filter: {} ", col_name))
            .style(Style::default().fg(Color::White)),
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
