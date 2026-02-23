use crate::app::{action::Action, state::AppState, ui};
use crate::components::revision_graph::calculate_row_height;
use crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind};
use ratatui::layout::Size;
use std::time::Instant;

pub fn resolve_clicked_row(
    app_state: &AppState<'_>,
    clicked_row: usize,
) -> Option<(usize, Option<usize>)> {
    let offset = app_state.log.list_state.offset();
    let repo = app_state.repo.as_ref()?;

    let mut current_y = 0;
    for i in offset..repo.graph.len() {
        let row = &repo.graph[i];
        let is_selected = app_state.log.list_state.selected() == Some(i);
        let row_height = calculate_row_height(row, is_selected, app_state.show_diffs) as usize;

        if clicked_row >= current_y && clicked_row < current_y + row_height {
            let file_idx = if is_selected && app_state.show_diffs && clicked_row >= current_y + 2 {
                Some(clicked_row - (current_y + 2))
            } else {
                None
            };
            return Some((i, file_idx));
        }
        current_y += row_height;
    }
    None
}

pub fn map_event_to_action(
    event: Event,
    app_state: &AppState<'_>,
    terminal_size: Size,
) -> Option<Action> {
    if let Event::Key(key) = &event {
        if key.kind == crossterm::event::KeyEventKind::Release {
            return None;
        }
    }

    match app_state.mode {
        crate::app::state::AppMode::Input
        | crate::app::state::AppMode::BookmarkInput
        | crate::app::state::AppMode::CommitInput => match event {
            Event::Key(key) => match key.code {
                KeyCode::Esc => Some(Action::CancelMode),
                KeyCode::Enter => {
                    if app_state.mode == crate::app::state::AppMode::CommitInput {
                        if let Some(input) = &app_state.input {
                            return Some(Action::CommitWorkingCopy(
                                input.text_area.lines().join("\n"),
                            ));
                        }
                        return None;
                    }

                    if let (Some(repo), Some(idx), Some(input)) = (
                        &app_state.repo,
                        app_state.log.list_state.selected(),
                        &app_state.input,
                    ) {
                        if let Some(row) = repo.graph.get(idx) {
                            if app_state.mode == crate::app::state::AppMode::BookmarkInput {
                                let name = input.text_area.lines().join("").trim().to_string();
                                if name.is_empty() {
                                    return None;
                                }
                                Some(Action::SetBookmark(row.commit_id.clone(), name))
                            } else {
                                Some(Action::DescribeRevision(
                                    row.commit_id.clone(),
                                    input.text_area.lines().join("\n"),
                                ))
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => Some(Action::TextAreaInput(key)),
            },
            _ => None,
        },
        crate::app::state::AppMode::FilterInput => match event {
            Event::Key(key) => {
                if let Some(action) = app_state.keymap.get_action(key, app_state) {
                    return Some(action);
                }
                Some(Action::TextAreaInput(key))
            }
            _ => None,
        },
        crate::app::state::AppMode::ContextMenu => {
            match event {
                Event::Key(key) => match key.code {
                    KeyCode::Esc => Some(Action::CloseContextMenu),
                    KeyCode::Char('j') | KeyCode::Down => Some(Action::SelectContextMenuNext),
                    KeyCode::Char('k') | KeyCode::Up => Some(Action::SelectContextMenuPrev),
                    KeyCode::Enter => app_state
                        .context_menu
                        .as_ref()
                        .map(|menu| Action::SelectContextMenuAction(menu.selected_index)),
                    _ => None,
                },
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        if let Some(menu) = &app_state.context_menu {
                            let area = ratatui::layout::Rect::new(
                                0,
                                0,
                                terminal_size.width,
                                terminal_size.height,
                            );
                            let menu_area = menu.calculate_rect(area);

                            if mouse.column >= menu_area.x
                                && mouse.column < menu_area.x + menu_area.width
                                && mouse.row >= menu_area.y
                                && mouse.row < menu_area.y + menu_area.height
                            {
                                // Adjust for borders: top/left border is at y/x, so content starts at y+1
                                let clicked_idx =
                                    (mouse.row.saturating_sub(menu_area.y + 1)) as usize;
                                Some(Action::SelectContextMenuAction(clicked_idx))
                            } else {
                                Some(Action::CloseContextMenu)
                            }
                        } else {
                            Some(Action::CloseContextMenu)
                        }
                    }
                    _ => None,
                },
                _ => None,
            }
        }
        crate::app::state::AppMode::NoRepo => match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
                KeyCode::Char('i') | KeyCode::Enter => Some(Action::InitRepo),
                _ => None,
            },
            _ => None,
        },
        crate::app::state::AppMode::Loading => None,
        crate::app::state::AppMode::Help => match event {
            Event::Key(key) => match key.code {
                KeyCode::Esc | KeyCode::Char('q' | '?') => Some(Action::ToggleHelp),
                _ => None,
            },
            _ => None,
        },
        crate::app::state::AppMode::Diff => {
            match event {
                Event::Key(key) => {
                    if let Some(action) = app_state.keymap.get_action(key, app_state) {
                        return Some(action);
                    }
                    match key.code {
                        KeyCode::Char('m') | KeyCode::Enter => {
                            if let (Some(repo), Some(idx)) =
                                (&app_state.repo, app_state.log.list_state.selected())
                            {
                                if let Some(row) = repo.graph.get(idx) {
                                    if let Some(file_idx) = app_state.log.selected_file_index {
                                        if let Some(file) = row.changed_files.get(file_idx) {
                                            if file.status
                                                == crate::domain::models::FileStatus::Conflicted
                                            {
                                                return Some(Action::ResolveConflict(
                                                    file.path.clone(),
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                            None
                        }
                        _ => None,
                    }
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => Some(Action::ScrollDiffUp(1)),
                    MouseEventKind::ScrollDown => Some(Action::ScrollDiffDown(1)),
                    MouseEventKind::Down(MouseButton::Left) => {
                        let now = Instant::now();
                        let is_double_click = app_state
                            .last_click_time
                            .is_some_and(|t| now.duration_since(t).as_millis() < 500)
                            && app_state.last_click_pos == Some((mouse.column, mouse.row));

                        let area = ratatui::layout::Rect::new(
                            0,
                            0,
                            terminal_size.width,
                            terminal_size.height,
                        );
                        let layout = ui::get_layout(area, app_state);

                        // Revision Graph Area
                        let graph_area = layout.body[0];
                        if mouse.column > graph_area.x
                            && mouse.column < graph_area.x + graph_area.width - 1
                            && mouse.row > graph_area.y
                            && mouse.row < graph_area.y + graph_area.height - 1
                        {
                            if is_double_click {
                                Some(Action::ToggleDiffs)
                            } else {
                                let clicked_row = (mouse.row - (graph_area.y + 1)) as usize;
                                if let Some((i, file_idx)) =
                                    resolve_clicked_row(app_state, clicked_row)
                                {
                                    if let Some(idx) = file_idx {
                                        Some(Action::SelectFile(idx))
                                    } else {
                                        Some(Action::SelectIndex(i))
                                    }
                                } else {
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                },
                _ => None,
            }
        }
        _ => match event {
            Event::Resize(w, h) => Some(Action::Resize(w, h)),
            Event::Key(key) => {
                if let Some(action) = app_state.keymap.get_action(key, app_state) {
                    return Some(action);
                }
                None
            }
            Event::Mouse(mouse) => {
                let area =
                    ratatui::layout::Rect::new(0, 0, terminal_size.width, terminal_size.height);
                let layout = ui::get_layout(area, app_state);

                let graph_area = layout.body[0];
                let diff_area = layout.body[1];

                match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        if app_state.show_diffs
                            && mouse.column >= diff_area.x
                            && mouse.column < diff_area.x + diff_area.width
                        {
                            Some(Action::ScrollDiffUp(1))
                        } else {
                            Some(Action::SelectPrev)
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if app_state.show_diffs
                            && mouse.column >= diff_area.x
                            && mouse.column < diff_area.x + diff_area.width
                        {
                            Some(Action::ScrollDiffDown(1))
                        } else {
                            Some(Action::SelectNext)
                        }
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        let now = Instant::now();
                        let is_double_click = app_state
                            .last_click_time
                            .is_some_and(|t| now.duration_since(t).as_millis() < 500)
                            && app_state.last_click_pos == Some((mouse.column, mouse.row));

                        // Double click anywhere toggles the diff panel
                        if is_double_click {
                            Some(Action::ToggleDiffs)
                        } else if mouse.column > graph_area.x
                            && mouse.column < graph_area.x + graph_area.width - 1
                            && mouse.row > graph_area.y
                            && mouse.row < graph_area.y + graph_area.height - 1
                        {
                            let clicked_row = (mouse.row - (graph_area.y + 1)) as usize;
                            if let Some((i, file_idx)) = resolve_clicked_row(app_state, clicked_row)
                            {
                                if let Some(idx) = file_idx {
                                    Some(Action::SelectFile(idx))
                                } else {
                                    Some(Action::SelectIndex(i))
                                }
                            } else {
                                None
                            }
                        } else if app_state.show_diffs
                            && mouse.column >= diff_area.x
                            && mouse.column < diff_area.x + diff_area.width
                        {
                            Some(Action::FocusDiff)
                        } else {
                            None
                        }
                    }
                    MouseEventKind::Down(MouseButton::Right) => {
                        if mouse.column > graph_area.x
                            && mouse.column < graph_area.x + graph_area.width - 1
                            && mouse.row > graph_area.y
                            && mouse.row < graph_area.y + graph_area.height - 1
                        {
                            let clicked_row = (mouse.row - (graph_area.y + 1)) as usize;
                            if let Some((i, _)) = resolve_clicked_row(app_state, clicked_row) {
                                if let Some(repo) = &app_state.repo {
                                    if let Some(row) = repo.graph.get(i) {
                                        return Some(Action::OpenContextMenu(
                                            Some(row.commit_id.clone()),
                                            (mouse.column, mouse.row),
                                        ));
                                    }
                                }
                            }
                            None
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        },
    }
}
