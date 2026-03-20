use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::{App, PromptState, View};
use crate::geo;

/// Map crossterm key events to app state mutations.
pub fn handle_event(app: &mut App, event: Event) {
    let Event::Key(key) = event else {
        return;
    };

    // Only handle key presses, not releases or repeats.
    if key.kind != KeyEventKind::Press {
        return;
    }

    // Ctrl-c force quits from any state.
    if is_ctrl_c(&key) {
        app.should_quit = true;
        return;
    }

    // Dispatch based on current prompt / view.
    match app.prompt {
        PromptState::ConfirmDescribe | PromptState::ConfirmUniques => {
            handle_prompt(app, &key);
        }
        PromptState::None => match app.view {
            View::Table => handle_table(app, &key),
            View::Describe | View::Uniques => handle_stats(app, &key),
            View::Help => handle_help(app, &key),
            View::Value => handle_value(app, &key),
            View::FilterMenu => handle_filter_menu(app, &key),
        },
    }
}

/// Handle keys when a confirmation prompt is active.
fn handle_prompt(app: &mut App, key: &KeyEvent) {
    match key.code {
        KeyCode::Char('y') => {
            let view = match app.prompt {
                PromptState::ConfirmDescribe => View::Describe,
                PromptState::ConfirmUniques => View::Uniques,
                PromptState::None => unreachable!(),
            };
            app.prompt = PromptState::None;
            app.loading = true;
            app.view = view;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.prompt = PromptState::None;
        }
        _ => {}
    }
}

/// Handle keys in the Table view.
fn handle_table(app: &mut App, key: &KeyEvent) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    match key.code {
        // Vim movement: h/j/k/l
        KeyCode::Char('h') if !ctrl => app.cursor_left(1),
        KeyCode::Char('j') if !ctrl => app.cursor_down(1),
        KeyCode::Char('k') if !ctrl => app.cursor_up(1),
        KeyCode::Char('l') if !ctrl => app.cursor_right(1),

        // Ctrl-d / Ctrl-u — jump 20 rows
        KeyCode::Char('d') if ctrl => app.cursor_down(20),
        KeyCode::Char('u') if ctrl => app.cursor_up(20),

        // Ctrl-l — jump to last column
        KeyCode::Char('l') if ctrl => app.cursor_last_col(),

        // Ctrl-h — jump to first column (also handle Backspace with Ctrl)
        KeyCode::Char('h') if ctrl => app.cursor_first_col(),
        KeyCode::Backspace if ctrl => app.cursor_first_col(),

        // g / G — jump to top / bottom
        KeyCode::Char('g') if !shift => app.cursor_top(),
        KeyCode::Char('G') => app.cursor_bottom(),

        // d — describe prompt (only without Ctrl, which is handled above)
        KeyCode::Char('d') if !ctrl => {
            app.stats_column = app.current_column_name().to_string();
            app.prompt = PromptState::ConfirmDescribe;
        }

        // u — uniques prompt (only without Ctrl, which is handled above)
        KeyCode::Char('u') if !ctrl => {
            app.stats_column = app.current_column_name().to_string();
            app.prompt = PromptState::ConfirmUniques;
        }

        // Enter — value view (show full cell value)
        KeyCode::Enter => {
            app.cell_value = app.current_cell_value();
            app.value_scroll = 0;
            app.view = View::Value;
        }

        // s — sort current column
        KeyCode::Char('s') => {
            app.toggle_sort(app.cursor_col);
        }

        // f — filter current column
        KeyCode::Char('f') => {
            app.stats_column = app.current_column_name().to_string();
            app.loading = true;
            app.view = View::FilterMenu;
        }

        // c — clear all filters
        KeyCode::Char('c') => {
            app.clear_all_filters();
        }

        // ? — help view
        KeyCode::Char('?') => {
            app.view = View::Help;
        }

        // q — quit
        KeyCode::Char('q') => {
            app.should_quit = true;
        }

        _ => {}
    }
}

