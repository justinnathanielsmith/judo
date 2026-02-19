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
        Action::ScrollDiffDown(amount) => {
            state.diff_scroll = state.diff_scroll.saturating_add(amount);
        }
        Action::ScrollDiffUp(amount) => {
            state.diff_scroll = state.diff_scroll.saturating_sub(amount);
        }

        // --- Mode Switching ---
        Action::EnterSquashMode => {
            state.mode = AppMode::SquashSelect;
        }
        Action::EnterCommandMode => {
            state.mode = AppMode::Command;
        }
        Action::CancelMode => {
            state.mode = AppMode::Normal;
            state.last_error = None;
            state.text_area = tui_textarea::TextArea::default(); // Reset input
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
                Ok(msg) => state.status_message = Some(msg),
                Err(err) => state.last_error = Some(err),
            }
            if state.mode == AppMode::Loading {
                state.mode = AppMode::Normal;
            }
            // Clear cache after operations that might change history?
            // For now, keep it simple.
        }
        Action::ErrorOccurred(err) => {
            state.last_error = Some(err);
            if state.mode == AppMode::Loading {
                state.mode = AppMode::Normal;
            }
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
