use super::{
    action::Action,
    command::Command,
    command_palette, recovery,
    state::{
        AppMode, AppState, AppTextArea, CommandPaletteState, ErrorSeverity, ErrorState,
        HeaderState, Panel,
    },
};
use chrono::Local;
use std::time::{Duration, Instant};

const STATUS_CLEAR_DURATION: Duration = Duration::from_secs(3);

pub fn update(state: &mut AppState, action: Action) -> Option<Command> {
    match action {
        // --- Navigation ---
        Action::SelectNext => {
            if state.mode == AppMode::ThemeSelection {
                if let Some(ts) = &mut state.theme_selection {
                    ts.selected_index = (ts.selected_index + 1) % ts.themes.len();
                }
                return None;
            }
            return move_selection(state, 1);
        }
        Action::SelectPrev => {
            if state.mode == AppMode::ThemeSelection {
                if let Some(ts) = &mut state.theme_selection {
                    ts.selected_index = if ts.selected_index == 0 {
                        ts.themes.len() - 1
                    } else {
                        ts.selected_index - 1
                    };
                }
                return None;
            }
            return move_selection(state, -1);
        }
        Action::SelectIndex(i) => {
            state.log.list_state.select(Some(i));
            return handle_selection(state);
        }
        Action::SelectFile(i) => {
            state.log.selected_file_index = Some(i);
            scroll_to_selected_file(state);
        }
        Action::SelectFileByPath(path) => {
            if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    if let Some(file_idx) = row.changed_files.iter().position(|f| f.path == path) {
                        state.log.selected_file_index = Some(file_idx);
                        scroll_to_selected_file(state);
                    }
                }
            }
        }
        Action::SelectNextFile => {
            if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    let len = row.changed_files.len();
                    if len > 0 {
                        let current = state.log.selected_file_index.unwrap_or(0);
                        state.log.selected_file_index = Some((current + 1) % len);
                        scroll_to_selected_file(state);
                    }
                }
            }
        }
        Action::SelectPrevFile => {
            if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    let len = row.changed_files.len();
                    if len > 0 {
                        let current = state.log.selected_file_index.unwrap_or(0);
                        state.log.selected_file_index =
                            Some(if current == 0 { len - 1 } else { current - 1 });
                        scroll_to_selected_file(state);
                    }
                }
            }
        }
        Action::ScrollDiffDown(amount) => {
            if let Some(diff) = &state.log.current_diff {
                let max_scroll = diff.lines().count().saturating_sub(1) as u16;
                state.log.diff_scroll =
                    state.log.diff_scroll.saturating_add(amount).min(max_scroll);
            } else {
                state.log.diff_scroll = state.log.diff_scroll.saturating_add(amount);
            }
        }
        Action::ScrollDiffUp(amount) => {
            state.log.diff_scroll = state.log.diff_scroll.saturating_sub(amount);
        }
        Action::ToggleDiffs => {
            state.show_diffs = !state.show_diffs;
            if state.show_diffs {
                state.focused_panel = Panel::Graph;
                state.diff_ratio = 30;
            }
            return handle_selection(state);
        }
        Action::NextHunk => {
            if let Some(diff) = &state.log.current_diff {
                let current_line = state.log.diff_scroll as usize;
                let mut lines = diff.lines().enumerate().skip(current_line + 1);
                if let Some((idx, _)) = lines.find(|(_, line)| line.starts_with("@@")) {
                    state.log.diff_scroll = idx as u16;
                    state.hunk_highlight_time = Some(Instant::now());
                }
            }
        }
        Action::PrevHunk => {
            if let Some(diff) = &state.log.current_diff {
                let current_line = state.log.diff_scroll as usize;
                let lines: Vec<_> = diff.lines().enumerate().collect();
                if current_line > 0 {
                    let mut prev_lines = lines[..current_line].iter().rev();
                    if let Some((idx, _)) = prev_lines.find(|(_, line)| line.starts_with("@@")) {
                        state.log.diff_scroll = *idx as u16;
                        state.hunk_highlight_time = Some(Instant::now());
                    }
                }
            }
        }

        // --- Mode Switching ---
        Action::EnterSquashMode => {
            state.mode = AppMode::SquashSelect;
        }
        Action::EnterCommandMode => {
            state.mode = AppMode::CommandPalette;
            state.command_palette = Some(CommandPaletteState {
                matches: command_palette::search_commands(""),
                ..Default::default()
            });
        }
        Action::EnterFilterMode => {
            state.mode = AppMode::FilterInput;
            let mut text_area = AppTextArea::default();
            if let Some(revset) = &state.revset {
                text_area.insert_str(revset);
            }
            state.input = Some(super::state::InputState { text_area });
            state.selected_filter_index = None;
        }
        Action::ApplyFilter(revset) => {
            state.mode = AppMode::Normal;
            state.input = None;
            state.selected_filter_index = None;

            let revset_str = revset.trim().to_string();
            let revset = if revset_str.is_empty() {
                None
            } else {
                // Update recent filters
                if !state.recent_filters.iter().any(|f| f == &revset_str) {
                    state.recent_filters.insert(0, revset_str.clone());
                    if state.recent_filters.len() > 10 {
                        state.recent_filters.truncate(10);
                    }
                    super::persistence::save_recent_filters(&state.recent_filters);
                } else {
                    // Move to front
                    if let Some(pos) = state.recent_filters.iter().position(|f| f == &revset_str) {
                        state.recent_filters.remove(pos);
                        state.recent_filters.insert(0, revset_str.clone());
                        super::persistence::save_recent_filters(&state.recent_filters);
                    }
                }
                Some(revset_str)
            };
            state.revset = revset.clone();
            state.log.list_state.select(Some(0));
            return Some(Command::LoadRepo(None, 100, revset));
        }
        Action::FilterMine => {
            state.revset = Some("mine()".to_string());
            state.log.list_state.select(Some(0));
            state.status_message = Some("Filtering: mine()".to_string());
            state.status_clear_time = Some(Instant::now() + STATUS_CLEAR_DURATION);
            return Some(Command::LoadRepo(None, 100, state.revset.clone()));
        }
        Action::FilterTrunk => {
            state.revset = Some("trunk()".to_string());
            state.log.list_state.select(Some(0));
            state.status_message = Some("Filtering: trunk()".to_string());
            state.status_clear_time = Some(Instant::now() + STATUS_CLEAR_DURATION);
            return Some(Command::LoadRepo(None, 100, state.revset.clone()));
        }
        Action::FilterConflicts => {
            state.revset = Some("conflicts()".to_string());
            state.log.list_state.select(Some(0));
            state.status_message = Some("Filtering: conflicts()".to_string());
            state.status_clear_time = Some(Instant::now() + STATUS_CLEAR_DURATION);
            return Some(Command::LoadRepo(None, 100, state.revset.clone()));
        }
        Action::FilterNext => {
            if state.mode == AppMode::FilterInput && !state.recent_filters.is_empty() {
                let current = state.selected_filter_index;
                let next = match current {
                    Some(i) => (i + 1) % state.recent_filters.len(),
                    None => 0,
                };
                state.selected_filter_index = Some(next);
                if let Some(input) = &mut state.input {
                    input.text_area = AppTextArea::default();
                    input.text_area.insert_str(&state.recent_filters[next]);
                }
            }
        }
        Action::FilterPrev => {
            if state.mode == AppMode::FilterInput && !state.recent_filters.is_empty() {
                let current = state.selected_filter_index;
                let next = match current {
                    Some(i) => {
                        if i == 0 {
                            state.recent_filters.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => state.recent_filters.len() - 1,
                };
                state.selected_filter_index = Some(next);
                if let Some(input) = &mut state.input {
                    input.text_area = AppTextArea::default();
                    input.text_area.insert_str(&state.recent_filters[next]);
                }
            }
        }
        Action::FocusDiff => {
            if state.show_diffs {
                state.mode = AppMode::Diff;
                state.focused_panel = Panel::Diff;
                state.diff_ratio = 70;
                if state.log.selected_file_index.is_none() {
                    state.log.selected_file_index = Some(0);
                }
                scroll_to_selected_file(state);
            }
        }
        Action::FocusGraph => {
            state.mode = AppMode::Normal;
            state.focused_panel = Panel::Graph;
            state.diff_ratio = 30;
        }
        Action::ToggleHelp => {
            if state.mode == AppMode::Help {
                state.mode = AppMode::Normal;
            } else {
                state.mode = AppMode::Help;
            }
        }
        Action::EnterThemeSelection => {
            state.mode = AppMode::ThemeSelection;
            state.theme_selection = Some(super::state::ThemeSelectionState::default());
            // Set current index based on existing palette_type
            if let Some(ts) = &mut state.theme_selection {
                ts.selected_index = ts
                    .themes
                    .iter()
                    .position(|p| *p == state.palette_type)
                    .unwrap_or(0);
            }
        }
        Action::SwitchTheme(palette) => {
            state.palette_type = palette;
            state.theme = crate::theme::Theme::from_palette_type(palette);
            state.mode = AppMode::Normal;
        }
        Action::ToggleSelection(commit_id) => {
            let id = if commit_id.0.is_empty() {
                if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                    repo.graph.get(idx).map(|r| r.commit_id.clone())
                } else {
                    None
                }
            } else {
                Some(commit_id)
            };

            if let Some(id) = id {
                if state.log.selected_ids.contains(&id) {
                    state.log.selected_ids.remove(&id);
                } else {
                    state.log.selected_ids.insert(id);
                }
            }
        }
        Action::ClearSelection => {
            state.log.selected_ids.clear();
        }
        Action::CancelMode | Action::CloseContextMenu => {
            if !state.log.selected_ids.is_empty() {
                state.log.selected_ids.clear();
            } else {
                state.mode = AppMode::Normal;
                state.focused_panel = Panel::Graph;
                state.last_error = None;
                state.input = None;
                state.context_menu = None;
                state.command_palette = None;
                state.theme_selection = None;
                state.selected_filter_index = None;
            }
        }
        Action::CommandPaletteNext => {
            if let Some(cp) = &mut state.command_palette {
                if !cp.matches.is_empty() {
                    cp.selected_index = (cp.selected_index + 1) % cp.matches.len();
                }
            }
        }
        Action::CommandPalettePrev => {
            if let Some(cp) = &mut state.command_palette {
                if !cp.matches.is_empty() {
                    cp.selected_index = if cp.selected_index == 0 {
                        cp.matches.len() - 1
                    } else {
                        cp.selected_index - 1
                    };
                }
            }
        }
        Action::CommandPaletteSelect => {
            if let Some(cp) = state.command_palette.clone() {
                if let Some(&idx) = cp.matches.get(cp.selected_index) {
                    let cmd = &command_palette::get_commands()[idx];
                    let action = cmd.action.clone();
                    state.command_palette = None;
                    state.mode = AppMode::Normal;
                    return update(state, action);
                }
            } else if state.mode == AppMode::ThemeSelection {
                if let Some(ts) = &state.theme_selection {
                    if let Some(palette) = ts.themes.get(ts.selected_index) {
                        let palette = *palette;
                        return update(state, Action::SwitchTheme(palette));
                    }
                }
            }
        }
        Action::TextAreaInput(key) => {
            if let Some(input) = &mut state.input {
                input.text_area.input(key);
            } else if state.mode == AppMode::CommandPalette {
                if let Some(cp) = &mut state.command_palette {
                    use crossterm::event::KeyCode;
                    match key.code {
                        KeyCode::Char(c) => {
                            cp.query.push(c);
                            cp.matches = command_palette::search_commands(&cp.query);
                            cp.selected_index = 0;
                        }
                        KeyCode::Backspace => {
                            cp.query.pop();
                            cp.matches = command_palette::search_commands(&cp.query);
                            cp.selected_index = 0;
                        }
                        _ => {}
                    }
                }
            }
        }
        Action::Quit => {
            state.should_quit = true;
        }

        // --- JJ Intents ---
        Action::SnapshotWorkingCopy => {
            return Some(Command::Snapshot);
        }
        Action::InitRepo => {
            return Some(Command::InitRepo);
        }
        Action::EditRevision(commit_id) => {
            if commit_id.0.is_empty() {
                if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                    if let Some(row) = repo.graph.get(idx) {
                        return Some(Command::Edit(row.commit_id.clone()));
                    }
                }
                return None;
            }
            return Some(Command::Edit(commit_id));
        }
        Action::SquashRevision(_commit_id) => {
            let ids = state.get_selected_commit_ids();
            if ids.is_empty() {
                if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                    if let Some(row) = repo.graph.get(idx) {
                        return Some(Command::Squash(vec![row.commit_id.clone()]));
                    }
                }
                return None;
            }
            state.log.selected_ids.clear();
            return Some(Command::Squash(ids));
        }
        Action::NewRevision(commit_id) => {
            if commit_id.0.is_empty() {
                if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                    if let Some(row) = repo.graph.get(idx) {
                        return Some(Command::New(row.commit_id.clone()));
                    }
                }
                return None;
            }
            return Some(Command::New(commit_id));
        }
        Action::AbandonRevision(_commit_id) => {
            let ids = state.get_selected_commit_ids();
            if ids.is_empty() {
                if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                    if let Some(row) = repo.graph.get(idx) {
                        return Some(Command::Abandon(vec![row.commit_id.clone()]));
                    }
                }
                return None;
            }
            state.log.selected_ids.clear();
            return Some(Command::Abandon(ids));
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
            if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
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
            let mut text_area = AppTextArea::default();
            // Pre-fill with existing description if possible?
            if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    text_area.insert_str(&row.description);
                }
            }
            state.input = Some(super::state::InputState { text_area });
        }
        Action::DescribeRevision(commit_id, message) => {
            state.mode = AppMode::Normal;
            state.input = None;
            return Some(Command::DescribeRevision(commit_id, message));
        }
        Action::SetBookmarkIntent => {
            state.mode = AppMode::BookmarkInput;
            state.input = Some(super::state::InputState {
                text_area: AppTextArea::default(),
            });
        }
        Action::SetBookmark(commit_id, name) => {
            state.mode = AppMode::Normal;
            return Some(Command::SetBookmark(commit_id, name));
        }
        Action::DeleteBookmarkIntent => {
            if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    if row.bookmarks.len() == 1 {
                        return Some(Command::DeleteBookmark(row.bookmarks[0].clone()));
                    } else if row.bookmarks.len() > 1 {
                        let mut actions = Vec::new();
                        for bookmark in &row.bookmarks {
                            actions.push((
                                format!("Delete: {}", bookmark),
                                Action::DeleteBookmark(bookmark.clone()),
                            ));
                        }
                        state.mode = AppMode::ContextMenu;
                        state.context_menu = Some(super::state::ContextMenuState {
                            commit_id: row.commit_id.clone(),
                            x: 10,
                            y: 10,
                            selected_index: 0,
                            actions,
                        });
                    }
                }
            }
        }
        Action::DeleteBookmark(name) => {
            state.mode = AppMode::Normal;
            return Some(Command::DeleteBookmark(name));
        }

        // --- Context Menu ---
        Action::OpenContextMenu(commit_id, pos) => {
            let mut actions = vec![
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

            // Conditionally add Delete Bookmark if the commit has bookmarks
            if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    if !row.bookmarks.is_empty() {
                        actions.insert(
                            actions.len() - 1,
                            ("Delete Bookmark".to_string(), Action::DeleteBookmarkIntent),
                        );
                    }
                }
            }

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
            update_repository_derived_state(state);
            // If nothing selected, select the working copy (or HEAD)
            if state.log.list_state.selected().is_none() {
                state.log.list_state.select(Some(0));
            }
            return handle_selection(state);
        }
        Action::RepoReloadedBackground(repo_status) => {
            state.workspace_id = repo_status.workspace_id.clone();
            state.active_tasks.retain(|t| !t.contains("Syncing"));

            let selected_commit_id =
                if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                    repo.graph.get(idx).map(|r| r.commit_id.clone())
                } else {
                    None
                };

            state.repo = Some(*repo_status);
            state.is_loading_more = false;
            state.has_more = true;
            update_repository_derived_state(state);

            if let Some(id) = selected_commit_id {
                if let Some(repo) = &state.repo {
                    if let Some(new_idx) = repo.graph.iter().position(|r| r.commit_id == id) {
                        state.log.list_state.select(Some(new_idx));
                        // If we are looking at this commit, maybe refresh its diff just in case
                        return handle_selection(state);
                    } else {
                        // Current selection disappeared
                        state.log.list_state.select(Some(0));
                        state.log.current_diff = None;
                        return handle_selection(state);
                    }
                }
            } else if state.log.list_state.selected().is_none() {
                state.log.list_state.select(Some(0));
                return handle_selection(state);
            }
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
                update_repository_derived_state(state);
            }
        }
        Action::DiffLoaded(commit_id, diff) => {
            state.log.diff_cache.insert(commit_id, diff.clone());
            // Only update current_diff if it matches the current selection
            if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    if row.commit_id
                        == *state
                            .log
                            .diff_cache
                            .keys()
                            .find(|k| **k == row.commit_id)
                            .unwrap_or(&row.commit_id)
                    {
                        state.log.current_diff = Some(diff);
                        state.log.is_loading_diff = false;
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
            // Remove the oldest task (FIFO) rather than pop() (LIFO),
            // so overlapping operations remove the correct entry.
            if !state.active_tasks.is_empty() {
                state.active_tasks.remove(0);
            }
            if state.mode == AppMode::Loading {
                state.mode = if state.repo.is_some() {
                    AppMode::Normal
                } else {
                    AppMode::NoRepo
                };
            }
            match result {
                Ok(msg) => {
                    state.status_message = Some(msg);
                    state.status_clear_time = Some(Instant::now() + STATUS_CLEAR_DURATION);
                    state.log.diff_cache.clear(); // Clear cache as operations might change history
                    return Some(Command::LoadRepo(None, 100, state.revset.clone()));
                }
                Err(err) => {
                    state.last_error = Some(ErrorState {
                        suggestions: recovery::get_suggestions(&err),
                        message: err,
                        timestamp: Local::now(),
                        severity: ErrorSeverity::Error,
                    });
                    if state.repo.is_some() {
                        return Some(Command::LoadRepo(None, 100, state.revset.clone()));
                    }
                }
            }
        }
        Action::ErrorOccurred(err) => {
            state.last_error = Some(ErrorState {
                suggestions: recovery::get_suggestions(&err),
                message: err,
                timestamp: Local::now(),
                severity: ErrorSeverity::Error,
            });
            if state.mode == AppMode::Loading {
                state.mode = if state.repo.is_some() {
                    AppMode::Normal
                } else {
                    AppMode::NoRepo
                };
            }
        }

        Action::ExternalChangeDetected => {
            state
                .active_tasks
                .push("Syncing in background...".to_string());
            return Some(Command::LoadRepoBackground(100, state.revset.clone()));
        }

        Action::Tick => {
            state.frame_count = state.frame_count.wrapping_add(1);
            if let Some(clear_time) = state.status_clear_time {
                if Instant::now() >= clear_time {
                    state.status_message = None;
                    state.status_clear_time = None;
                }
            }
            if let Some(highlight_time) = state.hunk_highlight_time {
                if highlight_time.elapsed() >= Duration::from_millis(200) {
                    state.hunk_highlight_time = None;
                }
            }
            update_spinner(state);

            // Pagination: check if we are near the end of the graph
            if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                if idx + 20 >= repo.graph.len() && !state.is_loading_more && state.has_more {
                    // We need to trigger LoadMoreGraph. Reducer can't dispatch actions.
                    // But we can return a command!
                    // Wait, Tick doesn't usually return a command, but it can.
                    // Let's check update() signature.
                    return update(state, Action::LoadMoreGraph);
                }
            }
        }

        Action::Render | Action::Resize(_, _) => {}
    }
    None
}