/// Handle keys in the stats views (Describe / Uniques).
fn handle_stats(app: &mut App, key: &KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.view = View::Table;
            app.stats_result = None;
        }
        _ => {}
    }
}

/// Handle keys in the Value view.
fn handle_value(app: &mut App, key: &KeyEvent) {
    // Any key clears a map error overlay
    if app.map_error.is_some() {
        app.map_error = None;
        return;
    }

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
            app.view = View::Table;
            app.value_scroll = 0;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.value_scroll = app.value_scroll.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.value_scroll = app.value_scroll.saturating_sub(1);
        }
        KeyCode::Char('m') => match geo::parse_geometry(&app.cell_value) {
            Ok(geom) => {
                if let Err(e) = geo::open_in_browser(&geom) {
                    app.map_error = Some(e);
                }
            }
            Err(e) => {
                app.map_error = Some(e);
            }
        },
        _ => {}
    }
}

/// Handle keys in the Help view.
fn handle_help(app: &mut App, key: &KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            app.view = View::Table;
        }
        _ => {}
    }
}

/// Handle keys in the Filter Menu view.
fn handle_filter_menu(app: &mut App, key: &KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.filter_menu_cursor < app.filter_items.len().saturating_sub(1) {
                app.filter_menu_cursor += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.filter_menu_cursor = app.filter_menu_cursor.saturating_sub(1);
        }
        KeyCode::Char(' ') => {
            // Toggle selection
            if let Some(item) = app.filter_items.get_mut(app.filter_menu_cursor) {
                item.1 = !item.1;
            }
        }
        KeyCode::Enter => {
            // Apply filter
            let selected: Vec<String> = app
                .filter_items
                .iter()
                .filter(|(_, sel)| *sel)
                .map(|(val, _)| val.clone())
                .collect();
            app.set_filter(app.stats_column.clone(), selected);
            app.view = View::Table;
            app.filter_items.clear();
            app.filter_menu_cursor = 0;
            app.filter_menu_scroll = 0;
        }
        KeyCode::Esc => {
            // Cancel
            app.view = View::Table;
            app.filter_items.clear();
            app.filter_menu_cursor = 0;
            app.filter_menu_scroll = 0;
        }
        KeyCode::Char('a') => {
            // Select all
            for item in &mut app.filter_items {
                item.1 = true;
            }
        }
        KeyCode::Char('n') => {
            // Select none
            for item in &mut app.filter_items {
                item.1 = false;
            }
        }
        _ => {}
    }
}

/// Check if the key event is Ctrl-c.
fn is_ctrl_c(key: &KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c')
}

