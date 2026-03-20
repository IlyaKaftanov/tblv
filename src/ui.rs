mod filter_menu;
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
        View::Table | View::Value | View::FilterMenu => table_view::render(frame, app),
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

    // Filter menu renders as overlay on top of table.
    if app.view == View::FilterMenu {
        filter_menu::render(frame, app);
    }

    // Map parse error renders as small overlay in value view.
    if app.view == View::Value
        && let Some(ref err) = app.map_error
    {
        value_view::render_error(frame, err);
    }
}
