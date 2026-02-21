use super::{
    action::Action,
    command::Command,
    state::{AppMode, AppState, AppTextArea},
};
use std::time::{Duration, Instant};

const STATUS_CLEAR_DURATION: Duration = Duration::from_secs(3);

pub fn update(state: &mut AppState, action: Action) -> Option<Command> {
    match action {
        // --- Navigation ---
        Action::SelectNext => {
            return move_selection(state, 1);
        }
        Action::SelectPrev => {
            return move_selection(state, -1);
        }
        Action::SelectIndex(i) => {
            state.log_list_state.select(Some(i));
            return handle_selection(state);
        }
        Action::SelectFile(i) => {
            state.selected_file_index = Some(i);
            scroll_to_selected_file(state);
        }
        Action::SelectFileByPath(path) => {
            if let (Some(repo), Some(idx)) = (&state.repo, state.log_list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    if let Some(file_idx) = row.changed_files.iter().position(|f| f.path == path) {
                        state.selected_file_index = Some(file_idx);
                        scroll_to_selected_file(state);
                    }
                }
            }
        }
        Action::SelectNextFile => {
            if let (Some(repo), Some(idx)) = (&state.repo, state.log_list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    let len = row.changed_files.len();
                    if len > 0 {
                        let current = state.selected_file_index.unwrap_or(0);
                        state.selected_file_index = Some((current + 1) % len);
                        scroll_to_selected_file(state);
                    }
                }
            }
        }
        Action::SelectPrevFile => {
            if let (Some(repo), Some(idx)) = (&state.repo, state.log_list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    let len = row.changed_files.len();
                    if len > 0 {
                        let current = state.selected_file_index.unwrap_or(0);
                        state.selected_file_index =
                            Some(if current == 0 { len - 1 } else { current - 1 });
                        scroll_to_selected_file(state);
                    }
                }
            }
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
        Action::EnterFilterMode => {
            state.mode = AppMode::FilterInput;
            state.text_area = AppTextArea::default();
            if let Some(revset) = &state.revset {
                state.text_area.insert_str(revset);
            }
        }
        Action::ApplyFilter(revset) => {
            state.mode = AppMode::Normal;
            let revset = if revset.trim().is_empty() {
                None
            } else {
                Some(revset.trim().to_string())
            };
            state.revset = revset.clone();
            state.log_list_state.select(Some(0));
            return Some(Command::LoadRepo(None, 100, revset));
        }
        Action::FilterMine => {
            state.revset = Some("mine()".to_string());
            state.log_list_state.select(Some(0));
            return Some(Command::LoadRepo(None, 100, state.revset.clone()));
        }
        Action::FilterTrunk => {
            state.revset = Some("trunk()".to_string());
            state.log_list_state.select(Some(0));
            return Some(Command::LoadRepo(None, 100, state.revset.clone()));
        }
        Action::FilterConflicts => {
            state.revset = Some("conflicts()".to_string());
            state.log_list_state.select(Some(0));
            return Some(Command::LoadRepo(None, 100, state.revset.clone()));
        }
        Action::FocusDiff => {
            if state.show_diffs {
                state.mode = AppMode::Diff;
                if state.selected_file_index.is_none() {
                    state.selected_file_index = Some(0);
                }
                scroll_to_selected_file(state);
            }
        }
        Action::FocusGraph => {
            state.mode = AppMode::Normal;
        }
        Action::CancelMode | Action::CloseContextMenu => {
            state.mode = AppMode::Normal;
            state.last_error = None;
            state.text_area = AppTextArea::default(); // Reset input
            state.context_menu = None;
        }
        Action::TextAreaInput(key) => {
            state.text_area.input(key);
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
        Action::Fetch => {
            return Some(Command::Fetch);
        }
        Action::PushIntent => {
            if let (Some(repo), Some(idx)) = (&state.repo, state.log_list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    if row.bookmarks.is_empty() {
                        // Push without bookmark (will push current working copy or as configured)
                        return Some(Command::Push(None));
                    } else if row.bookmarks.len() == 1 {
                        // Push the single bookmark
                        return Some(Command::Push(Some(row.bookmarks[0].clone())));
                    } else {
                        // Multiple bookmarks: open context menu for selection
                        let mut actions = Vec::new();
                        for bookmark in &row.bookmarks {
                            actions.push((
                                format!("Push bookmark: {}", bookmark),
                                Action::Push(Some(bookmark.clone())),
                            ));
                        }
                        actions.push(("Push All".to_string(), Action::Push(None)));

                        // Position it near the selection if possible, or just center-ish
                        state.mode = AppMode::ContextMenu;
                        state.context_menu = Some(super::state::ContextMenuState {
                            commit_id: row.commit_id.clone(),
                            x: 10, // Default position, loop.rs might override if we had mouse pos
                            y: 10,
                            selected_index: 0,
                            actions,
                        });
                    }
                }
            }
        }
        Action::Push(bookmark) => {
            state.mode = AppMode::Normal;
            return Some(Command::Push(bookmark));
        }
        Action::ResolveConflict(path) => {
            return Some(Command::ResolveConflict(path));
        }
        Action::DescribeRevisionIntent => {
            state.mode = AppMode::Input;
            state.text_area = AppTextArea::default();
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
            state.text_area = AppTextArea::default();
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

        Action::LoadMoreGraph => {
            if state.is_loading_more || !state.has_more {
                return None;
            }
            if let Some(repo) = &state.repo {
                use std::collections::HashSet;
                let existing_ids: HashSet<crate::domain::models::CommitId> =
                    repo.graph.iter().map(|r| r.commit_id.clone()).collect();
                let mut heads = Vec::new();
                for row in &repo.graph {
                    for parent in &row.parents {
                        if !existing_ids.contains(parent) {
                            heads.push(parent.clone());
                        }
                    }
                }
                if !heads.is_empty() {
                    state.is_loading_more = true;
                    // Deduplicate heads
                    heads.sort_by(|a, b| a.0.cmp(&b.0));
                    heads.dedup();
                    return Some(Command::LoadRepo(Some(heads), 100, state.revset.clone()));
                } else {
                    state.has_more = false;
                }
            }
        }

        // --- Async Results ---
        Action::RepoLoaded(repo_status) => {
            state.workspace_id = repo_status.workspace_id.clone();
            state.repo = Some(*repo_status);
            state.is_loading_more = false;
            state.has_more = true;
            state.mode = AppMode::Normal;
            state.active_tasks.retain(|t| !t.contains("Syncing"));
            refresh_derived_state(state);
            // If nothing selected, select the working copy (or HEAD)
            if state.log_list_state.selected().is_none() {
                state.log_list_state.select(Some(0));
            }
            return handle_selection(state);
        }
        Action::GraphBatchLoaded(repo_status) => {
            state.is_loading_more = false;
            if let Some(repo) = &mut state.repo {
                use std::collections::HashSet;
                let existing_ids: HashSet<crate::domain::models::CommitId> =
                    repo.graph.iter().map(|r| r.commit_id.clone()).collect();
                for row in repo_status.graph {
                    if !existing_ids.contains(&row.commit_id) {
                        repo.graph.push(row);
                    }
                }
                refresh_derived_state(state);
            }
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
            state.active_tasks.push(msg.clone());
            state.status_message = Some(msg);
            state.status_clear_time = Some(Instant::now() + STATUS_CLEAR_DURATION);
            state.mode = AppMode::Loading;
        }
        Action::OperationCompleted(result) => {
            state.active_tasks.pop();
            if state.mode == AppMode::Loading {
                state.mode = AppMode::Normal;
            }
            match result {
                Ok(msg) => {
                    state.status_message = Some(msg);
                    state.status_clear_time = Some(Instant::now() + STATUS_CLEAR_DURATION);
                    state.diff_cache.clear(); // Clear cache as operations might change history
                    return Some(Command::LoadRepo(None, 100, state.revset.clone()));
                }
                Err(err) => {
                    state.last_error = Some(err);
                    return Some(Command::LoadRepo(None, 100, state.revset.clone()));
                }
            }
        }
        Action::ErrorOccurred(err) => {
            state.last_error = Some(err);
            if state.mode == AppMode::Loading {
                state.mode = AppMode::Normal;
            }
            return Some(Command::LoadRepo(None, 100, state.revset.clone()));
        }

        Action::ExternalChangeDetected => {
            state.mode = AppMode::Loading;
            state.current_diff = None;
            state
                .active_tasks
                .push("Syncing external changes...".to_string());
            return Some(Command::LoadRepo(None, 100, state.revset.clone()));
        }

        Action::Tick => {
            state.frame_count = state.frame_count.wrapping_add(1);
            if let Some(clear_time) = state.status_clear_time {
                if Instant::now() >= clear_time {
                    state.status_message = None;
                    state.status_clear_time = None;
                }
            }
            refresh_derived_state(state);

            // Pagination: check if we are near the end of the graph
            if let (Some(repo), Some(idx)) = (&state.repo, state.log_list_state.selected()) {
                if idx + 20 >= repo.graph.len() && !state.is_loading_more && state.has_more {
                    // We need to trigger LoadMoreGraph. Reducer can't dispatch actions.
                    // But we can return a command!
                    // Wait, Tick doesn't usually return a command, but it can.
                    // Let's check update() signature.
                    return update(state, Action::LoadMoreGraph);
                }
            }
        }

        _ => {}
    }
    None
}