#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::df;

    fn make_test_app() -> App {
        let data = df! {
            "name" => &["alice", "bob", "carol", "dave", "eve"],
            "age"  => &[30, 25, 35, 40, 22],
            "score" => &[95.5, 87.3, 91.0, 78.2, 99.1],
        }
        .unwrap();
        let columns = vec!["name".to_string(), "age".to_string(), "score".to_string()];
        let dtypes = vec![
            "String".to_string(),
            "Int32".to_string(),
            "Float64".to_string(),
        ];
        App::new(data, columns, dtypes)
    }

    fn press(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn press_with(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    fn press_shift(code: KeyCode) -> Event {
        press_with(code, KeyModifiers::SHIFT)
    }

    fn press_ctrl(code: KeyCode) -> Event {
        press_with(code, KeyModifiers::CONTROL)
    }

    // --- Table view movement ---

    #[test]
    fn test_hjkl_movement() {
        let mut app = make_test_app();
        // Start at (0, 0)

        handle_event(&mut app, press(KeyCode::Char('j')));
        assert_eq!(app.cursor_row, 1);

        handle_event(&mut app, press(KeyCode::Char('k')));
        assert_eq!(app.cursor_row, 0);

        handle_event(&mut app, press(KeyCode::Char('l')));
        assert_eq!(app.cursor_col, 1);

        handle_event(&mut app, press(KeyCode::Char('h')));
        assert_eq!(app.cursor_col, 0);
    }

    #[test]
    fn test_ctrl_d_u_jump() {
        let mut app = make_test_app();

        handle_event(&mut app, press_ctrl(KeyCode::Char('d')));
        // 5 rows, so clamped to row 4
        assert_eq!(app.cursor_row, 4);

        handle_event(&mut app, press_ctrl(KeyCode::Char('u')));
        assert_eq!(app.cursor_row, 0);
    }

    #[test]
    fn test_ctrl_l_h_column_jump() {
        let mut app = make_test_app();

        handle_event(&mut app, press_ctrl(KeyCode::Char('l')));
        assert_eq!(app.cursor_col, 2);

        handle_event(&mut app, press_ctrl(KeyCode::Char('h')));
        assert_eq!(app.cursor_col, 0);
    }

    #[test]
    fn test_ctrl_backspace_first_col() {
        let mut app = make_test_app();
        app.cursor_col = 2;

        handle_event(
            &mut app,
            press_with(KeyCode::Backspace, KeyModifiers::CONTROL),
        );
        assert_eq!(app.cursor_col, 0);
    }

    #[test]
    fn test_g_and_shift_g() {
        let mut app = make_test_app();

        handle_event(&mut app, press_shift(KeyCode::Char('G')));
        assert_eq!(app.cursor_row, 4);

        handle_event(&mut app, press(KeyCode::Char('g')));
        assert_eq!(app.cursor_row, 0);
    }

    #[test]
    fn test_quit() {
        let mut app = make_test_app();
        handle_event(&mut app, press(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn test_ctrl_c_force_quit() {
        let mut app = make_test_app();
        app.view = View::Help; // from any view
        handle_event(&mut app, press_ctrl(KeyCode::Char('c')));
        assert!(app.should_quit);
    }

    // --- Prompt ---

    #[test]
    fn test_describe_prompt_flow() {
        let mut app = make_test_app();
        app.cursor_col = 1; // "age"

        // Press 'd' to trigger describe prompt
        handle_event(&mut app, press(KeyCode::Char('d')));
        assert_eq!(app.prompt, PromptState::ConfirmDescribe);
        assert_eq!(app.stats_column, "age");

        // Confirm with 'y'
        handle_event(&mut app, press(KeyCode::Char('y')));
        assert_eq!(app.prompt, PromptState::None);
        assert!(app.loading);
        assert_eq!(app.view, View::Describe);
    }

    #[test]
    fn test_uniques_prompt_cancel() {
        let mut app = make_test_app();

        handle_event(&mut app, press(KeyCode::Char('u')));
        assert_eq!(app.prompt, PromptState::ConfirmUniques);
        assert_eq!(app.stats_column, "name");

        // Cancel with 'n'
        handle_event(&mut app, press(KeyCode::Char('n')));
        assert_eq!(app.prompt, PromptState::None);
        assert_eq!(app.view, View::Table);
    }

    #[test]
    fn test_prompt_cancel_with_esc() {
        let mut app = make_test_app();

        handle_event(&mut app, press(KeyCode::Char('d')));
        assert_eq!(app.prompt, PromptState::ConfirmDescribe);

        handle_event(&mut app, press(KeyCode::Esc));
        assert_eq!(app.prompt, PromptState::None);
    }

    // --- Stats view ---

    #[test]
    fn test_stats_view_back_to_table() {
        let mut app = make_test_app();
        app.view = View::Describe;
        app.stats_result = Some(make_test_app().data.clone());

        handle_event(&mut app, press(KeyCode::Char('q')));
        assert_eq!(app.view, View::Table);
        assert!(app.stats_result.is_none());
    }

    #[test]
    fn test_stats_view_esc_back() {
        let mut app = make_test_app();
        app.view = View::Uniques;

        handle_event(&mut app, press(KeyCode::Esc));
        assert_eq!(app.view, View::Table);
    }

    // --- Help view ---

    #[test]
    fn test_help_view_open_close() {
        let mut app = make_test_app();

        handle_event(&mut app, press(KeyCode::Char('?')));
        assert_eq!(app.view, View::Help);

        handle_event(&mut app, press(KeyCode::Char('?')));
        assert_eq!(app.view, View::Table);
    }

    #[test]
    fn test_help_view_esc() {
        let mut app = make_test_app();
        app.view = View::Help;

        handle_event(&mut app, press(KeyCode::Esc));
        assert_eq!(app.view, View::Table);
    }

    // --- Non-press events are ignored ---

    #[test]
    fn test_release_event_ignored() {
        let mut app = make_test_app();
        let release = Event::Key(KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: crossterm::event::KeyEventState::NONE,
        });
        handle_event(&mut app, release);
        assert!(!app.should_quit);
    }

    // --- Sort ---

    #[test]
    fn test_sort_keybinding() {
        let mut app = make_test_app();
        app.cursor_col = 1; // "age"

        handle_event(&mut app, press(KeyCode::Char('s')));
        assert_eq!(app.sort_col, Some(1));
        assert!(!app.sort_desc);
        assert!(app.needs_refresh);
    }

    // --- Filter ---

    #[test]
    fn test_filter_keybinding_opens_menu() {
        let mut app = make_test_app();
        app.cursor_col = 0; // "name"

        handle_event(&mut app, press(KeyCode::Char('f')));
        assert_eq!(app.view, View::FilterMenu);
        assert!(app.loading);
        assert_eq!(app.stats_column, "name");
    }

    #[test]
    fn test_clear_filters_keybinding() {
        let mut app = make_test_app();
        app.set_filter("name".to_string(), vec!["alice".to_string()]);
        app.needs_refresh = false;

        handle_event(&mut app, press(KeyCode::Char('c')));
        assert!(app.filters.is_empty());
        assert!(app.needs_refresh);
    }

    // --- Filter menu ---

    #[test]
    fn test_filter_menu_navigation() {
        let mut app = make_test_app();
        app.view = View::FilterMenu;
        app.filter_items = vec![
            ("alice".to_string(), false),
            ("bob".to_string(), false),
            ("carol".to_string(), false),
        ];

        // Move down
        handle_event(&mut app, press(KeyCode::Char('j')));
        assert_eq!(app.filter_menu_cursor, 1);

        // Move down again
        handle_event(&mut app, press(KeyCode::Char('j')));
        assert_eq!(app.filter_menu_cursor, 2);

        // Move up
        handle_event(&mut app, press(KeyCode::Char('k')));
        assert_eq!(app.filter_menu_cursor, 1);
    }

    #[test]
    fn test_filter_menu_toggle_and_apply() {
        let mut app = make_test_app();
        app.view = View::FilterMenu;
        app.stats_column = "name".to_string();
        app.filter_items = vec![("alice".to_string(), false), ("bob".to_string(), false)];

        // Toggle first item
        handle_event(&mut app, press(KeyCode::Char(' ')));
        assert!(app.filter_items[0].1);
        assert!(!app.filter_items[1].1);

        // Apply
        handle_event(&mut app, press(KeyCode::Enter));
        assert_eq!(app.view, View::Table);
        assert_eq!(app.filters.len(), 1);
        assert_eq!(app.filters[0].1, vec!["alice".to_string()]);
        assert!(app.filter_items.is_empty());
    }

    #[test]
    fn test_filter_menu_select_all_none() {
        let mut app = make_test_app();
        app.view = View::FilterMenu;
        app.filter_items = vec![("alice".to_string(), false), ("bob".to_string(), false)];

        // Select all
        handle_event(&mut app, press(KeyCode::Char('a')));
        assert!(app.filter_items[0].1);
        assert!(app.filter_items[1].1);

        // Select none
        handle_event(&mut app, press(KeyCode::Char('n')));
        assert!(!app.filter_items[0].1);
        assert!(!app.filter_items[1].1);
    }

    #[test]
    fn test_filter_menu_cancel() {
        let mut app = make_test_app();
        app.view = View::FilterMenu;
        app.filter_items = vec![("alice".to_string(), true)];

        handle_event(&mut app, press(KeyCode::Esc));
        assert_eq!(app.view, View::Table);
        assert!(app.filter_items.is_empty());
    }
}
