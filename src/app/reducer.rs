use super::{
    action::Action,
    command::Command,
    state::{AppMode, AppState},
};

pub fn update(state: &mut AppState, action: Action) -> Option<Command> {
    match action {
        // --- Navigation ---
        Action::SelectNext => {
            let i = match state.log_list_state.selected() {
                Some(i) => {
                    if let Some(repo) = &state.repo {
                        if repo.graph.is_empty() || i >= repo.graph.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    } else {
                        0
                    }
                }
                None => 0,
            };
            state.log_list_state.select(Some(i));
            return handle_selection(state);
        }
        Action::SelectPrev => {
            let i = match state.log_list_state.selected() {
                Some(i) => {
                    if let Some(repo) = &state.repo {
                        if repo.graph.is_empty() {
                            0
                        } else if i == 0 {
                            repo.graph.len() - 1
                        } else {
                            i - 1
                        }
                    } else {
                        0
                    }
                }
                None => 0,
            };
            state.log_list_state.select(Some(i));
            return handle_selection(state);
        }
        Action::SelectIndex(i) => {
            state.log_list_state.select(Some(i));
            return handle_selection(state);
        }
        Action::ScrollDiffDown(amount) => {
            if let Some(diff) = &state.current_diff {
                let max_scroll = diff.lines().count().saturating_sub(1) as u16;
                state.diff_scroll = state.diff_scroll.saturating_add(amount).min(max_scroll);
            } else {
                state.diff_scroll = state.diff_scroll.saturating_add(amount);
            }
        }
        Action::ScrollDiffUp(amount) => {
            state.diff_scroll = state.diff_scroll.saturating_sub(amount);
        }
        Action::ToggleDiffs => {
            state.show_diffs = !state.show_diffs;
            return handle_selection(state);
        }
        Action::NextHunk => {
            if let Some(diff) = &state.current_diff {
                let current_line = state.diff_scroll as usize;
                let mut lines = diff.lines().enumerate().skip(current_line + 1);
                if let Some((idx, _)) = lines.find(|(_, line)| line.starts_with("@@")) {
                    state.diff_scroll = idx as u16;
                }
            }
        }
        Action::PrevHunk => {
            if let Some(diff) = &state.current_diff {
                let current_line = state.diff_scroll as usize;
                let lines: Vec<_> = diff.lines().enumerate().collect();
                if current_line > 0 {
                    let mut prev_lines = lines[..current_line].iter().rev();
                    if let Some((idx, _)) = prev_lines.find(|(_, line)| line.starts_with("@@")) {
                        state.diff_scroll = *idx as u16;
                    }
                }
            }
        }

        // --- Mode Switching ---
        Action::EnterSquashMode => {
            state.mode = AppMode::SquashSelect;
        }
        Action::EnterCommandMode => {
            state.mode = AppMode::Command;
        }
        Action::FocusDiff => {
            if state.show_diffs {
                state.mode = AppMode::Diff;
            }
        }
        Action::FocusGraph => {
            state.mode = AppMode::Normal;
        }
        Action::CancelMode | Action::CloseContextMenu => {
            state.mode = AppMode::Normal;
            state.last_error = None;
            state.text_area = tui_textarea::TextArea::default(); // Reset input
            state.context_menu = None;
        }
        Action::Quit => {
            state.should_quit = true;
        }

        // --- JJ Intents ---
        Action::SnapshotWorkingCopy => {
            return Some(Command::Snapshot);
        }
        Action::EditRevision(commit_id) => {
            return Some(Command::Edit(commit_id));
        }
        Action::SquashRevision(commit_id) => {
            return Some(Command::Squash(commit_id));
        }
        Action::NewRevision(commit_id) => {
            return Some(Command::New(commit_id));
        }
        Action::AbandonRevision(commit_id) => {
            return Some(Command::Abandon(commit_id));
        }
        Action::Undo => {
            return Some(Command::Undo);
        }
        Action::Redo => {
            return Some(Command::Redo);
        }
        Action::DescribeRevisionIntent => {
            state.mode = AppMode::Input;
            state.text_area = tui_textarea::TextArea::default();
            // Pre-fill with existing description if possible?
            if let (Some(repo), Some(idx)) = (&state.repo, state.log_list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    state.text_area.insert_str(&row.description);
                }
            }
        }
        Action::DescribeRevision(commit_id, message) => {
            state.mode = AppMode::Normal;
            return Some(Command::DescribeRevision(commit_id, message));
        }
        Action::SetBookmarkIntent => {
            state.mode = AppMode::BookmarkInput;
            state.text_area = tui_textarea::TextArea::default();
        }
        Action::SetBookmark(commit_id, name) => {
            state.mode = AppMode::Normal;
            return Some(Command::SetBookmark(commit_id, name));
        }
        Action::DeleteBookmark(name) => {
            return Some(Command::DeleteBookmark(name));
        }

        // --- Context Menu ---
        Action::OpenContextMenu(commit_id, pos) => {
            let actions = vec![
                ("Describe".to_string(), Action::DescribeRevisionIntent),
                (
                    "Squash into Parent".to_string(),
                    Action::SquashRevision(commit_id.clone()),
                ),
                (
                    "New Child".to_string(),
                    Action::NewRevision(commit_id.clone()),
                ),
                ("Edit".to_string(), Action::EditRevision(commit_id.clone())),
                (
                    "Abandon".to_string(),
                    Action::AbandonRevision(commit_id.clone()),
                ),
                ("Set Bookmark".to_string(), Action::SetBookmarkIntent),
                ("Toggle Diffs".to_string(), Action::ToggleDiffs),
            ];

            // If we are in SquashSelect mode, maybe add squash target?
            // For now, let's keep it simple.

            state.mode = AppMode::ContextMenu;
            state.context_menu = Some(super::state::ContextMenuState {
                commit_id,
                x: pos.0,
                y: pos.1,
                selected_index: 0,
                actions,
            });
        }
        Action::SelectContextMenuAction(idx) => {
            if let Some(menu) = &state.context_menu {
                if let Some((_, action)) = menu.actions.get(idx).cloned() {
                    state.context_menu = None;
                    state.mode = AppMode::Normal;
                    // Re-dispatch the action. We can't easily recurse here,
                    // so we just return the command if the action produces one.
                    return update(state, action);
                }
            }
        }
        Action::SelectContextMenuNext => {
            if let Some(menu) = &mut state.context_menu {
                menu.selected_index = (menu.selected_index + 1) % menu.actions.len();
            }
        }
        Action::SelectContextMenuPrev => {
            if let Some(menu) = &mut state.context_menu {
                if menu.selected_index == 0 {
                    menu.selected_index = menu.actions.len() - 1;
                } else {
                    menu.selected_index -= 1;
                }
            }
        }

        // --- Async Results ---
        Action::RepoLoaded(repo_status) => {
            state.repo = Some(*repo_status);
            // If nothing selected, select the working copy (or HEAD)
            if state.log_list_state.selected().is_none() {
                state.log_list_state.select(Some(0));
            }
            return handle_selection(state);
        }
        Action::DiffLoaded(commit_id, diff) => {
            state.diff_cache.insert(commit_id, diff.clone());
            // Only update current_diff if it matches the current selection
            if let (Some(repo), Some(idx)) = (&state.repo, state.log_list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    if row.commit_id
                        == *state
                            .diff_cache
                            .keys()
                            .find(|k| **k == row.commit_id)
                            .unwrap_or(&row.commit_id)
                    {
                        state.current_diff = Some(diff);
                        state.is_loading_diff = false;
                    }
                }
            }
        }
        Action::OperationStarted(msg) => {
            state.status_message = Some(msg);
            state.mode = AppMode::Loading;
        }
        Action::OperationCompleted(result) => {
            match result {
                Ok(msg) => {
                    state.status_message = Some(msg);
                    state.diff_cache.clear(); // Clear cache as operations might change history
                }
                Err(err) => state.last_error = Some(err),
            }
            if state.mode == AppMode::Loading {
                state.mode = AppMode::Normal;
            }
        }
        Action::ErrorOccurred(err) => {
            state.last_error = Some(err);
            if state.mode == AppMode::Loading {
                state.mode = AppMode::Normal;
            }
        }

        Action::Tick => {
            state.frame_count = state.frame_count.wrapping_add(1);
        }

        _ => {}
    }
    None
}

fn handle_selection(state: &mut AppState) -> Option<Command> {
    if let (Some(repo), Some(idx)) = (&state.repo, state.log_list_state.selected()) {
        if let Some(row) = repo.graph.get(idx) {
            let commit_id = row.commit_id.clone();
            state.diff_scroll = 0; // Reset scroll on selection change
            if let Some(cached_diff) = state.diff_cache.get(&commit_id) {
                state.current_diff = Some(cached_diff.clone());
                state.is_loading_diff = false;
                return None;
            } else {
                state.current_diff = None;
                state.is_loading_diff = true;
                return Some(Command::LoadDiff(commit_id));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scroll_diff() {
        let mut state = AppState::default();
        state.diff_scroll = 10;

        // Test ScrollDiffUp normal
        update(&mut state, Action::ScrollDiffUp(5));
        assert_eq!(state.diff_scroll, 5);

        // Test ScrollDiffUp saturating
        update(&mut state, Action::ScrollDiffUp(10));
        assert_eq!(state.diff_scroll, 0);

        // Test ScrollDiffDown
        update(&mut state, Action::ScrollDiffDown(15));
        assert_eq!(state.diff_scroll, 15);
    }
}
