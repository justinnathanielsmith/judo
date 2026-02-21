use crate::app::state::{AppMode, AppState};
use crate::components::diff_view::DiffViewPanel;
use crate::components::footer::Footer;
use crate::components::header::Header;
use crate::components::revision_graph::RevisionGraphPanel;
use crate::components::welcome::Welcome;
use crate::domain::models::GraphRow;
use crate::theme::Theme;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Row, Table},
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

    // --- Visual Dimming ---
    let is_modal_active = !matches!(
        app_state.mode,
        AppMode::Normal | AppMode::Diff | AppMode::NoRepo | AppMode::Loading
    ) || app_state.last_error.is_some();

    if is_modal_active {
        dim_area(f, f.area());
    }

    // --- Modals ---
    if app_state.mode == AppMode::Help && f.area().width > 0 && f.area().height > 0 {
        draw_help(f, theme);
    }

    // --- Input Modal ---
    if (app_state.mode == AppMode::Input || app_state.mode == AppMode::BookmarkInput)
        && f.area().width > 0
        && f.area().height > 0
    {
        let area = centered_rect(60, 20, f.area());
        if area.width > 0 && area.height > 0 {
            draw_drop_shadow(f, area);
            f.render_widget(Clear, area);
            let title = if app_state.mode == AppMode::BookmarkInput {
                " SET BOOKMARK "
            } else {
                " DESCRIBE REVISION "
            };
            let block = Block::default()
                .title(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(title, theme.header_active),
                    Span::raw(" "),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.border_focus);

            let inner_area = block.inner(area);
            let padded_area = Rect {
                x: inner_area.x + 1,
                y: inner_area.y + 1,
                width: inner_area.width.saturating_sub(2),
                height: inner_area.height.saturating_sub(2),
            };

            f.render_widget(block, area);
            app_state.text_area.set_block(Block::default());
            if padded_area.width > 0 && padded_area.height > 0 {
                f.render_widget(&app_state.text_area, padded_area);
            }
        }
    }

    // --- Filter Input Modal ---
    if app_state.mode == AppMode::FilterInput && f.area().width > 0 && f.area().height > 0 {
        let area = centered_rect_fixed_height(60, 3, f.area());
        if area.width > 0 && area.height > 0 {
            draw_drop_shadow(f, area);
            f.render_widget(Clear, area);
            let block = Block::default()
                .title(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(" FILTER (REVSET) ", theme.header_active),
                    Span::raw(" "),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme.border_focus);

            let inner_area = block.inner(area);
            f.render_widget(block, area);
            app_state.text_area.set_block(Block::default());
            if inner_area.width > 0 && inner_area.height > 0 {
                f.render_widget(&app_state.text_area, inner_area);
            }
        }
    }

    // --- Context Menu Popup ---
    if let (AppMode::ContextMenu, Some(menu)) = (app_state.mode, &app_state.context_menu) {
        if f.area().width > 0 && f.area().height > 0 {
            let area = menu.calculate_rect(f.area());
            if area.width > 0 && area.height > 0 {
                draw_drop_shadow(f, area);
                f.render_widget(Clear, area);

                let items: Vec<ListItem> = menu
                    .actions
                    .iter()
                    .enumerate()
                    .map(|(i, (name, _))| {
                        if i == menu.selected_index {
                            ListItem::new(format!("> {}", name)).style(theme.list_selected)
                        } else {
                            ListItem::new(format!("  {}", name)).style(theme.list_item)
                        }
                    })
                    .collect();

                let list = List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(theme.border_focus),
                );
                f.render_widget(list, area);
            }
        }
    }

    // --- Error Modal ---
    if let Some(err) = &app_state.last_error {
        if f.area().width > 0 && f.area().height > 0 {
            let area = centered_rect(60, 20, f.area());
            if area.width > 0 && area.height > 0 {
                draw_drop_shadow(f, area);
                f.render_widget(Clear, area);
                let block = Block::default()
                    .title(Line::from(vec![
                        Span::raw(" "),
                        Span::styled(" ERROR ", theme.status_error),
                        Span::raw(" "),
                    ]))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(theme.status_error);

                let text_lines = vec![
                    Line::from(""),
                    Line::from(Span::styled(err.clone(), theme.footer_segment_val)),
                    Line::from(""),
                    Line::from(vec![
                        Span::raw(" Press "),
                        Span::styled("Esc", theme.footer_segment_key),
                        Span::raw(" to acknowledge "),
                    ]),
                ];

                let paragraph = Paragraph::new(text_lines)
                    .alignment(ratatui::layout::Alignment::Center)
                    .block(block);

                f.render_widget(paragraph, area);
            }
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(100u16.saturating_sub(percent_y) / 2),
            Constraint::Percentage(percent_y.min(100)),
            Constraint::Percentage(100u16.saturating_sub(percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(100u16.saturating_sub(percent_x) / 2),
            Constraint::Percentage(percent_x.min(100)),
            Constraint::Percentage(100u16.saturating_sub(percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn centered_rect_fixed_height(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height.min(r.height)),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(100u16.saturating_sub(percent_x) / 2),
            Constraint::Percentage(percent_x.min(100)),
            Constraint::Percentage(100u16.saturating_sub(percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_help(f: &mut Frame, theme: &Theme) {
    let area = f.area();
    let help_area = centered_rect(70, 80, area);
    if help_area.width == 0 || help_area.height == 0 {
        return;
    }
    draw_drop_shadow(f, help_area);
    f.render_widget(Clear, help_area);

    let block = Block::default()
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(" HELP - KEYBINDINGS ", theme.header_active),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.border_focus);

    use ratatui::widgets::Cell;

    let key_style = theme.footer_segment_key;
    let desc_style = theme.list_item;
    let category_style = theme.header_item;

    let rows = vec![
        // Navigation
        Row::new(vec![
            Cell::from(Span::styled("Navigation", category_style)),
            Cell::from(""),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" j / ↓", key_style)),
            Cell::from(Span::styled("Select next revision", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" k / ↑", key_style)),
            Cell::from(Span::styled("Select previous revision", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" Enter", key_style)),
            Cell::from(Span::styled("Toggle diff panel", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" Tab / l", key_style)),
            Cell::from(Span::styled("Focus diff panel", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" h", key_style)),
            Cell::from(Span::styled("Focus revision graph", desc_style)),
        ]),
        Row::new(vec![Cell::from(""), Cell::from("")]),
        // Operations
        Row::new(vec![
            Cell::from(Span::styled("Operations", category_style)),
            Cell::from(""),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" s", key_style)),
            Cell::from(Span::styled("Snapshot working copy", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" e", key_style)),
            Cell::from(Span::styled("Edit selected revision", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" n", key_style)),
            Cell::from(Span::styled("Create new child revision", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" d", key_style)),
            Cell::from(Span::styled("Describe revision", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" a", key_style)),
            Cell::from(Span::styled("Abandon revision", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" S", key_style)),
            Cell::from(Span::styled("Squash into parent", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" b", key_style)),
            Cell::from(Span::styled("Set bookmark", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" B", key_style)),
            Cell::from(Span::styled("Delete bookmark", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" u / U", key_style)),
            Cell::from(Span::styled("Undo / Redo", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" f / p", key_style)),
            Cell::from(Span::styled("Fetch / Push", desc_style)),
        ]),
        Row::new(vec![Cell::from(""), Cell::from("")]),
        // Filtering
        Row::new(vec![
            Cell::from(Span::styled("Filtering", category_style)),
            Cell::from(""),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" /", key_style)),
            Cell::from(Span::styled("Custom revset filter", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" m", key_style)),
            Cell::from(Span::styled("Filter: mine()", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" t", key_style)),
            Cell::from(Span::styled("Filter: trunk()", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" c", key_style)),
            Cell::from(Span::styled("Filter: conflicts()", desc_style)),
        ]),
        Row::new(vec![Cell::from(""), Cell::from("")]),
        // General
        Row::new(vec![
            Cell::from(Span::styled("General", category_style)),
            Cell::from(""),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" ?", key_style)),
            Cell::from(Span::styled("Show this help", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" Esc", key_style)),
            Cell::from(Span::styled("Close modal / Clear errors", desc_style)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(" q", key_style)),
            Cell::from(Span::styled("Quit", desc_style)),
        ]),
    ];

    let table = Table::new(
        rows,
        [Constraint::Percentage(30), Constraint::Percentage(70)],
    )
    .block(block);

    f.render_widget(table, help_area);
}

fn dim_area(f: &mut Frame, area: Rect) {
    let buffer = f.buffer_mut();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buffer[(x, y)];
            cell.set_style(cell.style().add_modifier(ratatui::style::Modifier::DIM));
        }
    }
}

fn draw_drop_shadow(f: &mut Frame, area: Rect) {
    let shadow_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width,
        height: area.height,
    };

    let terminal_area = f.area();
    let shadow_area = shadow_area.intersection(terminal_area);

    let buffer = f.buffer_mut();
    for y in shadow_area.top()..shadow_area.bottom() {
        for x in shadow_area.left()..shadow_area.right() {
            let cell = &mut buffer[(x, y)];
            cell.set_style(ratatui::style::Style::default().bg(ratatui::style::Color::Black));
            cell.set_symbol(" ");
        }
    }
}