fn move_selection(state: &mut AppState, delta: isize) -> Option<Command> {
    let len = state.repo.as_ref().map(|r| r.graph.len()).unwrap_or(0);
    let current_index = state.log_list_state.selected();
    let new_index = calculate_new_index(current_index, delta, len);

    state.log_list_state.select(Some(new_index));
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

fn handle_selection(state: &mut AppState) -> Option<Command> {
    if let (Some(repo), Some(idx)) = (&state.repo, state.log_list_state.selected()) {
        if let Some(row) = repo.graph.get(idx) {
            let commit_id = row.commit_id.clone();
            state.diff_scroll = 0; // Reset scroll on selection change
            state.selected_file_index = None;
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

fn refresh_derived_state(state: &mut AppState) {
    // Update spinner
    let spin_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    state.spinner = spin_chars[(state.frame_count % spin_chars.len() as u64) as usize].to_string();

    // Update header state
    if let Some(repo) = &mut state.repo {
        let mutable_count = repo.graph.iter().filter(|r| !r.is_immutable).count();
        let immutable_count = repo.graph.iter().filter(|r| r.is_immutable).count();

        state.header_state.op_id = repo.operation_id[..8.min(repo.operation_id.len())].to_string();
        state.header_state.wc_info = format!(
            " WC: {} ",
            &repo.working_copy_id.0[..8.min(repo.working_copy_id.0.len())]
        );
        state.header_state.stats = format!(" | Mut: {} Imm: {} ", mutable_count, immutable_count);

        // --- Calculate Graph Lanes (Business logic extracted from View) ---
        let mut active_commits: Vec<Option<String>> = Vec::new();
        for row in &mut repo.graph {
            let commit_id_hex = &row.commit_id.0;

            // Find or assign a lane for this commit
            let current_lane = active_commits
                .iter()
                .position(|l| l.as_ref() == Some(commit_id_hex))
                .unwrap_or_else(|| {
                    if let Some(pos) = active_commits.iter().position(|l| l.is_none()) {
                        active_commits[pos] = Some(commit_id_hex.clone());
                        pos
                    } else {
                        active_commits.push(Some(commit_id_hex.clone()));
                        active_commits.len() - 1
                    }
                });

            // Store results in the model
            row.visual.column = current_lane;
            row.visual.active_lanes = active_commits.iter().map(|l| l.is_some()).collect();

            // Prepare lanes for parents (and for the connector lines)
            active_commits[current_lane] = None;
            for parent in &row.parents {
                if !active_commits.iter().any(|l| l.as_ref() == Some(&parent.0)) {
                    if let Some(pos) = active_commits.iter().position(|l| l.is_none()) {
                        active_commits[pos] = Some(parent.0.clone());
                    } else {
                        active_commits.push(Some(parent.0.clone()));
                    }
                }
            }

            row.visual.connector_lanes = active_commits.iter().map(|l| l.is_some()).collect();
        }
    } else {
        state.header_state.op_id = "........".to_string();
        state.header_state.wc_info = " Loading... ".to_string();
        state.header_state.stats = "".to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{CommitId, FileChange, FileStatus, GraphRow, RepoStatus};

    fn create_dummy_repo(count: usize) -> RepoStatus {
        let mut graph = Vec::new();
        for i in 0..count {
            graph.push(GraphRow {
                commit_id: CommitId(format!("commit-{}", i)),
                change_id: format!("change-{}", i),
                description: "desc".to_string(),
                author: "author".to_string(),
                timestamp: "time".to_string(),
                is_working_copy: false,
                is_immutable: false,
                parents: vec![],
                bookmarks: vec![],
                changed_files: vec![],
                visual: crate::domain::models::GraphRowVisual::default(),
            });
        }
        RepoStatus {
            operation_id: "op".to_string(),
            workspace_id: "default".to_string(),
            working_copy_id: CommitId("wc".to_string()),
            graph,
        }
    }

    #[test]
    fn test_navigation() {
        let mut state = AppState {
            repo: Some(create_dummy_repo(0)),
            ..Default::default()
        };

        // 1. Empty Repo
        update(&mut state, Action::SelectNext);
        assert_eq!(state.log_list_state.selected(), Some(0));

        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log_list_state.selected(), Some(0));

        // 2. Repo with 3 items
        state.repo = Some(create_dummy_repo(3));
        state.log_list_state.select(None); // Reset

        // Initial Next from None -> 0
        update(&mut state, Action::SelectNext);
        assert_eq!(state.log_list_state.selected(), Some(0));

        // Next -> 1
        update(&mut state, Action::SelectNext);
        assert_eq!(state.log_list_state.selected(), Some(1));

        // Next -> 2
        update(&mut state, Action::SelectNext);
        assert_eq!(state.log_list_state.selected(), Some(2));

        // Next (Wrap) -> 0
        update(&mut state, Action::SelectNext);
        assert_eq!(state.log_list_state.selected(), Some(0));

        // Prev (Wrap) -> 2
        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log_list_state.selected(), Some(2));

        // Prev -> 1
        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log_list_state.selected(), Some(1));

        // Test None selection behavior for Prev
        state.log_list_state.select(None);
        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log_list_state.selected(), Some(0));
    }

    #[test]
    fn test_scroll_diff() {
        let mut state = AppState {
            diff_scroll: 10,
            ..Default::default()
        };

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

    fn create_mock_repo(num_rows: usize) -> RepoStatus {
        let graph = (0..num_rows)
            .map(|i| GraphRow {
                commit_id: CommitId(format!("commit{}", i)),
                change_id: format!("change{}", i),
                description: format!("desc{}", i),
                author: "author".to_string(),
                timestamp: "2023-01-01 00:00:00".to_string(),
                is_working_copy: i == 0,
                is_immutable: false,
                parents: vec![],
                bookmarks: vec![],
                changed_files: vec![],
                visual: crate::domain::models::GraphRowVisual::default(),
            })
            .collect();

        RepoStatus {
            operation_id: "op".to_string(),
            workspace_id: "default".to_string(),
            working_copy_id: CommitId("commit0".to_string()),
            graph,
        }
    }

    #[test]
    fn test_navigation_basic() {
        let mut state = AppState {
            repo: Some(create_mock_repo(3)),
            ..Default::default()
        };
        state.log_list_state.select(Some(1));

        // Test SelectNext
        update(&mut state, Action::SelectNext);
        assert_eq!(state.log_list_state.selected(), Some(2));

        // Test SelectPrev
        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log_list_state.selected(), Some(1));
    }

    #[test]
    fn test_navigation_wrapping() {
        let num_rows = 3;
        let mut state = AppState {
            repo: Some(create_mock_repo(num_rows)),
            ..Default::default()
        };

        // Wrap from end to beginning
        state.log_list_state.select(Some(num_rows - 1));
        update(&mut state, Action::SelectNext);
        assert_eq!(state.log_list_state.selected(), Some(0));

        // Wrap from beginning to end
        state.log_list_state.select(Some(0));
        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log_list_state.selected(), Some(num_rows - 1));
    }

    #[test]
    fn test_navigation_empty_list() {
        let mut state = AppState {
            repo: Some(create_mock_repo(0)),
            ..Default::default()
        };

        // In both cases, it should default to 0 and not panic/underflow
        state.log_list_state.select(Some(0));
        update(&mut state, Action::SelectNext);
        assert_eq!(state.log_list_state.selected(), Some(0));

        state.log_list_state.select(Some(0));
        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log_list_state.selected(), Some(0));
    }

    #[test]
    fn test_text_area_input() {
        let mut state = AppState::default();
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        update(&mut state, Action::TextAreaInput(key));

        assert_eq!(state.text_area.lines()[0], "a");
    }

    #[test]
    fn test_clear_error_on_cancel_mode() {
        let mut state = AppState {
            last_error: Some("An error occurred".to_string()),
            mode: AppMode::Input,
            ..Default::default()
        };

        update(&mut state, Action::CancelMode);

        assert_eq!(state.last_error, None);
        assert_eq!(state.mode, AppMode::Normal);
    }

    #[test]
    fn test_refresh_derived_state() {
        let mut state = AppState {
            repo: Some(create_mock_repo(2)),
            ..Default::default()
        };

        // Before refresh
        assert_eq!(state.header_state.op_id, "");

        refresh_derived_state(&mut state);

        // After refresh
        assert_eq!(state.header_state.op_id, "op");
        assert_eq!(state.repo.as_ref().unwrap().graph[0].visual.column, 0);
        assert_eq!(
            state.repo.as_ref().unwrap().graph[0].visual.active_lanes,
            vec![true]
        );
    }

    #[test]
    fn test_file_selection() {
        let mut repo = create_mock_repo(1);
        repo.graph[0].changed_files = vec![
            FileChange {
                path: "f1".to_string(),
                status: FileStatus::Added,
            },
            FileChange {
                path: "f2".to_string(),
                status: FileStatus::Modified,
            },
        ];

        let mut state = AppState {
            repo: Some(repo),
            ..Default::default()
        };
        state.log_list_state.select(Some(0));

        // Initial state
        assert_eq!(state.selected_file_index, None);

        // Select next file
        update(&mut state, Action::SelectNextFile);
        assert_eq!(state.selected_file_index, Some(1)); // (0+1)%2 = 1

        update(&mut state, Action::SelectNextFile);
        assert_eq!(state.selected_file_index, Some(0)); // (1+1)%2 = 0

        // Select prev file
        update(&mut state, Action::SelectPrevFile);
        assert_eq!(state.selected_file_index, Some(1)); // (0-1)%2 = 1

        // Reset on commit change
        state.repo.as_mut().unwrap().graph.push(GraphRow {
            commit_id: CommitId("c2".to_string()),
            change_id: "ch2".to_string(),
            description: "d2".to_string(),
            author: "a".to_string(),
            timestamp: "t".to_string(),
            is_working_copy: false,
            is_immutable: false,
            parents: vec![],
            bookmarks: vec![],
            changed_files: vec![],
            visual: crate::domain::models::GraphRowVisual::default(),
        });
        update(&mut state, Action::SelectNext);
        assert_eq!(state.selected_file_index, None);
    }

    #[test]
    fn test_navigation_boundaries_and_empty_state() {
        // Test with empty repo
        let mut state = AppState::default();
        state.repo = Some(create_mock_repo(0));

        update(&mut state, Action::SelectNext);
        assert!(state.log_list_state.selected().unwrap_or(0) < 1);

        update(&mut state, Action::SelectPrev);
        assert!(state.log_list_state.selected().unwrap_or(0) < 1);

        // Test with 1 item repo
        state.repo = Some(create_mock_repo(1));
        state.log_list_state.select(Some(0));

        update(&mut state, Action::SelectNext);
        assert_eq!(state.log_list_state.selected(), Some(0));

        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log_list_state.selected(), Some(0));

        // Test with large deltas (if we had them, but we use 1/-1)
        // Ensure wrap-around is consistent
        state.repo = Some(create_mock_repo(10));
        state.log_list_state.select(Some(9));
        update(&mut state, Action::SelectNext);
        assert_eq!(state.log_list_state.selected(), Some(0));

        state.log_list_state.select(Some(0));
        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log_list_state.selected(), Some(9));
    }

    #[test]
    fn test_comprehensive_esc_behavior() {
        let modes = [
            AppMode::Command,
            AppMode::SquashSelect,
            AppMode::BookmarkInput,
            AppMode::Input,
            AppMode::Loading,
            AppMode::Diff,
            AppMode::ContextMenu,
            AppMode::FilterInput,
        ];

        for mode in modes {
            let mut state = AppState {
                mode,
                last_error: Some("error".to_string()),
                context_menu: Some(crate::app::state::ContextMenuState {
                    commit_id: CommitId("c1".to_string()),
                    x: 0,
                    y: 0,
                    selected_index: 0,
                    actions: vec![],
                }),
                ..Default::default()
            };
            // Set some text in text area
            state.text_area.insert_str("some text");

            update(&mut state, Action::CancelMode);

            assert_eq!(state.mode, AppMode::Normal, "Mode should reset to Normal from {:?}", mode);
            assert_eq!(state.last_error, None, "Error should be cleared from {:?}", mode);
            assert_eq!(state.context_menu, None, "Context menu should be cleared from {:?}", mode);
            assert!(state.text_area.is_empty(), "Text area should be cleared from {:?}", mode);
        }
    }
}

fn scroll_to_selected_file(state: &mut AppState) {
    if let (Some(repo), Some(idx), Some(file_idx), Some(diff)) = (
        &state.repo,
        state.log_list_state.selected(),
        state.selected_file_index,
        &state.current_diff,
    ) {
        if let Some(row) = repo.graph.get(idx) {
            if let Some(file) = row.changed_files.get(file_idx) {
                let target = format!("File: {}", file.path);
                if let Some(line_idx) = diff.lines().position(|l| l.starts_with(&target)) {
                    state.diff_scroll = line_idx as u16;
                }
            }
        }
    }
}
