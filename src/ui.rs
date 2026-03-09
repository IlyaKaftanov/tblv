mod help;
mod prompt;
mod stats_view;
mod table_view;
mod value_view;

use crate::app::{App, PromptState, View};
use ratatui::Frame;

/// Main render function — dispatches to the appropriate view.
pub fn render(frame: &mut Frame, app: &mut App) {
    match app.view {
        View::Table | View::Value => table_view::render(frame, app),
        View::Describe | View::Uniques => stats_view::render(frame, app),
        View::Help => help::render(frame, app),
    }

    // Overlay prompt on top of current view if active.
    if app.prompt != PromptState::None {
        prompt::render(frame, app);
    }

    // Value view renders as overlay on top of table.
    if app.view == View::Value {
        value_view::render(frame, app);
    }
}
