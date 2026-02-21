use judo::app::state::{AppMode, AppState, ContextMenuState};
use judo::app::ui;
use judo::theme::Theme;
use ratatui::{backend::TestBackend, Terminal};

fn main() {
    let mut app_state = AppState::default();
    let theme = Theme::default();

    let modes = [
        AppMode::Normal,
        AppMode::Input,
        AppMode::BookmarkInput,
        AppMode::FilterInput,
        AppMode::ContextMenu,
    ];

    for &mode in &modes {
        app_state.mode = mode;
        if mode == AppMode::ContextMenu {
            app_state.context_menu = Some(ContextMenuState {
                commit_id: judo::domain::models::CommitId("test".to_string()),
                x: 10,
                y: 10,
                selected_index: 0,
                actions: vec![("Test Action".to_string(), judo::app::action::Action::Tick)],
            });
        }

        for width in 0..100 {
            for height in 0..50 {
                let backend = TestBackend::new(width, height);
                let mut terminal = Terminal::new(backend).unwrap();

                // Test with and without diffs
                app_state.show_diffs = false;
                let _ = terminal.draw(|f| {
                    ui::draw(f, &mut app_state, &theme);
                });

                app_state.show_diffs = true;
                let _ = terminal.draw(|f| {
                    ui::draw(f, &mut app_state, &theme);
                });
            }
        }
    }

    // Also test with an error message
    app_state.last_error = Some("Test error message that might be long and cause issues if not handled correctly by the layout engine.".to_string());
    for width in 0..100 {
        for height in 0..50 {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            let _ = terminal.draw(|f| {
                ui::draw(f, &mut app_state, &theme);
            });
        }
    }

    println!("Layout verification completed successfully!");
}
