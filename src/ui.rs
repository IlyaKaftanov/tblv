mod table_view;
mod stats_view;
mod prompt;
mod help;

use ratatui::Frame;
use crate::app::{App, PromptState, View};

/// Main render function — dispatches to the appropriate view.
pub fn render(frame: &mut Frame, app: &mut App) {
    match app.view {
        View::Table => table_view::render(frame, app),
        View::Describe | View::Uniques => stats_view::render(frame, app),
        View::Help => help::render(frame, app),
    }

    // Overlay prompt on top of current view if active.
    if app.prompt != PromptState::None {
        prompt::render(frame, app);
    }
}
