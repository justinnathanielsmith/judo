use crate::app::{
    action::{Action, UpdateResult},
    command::Command,
    state::{AppMode, AppState, Panel},
};

pub fn update(state: &mut AppState, action: &Action) -> UpdateResult {
    match action {
        Action::SelectNext => UpdateResult::Handled(move_selection(state, 1)),
        Action::SelectPrev => UpdateResult::Handled(move_selection(state, -1)),
        Action::SelectIndex(idx) => {
            state.log.list_state.select(Some(*idx));
            UpdateResult::Handled(handle_selection(state))
        }
        Action::SelectFile(idx) => {
            state.log.selected_file_index = Some(*idx);
            scroll_to_selected_file(state);
            UpdateResult::Handled(None)
        }
        Action::SelectFileByPath(path) => {
            if let Some(repo) = &state.repo {
                if let Some(idx) = state.log.list_state.selected() {
                    if let Some(row) = repo.graph.get(idx) {
                        if let Some(file_idx) =
                            row.changed_files.iter().position(|f| f.path == *path)
                        {
                            state.log.selected_file_index = Some(file_idx);
                            scroll_to_selected_file(state);
                        }
                    }
                }
            }
            UpdateResult::Handled(None)
        }
        Action::SelectNextFile => {
            if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    let next = match state.log.selected_file_index {
                        Some(i) => (i + 1).min(row.changed_files.len().saturating_sub(1)),
                        None => 0,
                    };
                    state.log.selected_file_index = Some(next);
                    scroll_to_selected_file(state);
                }
            }
            UpdateResult::Handled(None)
        }
        Action::SelectPrevFile => {
            if let Some(i) = state.log.selected_file_index {
                state.log.selected_file_index = Some(i.saturating_sub(1));
                scroll_to_selected_file(state);
            }
            UpdateResult::Handled(None)
        }
        Action::ScrollDiffUp(n) => {
            state.log.diff_scroll = state.log.diff_scroll.saturating_sub(*n);
            UpdateResult::Handled(None)
        }
        Action::ScrollDiffDown(n) => {
            state.log.diff_scroll = state.log.diff_scroll.saturating_add(*n);
            UpdateResult::Handled(None)
        }
        Action::NextHunk => {
            if let Some(diff) = &state.log.current_diff {
                let current = state.log.diff_scroll as usize;
                let lines: Vec<&str> = diff.lines().collect();
                for (idx, line) in lines.iter().enumerate().skip(current + 1) {
                    if line.starts_with("@@") {
                        state.log.diff_scroll = idx as u16;
                        state.hunk_highlight_time = Some(std::time::Instant::now());
                        break;
                    }
                }
            }
            UpdateResult::Handled(None)
        }
        Action::PrevHunk => {
            if let Some(diff) = &state.log.current_diff {
                let current = state.log.diff_scroll as usize;
                let lines: Vec<&str> = diff.lines().collect();
                for (idx, line) in lines.iter().enumerate().take(current).rev() {
                    if line.starts_with("@@") {
                        state.log.diff_scroll = idx as u16;
                        state.hunk_highlight_time = Some(std::time::Instant::now());
                        break;
                    }
                }
            }
            UpdateResult::Handled(None)
        }
        Action::ToggleDiffs => {
            state.show_diffs = !state.show_diffs;
            if !state.show_diffs {
                state.mode = AppMode::Normal;
                state.focused_panel = Panel::Graph;
            } else {
                state.mode = AppMode::Diff;
                state.focused_panel = Panel::Diff;
                // Restore original opening size
                state.diff_ratio = 40;
                if state.log.selected_file_index.is_none() {
                    state.log.selected_file_index = Some(0);
                    scroll_to_selected_file(state);
                }
            }
            UpdateResult::Handled(None)
        }
        Action::FocusDiff => {
            state.show_diffs = true;
            state.mode = AppMode::Diff;
            state.focused_panel = Panel::Diff;
            if state.log.selected_file_index.is_none() {
                state.log.selected_file_index = Some(0);
                scroll_to_selected_file(state);
            }
            UpdateResult::Handled(None)
        }
        Action::FocusGraph => {
            state.mode = AppMode::Normal;
            state.focused_panel = Panel::Graph;
            UpdateResult::Handled(None)
        }
        _ => UpdateResult::NotHandled,
    }
}

fn move_selection(state: &mut AppState, delta: isize) -> Option<Command> {
    let len = state.repo.as_ref().map_or(0, |r| r.graph.len());
    let current_index = state.log.list_state.selected();
    let new_index = calculate_new_index(current_index, delta, len);

    state.log.list_state.select(Some(new_index));
    handle_selection(state)
}

fn calculate_new_index(current: Option<usize>, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    match current {
        Some(i) => (i as isize + delta).rem_euclid(len as isize) as usize,
        None => 0,
    }
}

pub fn handle_selection(state: &mut AppState) -> Option<Command> {
    if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
        if let Some(row) = repo.graph.get(idx) {
            let commit_id = row.commit_id.clone();
            state.log.diff_scroll = 0; // Reset scroll on selection change
            state.log.selected_file_index = None;
            if let Some(cached_diff) = state.log.diff_cache.get(&commit_id) {
                state.log.current_diff = Some(cached_diff.clone());
                state.log.is_loading_diff = false;
                return None;
            }
            state.log.current_diff = None;
            state.log.is_loading_diff = true;
            return Some(Command::LoadDiff(commit_id));
        }
    }
    None
}

fn scroll_to_selected_file(state: &mut AppState) {
    if let (Some(repo), Some(idx), Some(file_idx), Some(diff)) = (
        &state.repo,
        state.log.list_state.selected(),
        state.log.selected_file_index,
        &state.log.current_diff,
    ) {
        if let Some(row) = repo.graph.get(idx) {
            if let Some(file) = row.changed_files.get(file_idx) {
                let target = format!("File: {}", file.path);
                if let Some(line_idx) = diff.lines().position(|l| l.starts_with(&target)) {
                    state.log.diff_scroll = line_idx as u16;
                }
            }
        }
    }
}
