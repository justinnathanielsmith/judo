use crate::app::state::{AppMode, AppState};
use crate::components::diff_view::DiffViewPanel;
use crate::components::footer::Footer;
use crate::components::header::Header;
use crate::components::modals::ModalManager;
use crate::components::revision_graph::RevisionGraphPanel;
use crate::components::welcome::Welcome;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

pub struct AppLayout {
    pub main: Vec<Rect>,
    pub body: Vec<Rect>,
}

pub fn get_layout(area: Rect, app_state: &AppState) -> AppLayout {
    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(0),    // Body
            Constraint::Length(1), // Footer
        ])
        .split(area)
        .to_vec();

    let body = if main.len() > 1 {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(if app_state.show_diffs {
                [
                    Constraint::Percentage(100u16.saturating_sub(app_state.diff_ratio)),
                    Constraint::Percentage(app_state.diff_ratio),
                ]
            } else {
                [Constraint::Percentage(100), Constraint::Percentage(0)]
            })
            .split(main[1])
            .to_vec()
    } else {
        vec![Rect::default(), Rect::default()]
    };

    AppLayout { main, body }
}

pub fn draw(f: &mut Frame, app_state: &mut AppState) {
    let theme = &app_state.theme;
    if app_state.mode == AppMode::NoRepo {
        let welcome = Welcome { app_state, theme };
        f.render_widget(welcome, f.area());
        return;
    }

    let layout = get_layout(f.area(), app_state);

    // --- Header ---
    let header = Header {
        state: &app_state.header_state,
        theme,
        terminal_width: f.area().width,
    };
    f.render_widget(header, layout.main[0]);

    // --- Left: Revision Graph Panel ---
    let panel = RevisionGraphPanel {
        repo: app_state.repo.as_ref(),
        theme,
        show_diffs: app_state.show_diffs,
        selected_file_index: app_state.log.selected_file_index,
        spinner: &app_state.spinner,
        focused_panel: app_state.focused_panel,
        mode: app_state.mode,
        revset: app_state.revset.as_deref(),
    };
    f.render_stateful_widget(panel, layout.body[0], &mut app_state.log.list_state);

    // --- Right: Diff View Panel ---
    if app_state.show_diffs {
        let panel = DiffViewPanel {
            diff_content: app_state.log.current_diff.as_deref(),
            scroll_offset: app_state.log.diff_scroll,
            theme,
            hunk_highlight_time: app_state.hunk_highlight_time,
            focused_panel: app_state.focused_panel,
            mode: app_state.mode,
        };
        f.render_widget(panel, layout.body[1]);
    }

    // --- Footer ---
    if layout.main.len() > 2 {
        let footer = Footer {
            state: app_state,
            theme,
        };
        f.render_widget(footer, layout.main[2]);
    }

    // --- Modals & Overlays ---
    f.render_widget(ModalManager { theme, app_state }, f.area());
}
