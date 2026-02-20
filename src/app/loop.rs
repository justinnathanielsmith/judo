use crate::app::{
    action::Action,
    command::Command,
    reducer,
    state::{AppMode, AppState},
};
use crate::components::diff_view::DiffView;
use crate::components::revision_graph::RevisionGraph;
use crate::domain::vcs::VcsFacade;
use crate::theme::Theme;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, MouseButton, MouseEventKind};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
    Terminal,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;

const TICK_RATE: Duration = Duration::from_millis(250);

pub async fn run_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app_state: AppState<'_>,
    adapter: Arc<dyn VcsFacade>,
) -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::channel(100);
    let mut interval = interval(TICK_RATE);
    let theme = Theme::default();

    // User input channel
    let (event_tx, mut event_rx) = mpsc::channel(100);
    tokio::task::spawn_blocking(move || loop {
        match event::read() {
            Ok(evt) => {
                if event_tx.blocking_send(Ok(evt)).is_err() {
                    break;
                }
            }
            Err(e) => {
                let _ = event_tx.blocking_send(Err(e));
                break;
            }
        }
    });

    // Initial fetch
    handle_command(Command::LoadRepo, adapter.clone(), action_tx.clone()).await?;

    loop {
        // --- 1. Render ---
        terminal.draw(|f| {
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Header
                    Constraint::Min(0),    // Body
                    Constraint::Length(1), // Footer
                ])
                .split(f.area());

            // --- Header ---
            let (op_id, wc_info, stats) = if let Some(repo) = &app_state.repo {
                let mutable_count = repo.graph.iter().filter(|r| !r.is_immutable).count();
                let immutable_count = repo.graph.iter().filter(|r| r.is_immutable).count();
                (
                    &repo.operation_id[..8.min(repo.operation_id.len())],
                    format!(
                        " WC: {} ",
                        &repo.working_copy_id.0[..8.min(repo.working_copy_id.0.len())]
                    ),
                    format!(" | Mut: {} Imm: {} ", mutable_count, immutable_count),
                )
            } else {
                ("........", " Loading... ".to_string(), "".to_string())
            };

            let header = Paragraph::new(Line::from(vec![
                Span::styled(" JUDO ", theme.header_logo),
                Span::styled(format!(" Op: {} ", op_id), theme.header_item),
                Span::styled(wc_info, theme.header_item),
                Span::styled(stats, theme.header),
                Span::styled(" ".repeat(f.area().width as usize), theme.header),
            ]))
            .style(theme.header);
            f.render_widget(header, main_chunks[0]);

            // --- Body ---
            let body_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(if app_state.show_diffs {
                    [Constraint::Percentage(50), Constraint::Percentage(50)]
                } else {
                    [Constraint::Percentage(100), Constraint::Percentage(0)]
                })
                .split(main_chunks[1]);

            // Left: Revision Graph
            let graph_border = if app_state.mode == crate::app::state::AppMode::Normal {
                theme.border_focus
            } else {
                theme.border
            };
            let graph_block = Block::default()
                .title(Line::from(vec![
                    Span::raw(" "),
                    Span::styled("REVISION GRAPH", theme.header_logo),
                    Span::raw(" "),
                ]))
                .title_bottom(Line::from(vec![
                    Span::raw(" "),
                    Span::styled("j/k", theme.footer_segment_key),
                    Span::raw(": navigate "),
                    Span::styled("d", theme.footer_segment_key),
                    Span::raw(": describe "),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(graph_border);

            if let Some(repo) = &app_state.repo {
                let graph = RevisionGraph {
                    repo,
                    theme: &theme,
                    show_diffs: app_state.show_diffs,
                };
                f.render_stateful_widget(
                    graph,
                    graph_block.inner(body_chunks[0]),
                    &mut app_state.log_list_state,
                );
            } else {
                let spin_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                let spinner =
                    spin_chars[(app_state.frame_count % spin_chars.len() as u64) as usize];

                let logo_ascii = vec![
                    "   _ _   _ ___   ___ ",
                    "  | | | | |   \\ / _ \\",
                    " _| | |_| | |) | (_) |",
                    "|___|_____|___/ \\___/ ",
                ];

                let mut lines: Vec<Line> = logo_ascii
                    .iter()
                    .map(|l| Line::from(Span::styled(*l, theme.header_logo)))
                    .collect();
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled(spinner, theme.header_logo),
                    Span::raw(" Loading Jujutsu Repository... "),
                ]));

                let loading = Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center);

                let area = body_chunks[0];
                let logo_height = 6;
                let centered_area = Rect {
                    x: area.x,
                    y: area.y + area.height / 2 - (logo_height / 2),
                    width: area.width,
                    height: logo_height as u16,
                };
                f.render_widget(loading, centered_area);
            }
            f.render_widget(graph_block, body_chunks[0]);

            // Right: Diff View
            if app_state.show_diffs {
                let diff_border = if app_state.mode == crate::app::state::AppMode::Diff {
                    theme.border_focus
                } else {
                    theme.border
                };
                let diff_block = Block::default()
                    .title(Line::from(vec![
                        Span::raw(" "),
                        Span::styled("DIFF VIEW", theme.header_logo),
                        Span::raw(" "),
                    ]))
                    .title_bottom(Line::from(vec![
                        Span::raw(" "),
                        Span::styled("PgUp/PgDn", theme.footer_segment_key),
                        Span::raw(": scroll "),
                        Span::styled("[/]", theme.footer_segment_key),
                        Span::raw(": hunks "),
                    ]))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(diff_border);

                let diff_view = DiffView {
                    diff_content: app_state.current_diff.as_deref(),
                    scroll_offset: app_state.diff_scroll,
                    theme: &theme,
                };
                f.render_widget(diff_view, diff_block.inner(body_chunks[1]));
                f.render_widget(diff_block, body_chunks[1]);
            }

            // --- Footer ---
            let status_content = if let Some(err) = &app_state.last_error {
                Span::styled(format!(" Error: {} ", err), theme.status_error)
            } else if let Some(msg) = &app_state.status_message {
                Span::styled(format!(" {} ", msg), theme.status_info)
            } else {
                Span::styled(" Ready ", theme.status_ready)
            };

            let help_legend = Line::from(vec![
                status_content,
                Span::styled(" ENTER ", theme.footer_segment_key),
                Span::styled(" toggle diff ", theme.footer_segment_val),
                Span::styled(" j/k ", theme.footer_segment_key),
                Span::styled(" move ", theme.footer_segment_val),
                Span::styled(" d ", theme.footer_segment_key),
                Span::styled(" desc ", theme.footer_segment_val),
                Span::styled(" n ", theme.footer_segment_key),
                Span::styled(" new ", theme.footer_segment_val),
                Span::styled(" a ", theme.footer_segment_key),
                Span::styled(" abdn ", theme.footer_segment_val),
                Span::styled(" b ", theme.footer_segment_key),
                Span::styled(" bkmk ", theme.footer_segment_val),
                Span::styled(" u/U ", theme.footer_segment_key),
                Span::styled(" undo/redo ", theme.footer_segment_val),
                Span::styled(" q ", theme.footer_segment_key),
                Span::styled(" quit ", theme.footer_segment_val),
            ]);
            let footer = Paragraph::new(help_legend).style(theme.footer);
            f.render_widget(footer, main_chunks[2]);

            // --- Input Modal ---
            if app_state.mode == crate::app::state::AppMode::Input
                || app_state.mode == crate::app::state::AppMode::BookmarkInput
            {
                let area = centered_rect(60, 20, f.area());
                f.render_widget(Clear, area);
                let title = if app_state.mode == crate::app::state::AppMode::BookmarkInput {
                    " Set Bookmark "
                } else {
                    " Describe Revision "
                };
                let block = Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(theme.border_focus);
                app_state.text_area.set_block(block);
                f.render_widget(&app_state.text_area, area);
            }

            // --- Context Menu Popup ---
            if let (AppMode::ContextMenu, Some(menu)) = (app_state.mode, &app_state.context_menu) {
                let menu_width = 20;
                let menu_height = menu.actions.len() as u16 + 2;

                // Position adjustment to keep menu on screen
                let mut x = menu.x;
                let mut y = menu.y;
                if x + menu_width > f.area().width {
                    x = f.area().width.saturating_sub(menu_width);
                }
                if y + menu_height > f.area().height {
                    y = f.area().height.saturating_sub(menu_height);
                }

                let area = ratatui::layout::Rect::new(x, y, menu_width, menu_height);
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
        })?;

        // --- 2. Event Handling (TEA Runtime) ---
        let action = tokio::select! {
            _ = interval.tick() => Some(Action::Tick),

            // User Input
            Some(res) = event_rx.recv() => {
                let event = match res {
                    Ok(e) => e,
                    Err(e) => return Err(e.into()),
                };
                match app_state.mode {
                    crate::app::state::AppMode::Input | crate::app::state::AppMode::BookmarkInput => {
                        match event {
                            Event::Key(key) => {
                                match key.code {
                                    KeyCode::Esc => Some(Action::CancelMode),
                                    KeyCode::Enter => {
                                        if let (Some(repo), Some(idx)) = (&app_state.repo, app_state.log_list_state.selected()) {
                                            if let Some(row) = repo.graph.get(idx) {
                                                if app_state.mode == crate::app::state::AppMode::BookmarkInput {
                                                    Some(Action::SetBookmark(row.commit_id.clone(), app_state.text_area.lines().join("")))
                                                } else {
                                                    Some(Action::DescribeRevision(row.commit_id.clone(), app_state.text_area.lines().join("\n")))
                                                }
                                            } else { None }
                                        } else { None }
                                    },
                                    _ => {
                                        app_state.text_area.input(key);
                                        None
                                    }
                                }
                            },
                            _ => None,
                        }
                    },
                    crate::app::state::AppMode::ContextMenu => {
                        match event {
                            Event::Key(key) => {
                                match key.code {
                                    KeyCode::Esc => Some(Action::CloseContextMenu),
                                    KeyCode::Char('j') | KeyCode::Down => Some(Action::SelectContextMenuNext),
                                    KeyCode::Char('k') | KeyCode::Up => Some(Action::SelectContextMenuPrev),
                                    KeyCode::Enter => {
                                        if let Some(menu) = &app_state.context_menu {
                                            Some(Action::SelectContextMenuAction(menu.selected_index))
                                        } else { None }
                                    },
                                    _ => None,
                                }
                            },
                            Event::Mouse(mouse) => {
                                match mouse.kind {
                                    MouseEventKind::Down(MouseButton::Left) => {
                                        if let Some(menu) = &app_state.context_menu {
                                            let menu_width = 20;
                                            let menu_height = menu.actions.len() as u16 + 2;

                                            let mut x = menu.x;
                                            let mut y = menu.y;
                                            if x + menu_width > terminal.size()?.width {
                                                x = terminal.size()?.width.saturating_sub(menu_width);
                                            }
                                            if y + menu_height > terminal.size()?.height {
                                                y = terminal.size()?.height.saturating_sub(menu_height);
                                            }

                                            if mouse.column >= x && mouse.column < x + menu_width
                                                && mouse.row >= y + 1 && mouse.row < y + menu_height - 1
                                            {
                                                let clicked_idx = (mouse.row - (y + 1)) as usize;
                                                Some(Action::SelectContextMenuAction(clicked_idx))
                                            } else {
                                                Some(Action::CloseContextMenu)
                                            }
                                        } else {
                                            Some(Action::CloseContextMenu)
                                        }
                                    },
                                    _ => None,
                                }
                            }
                            _ => None,
                        }
                    },
                    crate::app::state::AppMode::Diff => {
                        match event {
                            Event::Key(key) => {
                                match key.code {
                                    KeyCode::Esc | KeyCode::Char('q') => Some(Action::Quit),
                                    KeyCode::Char('h') | KeyCode::Tab => Some(Action::FocusGraph),
                                    KeyCode::Down | KeyCode::Char('j') => Some(Action::ScrollDiffDown(1)),
                                    KeyCode::Up | KeyCode::Char('k') => Some(Action::ScrollDiffUp(1)),
                                    KeyCode::PageDown => Some(Action::ScrollDiffDown(10)),
                                    KeyCode::PageUp => Some(Action::ScrollDiffUp(10)),
                                    KeyCode::Char('[') => Some(Action::PrevHunk),
                                    KeyCode::Char(']') => Some(Action::NextHunk),
                                    _ => None,
                                }
                            }
                            Event::Mouse(mouse) => {
                                match mouse.kind {
                                    MouseEventKind::ScrollUp => Some(Action::ScrollDiffUp(1)),
                                    MouseEventKind::ScrollDown => Some(Action::ScrollDiffDown(1)),
                                    MouseEventKind::Down(MouseButton::Left) => {
                                        let now = Instant::now();
                                        let is_double_click = app_state.last_click_time.map_or(false, |t| now.duration_since(t).as_millis() < 500)
                                            && app_state.last_click_pos == Some((mouse.column, mouse.row));
                                        app_state.last_click_time = Some(now);
                                        app_state.last_click_pos = Some((mouse.column, mouse.row));

                                        let size = terminal.size()?;
                                        let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
                                        let main_chunks = Layout::default()
                                            .direction(Direction::Vertical)
                                            .constraints([
                                                Constraint::Length(1), // Header
                                                Constraint::Min(0),    // Body
                                                Constraint::Length(1), // Footer
                                            ])
                                            .split(area);
                                        let body_chunks = Layout::default()
                                            .direction(Direction::Horizontal)
                                            .constraints(if app_state.show_diffs {
                                                [Constraint::Percentage(50), Constraint::Percentage(50)]
                                            } else {
                                                [Constraint::Percentage(100), Constraint::Percentage(0)]
                                            })
                                            .split(main_chunks[1]);

                                        // Revision Graph Area
                                        let graph_area = body_chunks[0];
                                        if mouse.column >= graph_area.x + 1 && mouse.column < graph_area.x + graph_area.width - 1
                                            && mouse.row >= graph_area.y + 1 && mouse.row < graph_area.y + graph_area.height - 1
                                        {
                                            if is_double_click {
                                                Some(Action::ToggleDiffs)
                                            } else {
                                                let clicked_row = (mouse.row - (graph_area.y + 1)) as usize;
                                                let offset = app_state.log_list_state.offset();
                                                let mut result = None;
                                                if let Some(repo) = &app_state.repo {
                                                    let mut current_y = 0;
                                                    for i in offset..repo.graph.len() {
                                                        let row = &repo.graph[i];
                                                        let is_selected = app_state.log_list_state.selected() == Some(i);
                                                        let row_height = 2 + if is_selected && app_state.show_diffs { row.changed_files.len() as usize } else { 0 };

                                                        if clicked_row >= current_y && clicked_row < current_y + row_height {
                                                            result = Some(Action::SelectIndex(i));
                                                            break;
                                                        }
                                                        current_y += row_height;
                                                        if current_y > graph_area.height as usize {
                                                            break;
                                                        }
                                                    }
                                                }
                                                result
                                            }
                                        } else {
                                            None
                                        }
                                    }
                                    _ => None,
                                }
                            }
                            _ => None,
                        }
                    }
                    _ => {
                        match event {
                            Event::Key(key) => {
                                match key.code {
                                    KeyCode::Char('q') => Some(Action::Quit),
                                    KeyCode::Enter => Some(Action::ToggleDiffs),
                                    KeyCode::Tab | KeyCode::Char('l') => Some(Action::FocusDiff),
                                    KeyCode::Down | KeyCode::Char('j') => Some(Action::SelectNext),
                                    KeyCode::Up | KeyCode::Char('k') => Some(Action::SelectPrev),
                                    KeyCode::Char('s') => Some(Action::SnapshotWorkingCopy),
                                    KeyCode::Char('S') => {
                                        if let (Some(repo), Some(idx)) = (&app_state.repo, app_state.log_list_state.selected()) {
                                            if let Some(row) = repo.graph.get(idx) {
                                                Some(Action::SquashRevision(row.commit_id.clone()))
                                            } else { None }
                                        } else { None }
                                    },
                                    KeyCode::Char('e') => {
                                        if let (Some(repo), Some(idx)) = (&app_state.repo, app_state.log_list_state.selected()) {
                                            if let Some(row) = repo.graph.get(idx) {
                                                Some(Action::EditRevision(row.commit_id.clone()))
                                            } else { None }
                                        } else { None }
                                    },
                                    KeyCode::Char('n') => {
                                        if let (Some(repo), Some(idx)) = (&app_state.repo, app_state.log_list_state.selected()) {
                                            if let Some(row) = repo.graph.get(idx) {
                                                Some(Action::NewRevision(row.commit_id.clone()))
                                            } else { None }
                                        } else { None }
                                    },
                                    KeyCode::Char('a') => {
                                        if let (Some(repo), Some(idx)) = (&app_state.repo, app_state.log_list_state.selected()) {
                                            if let Some(row) = repo.graph.get(idx) {
                                                Some(Action::AbandonRevision(row.commit_id.clone()))
                                            } else { None }
                                        } else { None }
                                    },
                                    KeyCode::Char('b') => Some(Action::SetBookmarkIntent),
                                    KeyCode::Char('B') => {
                                        if let (Some(repo), Some(idx)) = (&app_state.repo, app_state.log_list_state.selected()) {
                                            if let Some(row) = repo.graph.get(idx) {
                                                // For now, delete the first bookmark if exists
                                                if let Some(bookmark) = row.bookmarks.first() {
                                                    Some(Action::DeleteBookmark(bookmark.clone()))
                                                } else { None }
                                            } else { None }
                                        } else { None }
                                    },
                                    KeyCode::Char('d') => Some(Action::DescribeRevisionIntent),
                                    KeyCode::Char('u') => Some(Action::Undo),
                                    KeyCode::Char('U') => Some(Action::Redo),
                                    KeyCode::PageDown => Some(Action::ScrollDiffDown(10)),
                                    KeyCode::PageUp => Some(Action::ScrollDiffUp(10)),
                                    KeyCode::Char('[') => Some(Action::PrevHunk),
                                    KeyCode::Char(']') => Some(Action::NextHunk),
                                    _ => None,
                                }
                            },
                            Event::Mouse(mouse) => {
                                let size = terminal.size()?;
                                let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
                                let main_chunks = Layout::default()
                                    .direction(Direction::Vertical)
                                    .constraints([
                                        Constraint::Length(1), // Header
                                        Constraint::Min(0),    // Body
                                        Constraint::Length(1), // Footer
                                    ])
                                    .split(area);
                                let body_chunks = Layout::default()
                                    .direction(Direction::Horizontal)
                                    .constraints(if app_state.show_diffs {
                                        [Constraint::Percentage(50), Constraint::Percentage(50)]
                                    } else {
                                        [Constraint::Percentage(100), Constraint::Percentage(0)]
                                    })
                                    .split(main_chunks[1]);

                                let graph_area = body_chunks[0];
                                let diff_area = body_chunks[1];

                                match mouse.kind {
                                    MouseEventKind::ScrollUp => {
                                        if app_state.show_diffs && mouse.column >= diff_area.x && mouse.column < diff_area.x + diff_area.width {
                                            Some(Action::ScrollDiffUp(1))
                                        } else {
                                            Some(Action::SelectPrev)
                                        }
                                    }
                                    MouseEventKind::ScrollDown => {
                                        if app_state.show_diffs && mouse.column >= diff_area.x && mouse.column < diff_area.x + diff_area.width {
                                            Some(Action::ScrollDiffDown(1))
                                        } else {
                                            Some(Action::SelectNext)
                                        }
                                    }
                                    MouseEventKind::Down(MouseButton::Left) => {
                                        let now = Instant::now();
                                        let is_double_click = app_state.last_click_time.map_or(false, |t| now.duration_since(t).as_millis() < 500)
                                            && app_state.last_click_pos == Some((mouse.column, mouse.row));
                                        app_state.last_click_time = Some(now);
                                        app_state.last_click_pos = Some((mouse.column, mouse.row));

                                        // Double click anywhere toggles the diff panel
                                        if is_double_click {
                                            Some(Action::ToggleDiffs)
                                        } else if mouse.column >= graph_area.x + 1 && mouse.column < graph_area.x + graph_area.width - 1
                                            && mouse.row >= graph_area.y + 1 && mouse.row < graph_area.y + graph_area.height - 1
                                        {
                                            let clicked_row = (mouse.row - (graph_area.y + 1)) as usize;
                                            let offset = app_state.log_list_state.offset();
                                            let mut result = None;
                                            if let Some(repo) = &app_state.repo {
                                                let mut current_y = 0;
                                                for i in offset..repo.graph.len() {
                                                    let row = &repo.graph[i];
                                                    let is_selected = app_state.log_list_state.selected() == Some(i);
                                                    let row_height = 2 + if is_selected && app_state.show_diffs { row.changed_files.len() as usize } else { 0 };

                                                    if clicked_row >= current_y && clicked_row < current_y + row_height {
                                                        result = Some(Action::SelectIndex(i));
                                                        break;
                                                    }
                                                    current_y += row_height;
                                                    if current_y > graph_area.height as usize {
                                                        break;
                                                    }
                                                }
                                            }
                                            result
                                        } else if app_state.show_diffs && mouse.column >= diff_area.x && mouse.column < diff_area.x + diff_area.width {
                                            Some(Action::FocusDiff)
                                        } else {
                                            None
                                        }
                                    }
                                    MouseEventKind::Down(MouseButton::Right) => {
                                        if mouse.column >= graph_area.x + 1 && mouse.column < graph_area.x + graph_area.width - 1
                                            && mouse.row >= graph_area.y + 1 && mouse.row < graph_area.y + graph_area.height - 1
                                        {
                                            let clicked_row = (mouse.row - (graph_area.y + 1)) as usize;
                                            let offset = app_state.log_list_state.offset();
                                            let mut result = None;
                                            if let Some(repo) = &app_state.repo {
                                                let mut current_y = 0;
                                                for i in offset..repo.graph.len() {
                                                    let row = &repo.graph[i];
                                                    let is_selected = app_state.log_list_state.selected() == Some(i);
                                                    let row_height = 2 + if is_selected && app_state.show_diffs { row.changed_files.len() as usize } else { 0 };

                                                    if clicked_row >= current_y && clicked_row < current_y + row_height {
                                                        // Selection happens on right click too
                                                        action_tx.try_send(Action::SelectIndex(i)).ok();
                                                        result = Some(Action::OpenContextMenu(row.commit_id.clone(), (mouse.column, mouse.row)));
                                                        break;
                                                    }
                                                    current_y += row_height;
                                                    if current_y > graph_area.height as usize {
                                                        break;
                                                    }
                                                }
                                            }
                                            result
                                        } else {
                                            None
                                        }
                                    }
                                    _ => None,
                                }
                            }
                            _ => None,
                        }
                    }
                }
            },

            // Async Results
            Some(a) = action_rx.recv() => Some(a),
        };

        // --- 3. Update (Reducer) ---
        if let Some(action) = action {
            if let Action::Quit = action {
                break;
            }

            // Run reducer
            let command = reducer::update(&mut app_state, action.clone());

            // Post-reducer side effects (Runtime logic)
            if app_state.should_quit {
                break;
            }

            if let Some(cmd) = command {
                handle_command(cmd, adapter.clone(), action_tx.clone()).await?;
            }
        }
    }

    Ok(())
}

fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

async fn handle_command(
    command: Command,
    adapter: Arc<dyn VcsFacade>,
    tx: mpsc::Sender<Action>,
) -> Result<()> {
    match command {
        Command::LoadRepo => {
            tokio::spawn(async move {
                match adapter.get_operation_log().await {
                    Ok(repo) => {
                        let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::ErrorOccurred(format!("Failed to load repo: {}", e)))
                            .await;
                    }
                }
            });
        }
        Command::LoadDiff(commit_id) => {
            let commit_id_clone = commit_id.clone();
            tokio::spawn(async move {
                match adapter.get_commit_diff(&commit_id).await {
                    Ok(diff) => {
                        let _ = tx.send(Action::DiffLoaded(commit_id_clone, diff)).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::DiffLoaded(commit_id_clone, format!("Error: {}", e)))
                            .await;
                    }
                }
            });
        }
        Command::DescribeRevision(commit_id, message) => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted(format!(
                        "Describing {}...",
                        commit_id
                    )))
                    .await;
                match adapter.describe_revision(&commit_id.0, &message).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Ok("Described".to_string())))
                            .await;
                        // Reload repo after operation
                        if let Ok(repo) = adapter.get_operation_log().await {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::Snapshot => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted("Snapshotting...".to_string()))
                    .await;
                match adapter.snapshot().await {
                    Ok(msg) => {
                        let _ = tx.send(Action::OperationCompleted(Ok(msg))).await;
                        if let Ok(repo) = adapter.get_operation_log().await {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::Edit(commit_id) => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted(format!(
                        "Editing {}...",
                        commit_id
                    )))
                    .await;
                match adapter.edit(&commit_id).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(
                                Ok("Edit successful".to_string()),
                            ))
                            .await;
                        if let Ok(repo) = adapter.get_operation_log().await {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::Squash(commit_id) => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted(format!(
                        "Squashing {}...",
                        commit_id
                    )))
                    .await;
                match adapter.squash(&commit_id).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Ok(
                                "Squash successful".to_string()
                            )))
                            .await;
                        if let Ok(repo) = adapter.get_operation_log().await {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::New(commit_id) => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted(format!(
                        "Creating child of {}...",
                        commit_id
                    )))
                    .await;
                match adapter.new_child(&commit_id).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Ok(
                                "New revision created".to_string()
                            )))
                            .await;
                        if let Ok(repo) = adapter.get_operation_log().await {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::Abandon(commit_id) => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted(format!(
                        "Abandoning {}...",
                        commit_id
                    )))
                    .await;
                match adapter.abandon(&commit_id).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Ok(
                                "Revision abandoned".to_string()
                            )))
                            .await;
                        if let Ok(repo) = adapter.get_operation_log().await {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::SetBookmark(commit_id, name) => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted(format!(
                        "Setting bookmark {}...",
                        name
                    )))
                    .await;
                match adapter.set_bookmark(&commit_id, &name).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Ok(format!(
                                "Bookmark {} set",
                                name
                            ))))
                            .await;
                        if let Ok(repo) = adapter.get_operation_log().await {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::DeleteBookmark(name) => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted(format!(
                        "Deleting bookmark {}...",
                        name
                    )))
                    .await;
                match adapter.delete_bookmark(&name).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Ok(format!(
                                "Bookmark {} deleted",
                                name
                            ))))
                            .await;
                        if let Ok(repo) = adapter.get_operation_log().await {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::Undo => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted("Undoing...".to_string()))
                    .await;
                match adapter.undo().await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(
                                Ok("Undo successful".to_string()),
                            ))
                            .await;
                        if let Ok(repo) = adapter.get_operation_log().await {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
        Command::Redo => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted("Redoing...".to_string()))
                    .await;
                match adapter.redo().await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::OperationCompleted(
                                Ok("Redo successful".to_string()),
                            ))
                            .await;
                        if let Ok(repo) = adapter.get_operation_log().await {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {}", e))))
                            .await;
                    }
                }
            });
        }
    }
    Ok(())
}