fn move_selection(state: &mut AppState, delta: isize) -> Option<Command> {
    let len = state.repo.as_ref().map(|r| r.graph.len()).unwrap_or(0);
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

fn handle_selection(state: &mut AppState) -> Option<Command> {
    if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
        if let Some(row) = repo.graph.get(idx) {
            let commit_id = row.commit_id.clone();
            state.log.diff_scroll = 0; // Reset scroll on selection change
            state.log.selected_file_index = None;
            if let Some(cached_diff) = state.log.diff_cache.get(&commit_id) {
                state.log.current_diff = Some(cached_diff.clone());
                state.log.is_loading_diff = false;
                return None;
            } else {
                state.log.current_diff = None;
                state.log.is_loading_diff = true;
                return Some(Command::LoadDiff(commit_id));
            }
        }
    }
    None
}

fn update_spinner(state: &mut AppState) {
    let spin_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    state.spinner = spin_chars[(state.frame_count % spin_chars.len() as u64) as usize].to_string();
}

fn update_repository_derived_state(state: &mut AppState) {
    // Update header state
    if let Some(repo) = &mut state.repo {
        let mutable_count = repo.graph.iter().filter(|r| !r.is_immutable).count();
        let immutable_count = repo.graph.iter().filter(|r| r.is_immutable).count();

        let repo_name = if repo.repo_name.is_empty() {
            "no repo".to_string()
        } else {
            repo.repo_name.clone()
        };
        state.header_state.repo_text = format!(" {} ", repo_name);

        // Find branch/bookmark of working copy
        let wc_id = &repo.working_copy_id;
        let branch_name = repo
            .graph
            .iter()
            .find(|r| r.commit_id == *wc_id)
            .and_then(|r| r.bookmarks.first())
            .cloned()
            .unwrap_or_else(|| "(detached)".to_string());
        state.header_state.branch_text =
            format!(" {} {} ", crate::theme::glyphs::BRANCH, branch_name);

        let short_op = &repo.operation_id[..8.min(repo.operation_id.len())];
        state.header_state.op_text = format!(" OP: {} ", short_op);

        let short_wc = &repo.working_copy_id.0[..8.min(repo.working_copy_id.0.len())];
        state.header_state.wc_text = format!(" WC: {} ", short_wc);

        state.header_state.stats_text =
            format!(" | Mut: {} Imm: {} ", mutable_count, immutable_count);

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

            // Track continuing lanes before we modify active_commits for parents
            let mut continuing = Vec::new();
            for (i, lane) in active_commits.iter().enumerate() {
                if i != current_lane && lane.is_some() {
                    continuing.push(i);
                }
            }

            // Prepare lanes for parents (and for the connector lines)
            active_commits[current_lane] = None;

            let mut parent_cols = Vec::new();
            for parent in &row.parents {
                let parent_id = &parent.0;
                let parent_lane = if let Some(pos) = active_commits
                    .iter()
                    .position(|l| l.as_ref() == Some(parent_id))
                {
                    pos
                } else if let Some(pos) = active_commits.iter().position(|l| l.is_none()) {
                    active_commits[pos] = Some(parent_id.clone());
                    pos
                } else {
                    active_commits.push(Some(parent_id.clone()));
                    active_commits.len() - 1
                };
                parent_cols.push(parent_lane);
            }
            row.visual.parent_columns = parent_cols.clone();
            row.visual.parent_min = parent_cols
                .iter()
                .cloned()
                .min()
                .unwrap_or(row.visual.column)
                .min(row.visual.column);
            row.visual.parent_max = parent_cols
                .iter()
                .cloned()
                .max()
                .unwrap_or(row.visual.column)
                .max(row.visual.column);

            // Map continuing lanes to their new positions (they shouldn't move in this simple model)
            row.visual.continuing_lanes = continuing.into_iter().map(|i| (i, i)).collect();

            row.visual.connector_lanes = active_commits.iter().map(|l| l.is_some()).collect();
        }
    } else {
        state.header_state = HeaderState::default();
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state;
    use crate::domain::models::{CommitId, FileChange, FileStatus, GraphRow, RepoStatus};

    fn create_dummy_repo(count: usize) -> RepoStatus {
        let mut graph = Vec::new();
        for i in 0..count {
            graph.push(GraphRow {
                timestamp_secs: 0,
                commit_id: CommitId(format!("commit-{}", i)),
                commit_id_short: format!("c{}", i),
                change_id: format!("change-{}", i),
                change_id_short: format!("ch{}", i),
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
            repo_name: "test-repo".to_string(),
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
        assert_eq!(state.log.list_state.selected(), Some(0));

        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log.list_state.selected(), Some(0));

        // 2. Repo with 3 items
        state.repo = Some(create_dummy_repo(3));
        state.log.list_state.select(None); // Reset

        // Initial Next from None -> 0
        update(&mut state, Action::SelectNext);
        assert_eq!(state.log.list_state.selected(), Some(0));

        // Next -> 1
        update(&mut state, Action::SelectNext);
        assert_eq!(state.log.list_state.selected(), Some(1));

        // Next -> 2
        update(&mut state, Action::SelectNext);
        assert_eq!(state.log.list_state.selected(), Some(2));

        // Next (Wrap) -> 0
        update(&mut state, Action::SelectNext);
        assert_eq!(state.log.list_state.selected(), Some(0));

        // Prev (Wrap) -> 2
        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log.list_state.selected(), Some(2));

        // Prev -> 1
        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log.list_state.selected(), Some(1));

        // Test None selection behavior for Prev
        state.log.list_state.select(None);
        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log.list_state.selected(), Some(0));
    }

    #[test]
    fn test_scroll_diff() {
        let mut state = AppState::default();
        state.log.diff_scroll = 10;

        // Test ScrollDiffUp normal
        update(&mut state, Action::ScrollDiffUp(5));
        assert_eq!(state.log.diff_scroll, 5);

        // Test ScrollDiffUp saturating
        update(&mut state, Action::ScrollDiffUp(10));
        assert_eq!(state.log.diff_scroll, 0);

        // Test ScrollDiffDown
        update(&mut state, Action::ScrollDiffDown(15));
        assert_eq!(state.log.diff_scroll, 15);
    }

    fn create_mock_repo(num_rows: usize) -> RepoStatus {
        let graph = (0..num_rows)
            .map(|i| GraphRow {
                timestamp_secs: 0,
                commit_id: CommitId(format!("commit{}", i)),
                commit_id_short: format!("c{}", i),
                change_id: format!("change{}", i),
                change_id_short: format!("ch{}", i),
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
            repo_name: "test-repo".to_string(),
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
        state.log.list_state.select(Some(1));

        // Test SelectNext
        update(&mut state, Action::SelectNext);
        assert_eq!(state.log.list_state.selected(), Some(2));

        // Test SelectPrev
        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log.list_state.selected(), Some(1));
    }

    #[test]
    fn test_navigation_wrapping() {
        let num_rows = 3;
        let mut state = AppState {
            repo: Some(create_mock_repo(num_rows)),
            ..Default::default()
        };

        // Wrap from end to beginning
        state.log.list_state.select(Some(num_rows - 1));
        update(&mut state, Action::SelectNext);
        assert_eq!(state.log.list_state.selected(), Some(0));

        // Wrap from beginning to end
        state.log.list_state.select(Some(0));
        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log.list_state.selected(), Some(num_rows - 1));
    }

    #[test]
    fn test_navigation_empty_list() {
        let mut state = AppState {
            repo: Some(create_mock_repo(0)),
            ..Default::default()
        };

        // In both cases, it should default to 0 and not panic/underflow
        state.log.list_state.select(Some(0));
        update(&mut state, Action::SelectNext);
        assert_eq!(state.log.list_state.selected(), Some(0));

        state.log.list_state.select(Some(0));
        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log.list_state.selected(), Some(0));
    }

    #[test]
    fn test_text_area_input() {
        let mut state = AppState {
            input: Some(state::InputState {
                text_area: AppTextArea::default(),
            }),
            ..Default::default()
        };
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        update(&mut state, Action::TextAreaInput(key));

        assert_eq!(state.input.as_ref().unwrap().text_area.lines()[0], "a");
    }

    #[test]
    fn test_clear_error_on_cancel_mode() {
        let mut state = AppState {
            last_error: Some(ErrorState {
                message: "An error occurred".to_string(),
                timestamp: Local::now(),
                severity: ErrorSeverity::Error,
                suggestions: vec![],
            }),
            mode: AppMode::Input,
            ..Default::default()
        };

        update(&mut state, Action::CancelMode);

        assert_eq!(state.last_error, None);
        assert_eq!(state.mode, AppMode::Normal);
        assert!(state.input.is_none());
    }

    #[test]
    fn test_refresh_derived_state() {
        let mut state = AppState {
            repo: Some(create_mock_repo(2)),
            ..Default::default()
        };

        // Before refresh
        assert_eq!(state.header_state.repo_text, " no repo ");

        update_repository_derived_state(&mut state);

        // After refresh
        assert_eq!(state.header_state.op_text, " OP: op ");
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
        state.log.list_state.select(Some(0));

        // Initial state
        assert_eq!(state.log.selected_file_index, None);

        // Select next file
        update(&mut state, Action::SelectNextFile);
        assert_eq!(state.log.selected_file_index, Some(1)); // (0+1)%2 = 1

        update(&mut state, Action::SelectNextFile);
        assert_eq!(state.log.selected_file_index, Some(0)); // (1+1)%2 = 0

        // Select prev file
        update(&mut state, Action::SelectPrevFile);
        assert_eq!(state.log.selected_file_index, Some(1)); // (0-1)%2 = 1

        // Reset on commit change
        state.repo.as_mut().unwrap().graph.push(GraphRow {
            timestamp_secs: 0,
            commit_id: CommitId("c2".to_string()),
            commit_id_short: "c2".to_string(),
            change_id: "ch2".to_string(),
            change_id_short: "ch2".to_string(),
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
        assert_eq!(state.log.selected_file_index, None);
    }

    #[test]
    fn test_navigation_boundaries_and_empty_state() {
        // Test with empty repo
        let mut state = AppState::<'_> {
            repo: Some(create_mock_repo(0)),
            ..Default::default()
        };

        update(&mut state, Action::SelectNext);
        assert!(state.log.list_state.selected().unwrap_or(0) < 1);

        update(&mut state, Action::SelectPrev);
        assert!(state.log.list_state.selected().unwrap_or(0) < 1);

        // Test with 1 item repo
        state.repo = Some(create_mock_repo(1));
        state.log.list_state.select(Some(0));

        update(&mut state, Action::SelectNext);
        assert_eq!(state.log.list_state.selected(), Some(0));

        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log.list_state.selected(), Some(0));

        // Test with large deltas (if we had them, but we use 1/-1)
        // Ensure wrap-around is consistent
        state.repo = Some(create_mock_repo(10));
        state.log.list_state.select(Some(9));
        update(&mut state, Action::SelectNext);
        assert_eq!(state.log.list_state.selected(), Some(0));

        state.log.list_state.select(Some(0));
        update(&mut state, Action::SelectPrev);
        assert_eq!(state.log.list_state.selected(), Some(9));
    }

    #[test]
    fn test_comprehensive_esc_behavior() {
        let modes = [
            AppMode::CommandPalette,
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
                last_error: Some(ErrorState {
                    message: "error".to_string(),
                    timestamp: Local::now(),
                    severity: ErrorSeverity::Error,
                    suggestions: vec![],
                }),
                context_menu: Some(state::ContextMenuState {
                    commit_id: CommitId("c1".to_string()),
                    x: 0,
                    y: 0,
                    selected_index: 0,
                    actions: vec![],
                }),
                ..Default::default()
            };
            // Set some text in text area
            state.input = Some(state::InputState {
                text_area: AppTextArea::default(),
            });
            state
                .input
                .as_mut()
                .unwrap()
                .text_area
                .insert_str("some text");

            update(&mut state, Action::CancelMode);

            assert_eq!(
                state.mode,
                AppMode::Normal,
                "Mode should reset to Normal from {:?}",
                mode
            );
            assert_eq!(
                state.last_error, None,
                "Error should be cleared from {:?}",
                mode
            );
            assert_eq!(
                state.context_menu, None,
                "Context menu should be cleared from {:?}",
                mode
            );
            assert!(
                state.input.is_none(),
                "Input should be cleared from {:?}",
                mode
            );
        }
    }

    #[test]
    fn test_dynamic_diff_ratio() {
        let mut state = AppState::<'_> {
            show_diffs: true,
            ..Default::default()
        };
        assert_eq!(state.diff_ratio, 50);

        // Focus Diff
        update(&mut state, Action::FocusDiff);
        assert_eq!(state.mode, AppMode::Diff);
        assert_eq!(state.diff_ratio, 70);

        // Focus Graph
        update(&mut state, Action::FocusGraph);
        assert_eq!(state.mode, AppMode::Normal);
        assert_eq!(state.diff_ratio, 30);
    }

    #[test]
    fn test_toggle_help() {
        let mut state = AppState::default();
        assert_eq!(state.mode, AppMode::Normal);

        update(&mut state, Action::ToggleHelp);
        assert_eq!(state.mode, AppMode::Help);

        update(&mut state, Action::ToggleHelp);
        assert_eq!(state.mode, AppMode::Normal);

        // Ensure CancelMode also exits Help
        update(&mut state, Action::ToggleHelp);
        assert_eq!(state.mode, AppMode::Help);
        update(&mut state, Action::CancelMode);
        assert_eq!(state.mode, AppMode::Normal);
    }

    #[test]
    fn test_refresh_derived_state_merge() {
        // A (0) -> B (0), C (1)
        let graph = vec![
            GraphRow {
                commit_id: CommitId("A".to_string()),
                commit_id_short: "A".to_string(),
                parents: vec![CommitId("B".to_string()), CommitId("C".to_string())],
                ..GraphRow::default()
            },
            // B (0) -> D (0)
            GraphRow {
                commit_id: CommitId("B".to_string()),
                commit_id_short: "B".to_string(),
                parents: vec![CommitId("D".to_string())],
                ..GraphRow::default()
            },
            // C (1) -> D (0)
            GraphRow {
                commit_id: CommitId("C".to_string()),
                commit_id_short: "C".to_string(),
                parents: vec![CommitId("D".to_string())],
                ..GraphRow::default()
            },
            // D (0)
            GraphRow {
                commit_id: CommitId("D".to_string()),
                commit_id_short: "D".to_string(),
                parents: vec![],
                ..GraphRow::default()
            },
        ];

        let mut state = AppState {
            repo: Some(RepoStatus {
                repo_name: "test-repo".to_string(),
                operation_id: "op".to_string(),
                workspace_id: "ws".to_string(),
                working_copy_id: CommitId("A".to_string()),
                graph,
            }),
            ..Default::default()
        };

        update_repository_derived_state(&mut state);

        let repo = state.repo.as_ref().unwrap();

        // Row A: node at 0, parents [0, 1]
        assert_eq!(repo.graph[0].visual.column, 0);
        assert_eq!(repo.graph[0].visual.parent_columns, vec![0, 1]);

        // Row B: node at 0, parent [0]. Lane 1 (C) is active.
        assert_eq!(repo.graph[1].visual.column, 0);
        assert_eq!(repo.graph[1].visual.parent_columns, vec![0]);
        assert_eq!(repo.graph[1].visual.active_lanes, vec![true, true]);

        // Row C: node at 1, parent [0]. Lane 0 (D, from B) is active.
        assert_eq!(repo.graph[2].visual.column, 1);
        assert_eq!(repo.graph[2].visual.parent_columns, vec![0]);
        assert_eq!(repo.graph[2].visual.active_lanes, vec![true, true]);

        // Row D: node at 0, no parents.
        assert_eq!(repo.graph[3].visual.column, 0);
        assert!(repo.graph[3].visual.parent_columns.is_empty());
    }

    fn create_repo_with_bookmarks(bookmarks: Vec<String>) -> RepoStatus {
        let graph = vec![GraphRow {
            commit_id: CommitId("commit0".to_string()),
            commit_id_short: "c0".to_string(),
            change_id: "change0".to_string(),
            change_id_short: "ch0".to_string(),
            description: "desc".to_string(),
            author: "author".to_string(),
            timestamp: "time".to_string(),
            timestamp_secs: 0,
            is_working_copy: true,
            is_immutable: false,
            parents: vec![],
            bookmarks,
            changed_files: vec![],
            visual: crate::domain::models::GraphRowVisual::default(),
        }];
        RepoStatus {
            repo_name: "test-repo".to_string(),
            operation_id: "op".to_string(),
            workspace_id: "default".to_string(),
            working_copy_id: CommitId("commit0".to_string()),
            graph,
        }
    }

    #[test]
    fn test_delete_bookmark_intent_single() {
        let mut state = AppState {
            repo: Some(create_repo_with_bookmarks(vec!["main".to_string()])),
            ..Default::default()
        };
        state.log.list_state.select(Some(0));

        let cmd = update(&mut state, Action::DeleteBookmarkIntent);
        assert!(
            matches!(cmd, Some(Command::DeleteBookmark(ref name)) if name == "main"),
            "Single bookmark should produce DeleteBookmark command"
        );
    }

    #[test]
    fn test_delete_bookmark_intent_multiple() {
        let mut state = AppState {
            repo: Some(create_repo_with_bookmarks(vec![
                "main".to_string(),
                "dev".to_string(),
            ])),
            ..Default::default()
        };
        state.log.list_state.select(Some(0));

        let cmd = update(&mut state, Action::DeleteBookmarkIntent);
        assert!(
            cmd.is_none(),
            "Multiple bookmarks should open menu, not return command"
        );
        assert_eq!(state.mode, AppMode::ContextMenu);
        let menu = state.context_menu.as_ref().unwrap();
        assert_eq!(menu.actions.len(), 2);
        assert!(menu.actions[0].0.contains("main"));
        assert!(menu.actions[1].0.contains("dev"));
    }

    #[test]
    fn test_delete_bookmark_intent_none() {
        let mut state = AppState {
            repo: Some(create_repo_with_bookmarks(vec![])),
            ..Default::default()
        };
        state.log.list_state.select(Some(0));

        let cmd = update(&mut state, Action::DeleteBookmarkIntent);
        assert!(cmd.is_none(), "No bookmarks should be a no-op");
        assert_ne!(state.mode, AppMode::ContextMenu);
    }

    #[test]
    fn test_delete_bookmark_resets_mode() {
        let mut state = AppState {
            mode: AppMode::ContextMenu,
            ..Default::default()
        };

        let cmd = update(&mut state, Action::DeleteBookmark("main".to_string()));
        assert_eq!(state.mode, AppMode::Normal);
        assert!(matches!(cmd, Some(Command::DeleteBookmark(ref name)) if name == "main"));
    }
}
