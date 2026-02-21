use crate::app::state::{AppMode, AppState};
use crate::components::diff_view::DiffViewPanel;
use crate::components::footer::Footer;
use crate::components::header::Header;
use crate::components::modals::ModalManager;
use crate::components::revision_graph::RevisionGraphPanel;
use crate::components::welcome::Welcome;
use crate::domain::models::GraphRow;
use crate::theme::Theme;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

pub fn calculate_row_height(row: &GraphRow, is_selected: bool, show_diffs: bool) -> u16 {
    let num_files = if is_selected && show_diffs {
        row.changed_files.len()
    } else {
        0
    };
    2 + num_files as u16
}

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

pub fn draw(f: &mut Frame, app_state: &mut AppState, theme: &Theme) {
    if f.area().width == 0 || f.area().height == 0 {
        return;
    }

    if app_state.mode == AppMode::NoRepo {
        let welcome = Welcome { app_state, theme };
        f.render_widget(welcome, f.area());
        return;
    }

    let layout = get_layout(f.area(), app_state);

    // --- Header ---
    if layout.main[0].width > 0 && layout.main[0].height > 0 {
        let header = Header {
            state: &app_state.header_state,
            theme,
            terminal_width: f.area().width,
        };
        f.render_widget(header, layout.main[0]);
    }

    // --- Left: Revision Graph Panel ---
    if layout.body[0].width > 0 && layout.body[0].height > 0 {
        let panel = RevisionGraphPanel {
            repo: app_state.repo.as_ref(),
            theme,
            show_diffs: app_state.show_diffs,
            selected_file_index: app_state.selected_file_index,
            spinner: &app_state.spinner,
            focused_panel: app_state.focused_panel,
            mode: app_state.mode,
            revset: app_state.revset.as_deref(),
        };
        f.render_stateful_widget(panel, layout.body[0], &mut app_state.log_list_state);
    }

    // --- Right: Diff View Panel ---
    if app_state.show_diffs && layout.body[1].width > 0 && layout.body[1].height > 0 {
        let panel = DiffViewPanel {
            diff_content: app_state.current_diff.as_deref(),
            scroll_offset: app_state.diff_scroll,
            theme,
            hunk_highlight_time: app_state.hunk_highlight_time,
            focused_panel: app_state.focused_panel,
            mode: app_state.mode,
        };
        f.render_widget(panel, layout.body[1]);
    }

    // --- Footer ---
    if layout.main.len() > 2 && layout.main[2].width > 0 && layout.main[2].height > 0 {
        let footer = Footer {
            state: app_state,
            theme,
        };
        f.render_widget(footer, layout.main[2]);
    }

    // --- Modals & Overlays ---
    f.render_widget(ModalManager { theme, app_state }, f.area());
}
