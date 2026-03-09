use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::{App, PromptState, View};

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

/// Handle keys in the Help view.
fn handle_help(app: &mut App, key: &KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            app.view = View::Table;
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
        let columns = vec![
            "name".to_string(),
            "age".to_string(),
            "score".to_string(),
        ];
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

        handle_event(&mut app, press_with(KeyCode::Backspace, KeyModifiers::CONTROL));
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
}
