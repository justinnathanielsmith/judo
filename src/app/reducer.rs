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
            state.current_diff = None; // Invalidate cache
            state.is_loading_diff = true; // Optimistic loading state

            if let (Some(repo), Some(idx)) = (&state.repo, state.log_list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    return Some(Command::LoadDiff(row.commit_id.clone()));
                }
            }
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
            state.current_diff = None;
            state.is_loading_diff = true;

            if let (Some(repo), Some(idx)) = (&state.repo, state.log_list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    return Some(Command::LoadDiff(row.commit_id.clone()));
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
        Action::CancelMode => {
            state.mode = AppMode::Normal;
            state.last_error = None;
            state.text_area = tui_textarea::TextArea::default(); // Reset input
        }
        Action::Quit => {
            state.should_quit = true;
        }

        // --- Async Results ---
        Action::RepoLoaded(repo_status) => {
            state.repo = Some(*repo_status);
            // If nothing selected, select the working copy (or HEAD)
            if state.log_list_state.selected().is_none() {
                state.log_list_state.select(Some(0));
            }
            state.is_loading_diff = true;

            if let (Some(repo), Some(idx)) = (&state.repo, state.log_list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    return Some(Command::LoadDiff(row.commit_id.clone()));
                }
            }
        }
        Action::DiffLoaded(diff) => {
            state.current_diff = Some(diff);
            state.is_loading_diff = false;
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
        }

        _ => {}
    }
    None
}
