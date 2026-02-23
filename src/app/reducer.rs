use crate::app::features::{filter, navigation, ui, vcs};
use crate::domain::graph_layout;
use crate::app::{
    action::{Action, UpdateResult},
    command::Command,
    recovery,
    state::{AppMode, AppState, ErrorSeverity, ErrorState, Panel},
};
use chrono::Local;
use std::time::{Duration, Instant};

const STATUS_CLEAR_DURATION: Duration = Duration::from_secs(5);

pub fn update(state: &mut AppState, action: Action) -> Option<Command> {
    // 1. Feature delegation with short-circuit on handled
    match navigation::update(state, &action) {
        UpdateResult::Handled(cmd) => return cmd,
        UpdateResult::NotHandled => {}
    }
    match vcs::update(state, &action) {
        UpdateResult::Handled(cmd) => return cmd,
        UpdateResult::NotHandled => {}
    }
    match filter::update(state, &action) {
        UpdateResult::Handled(cmd) => return cmd,
        UpdateResult::NotHandled => {}
    }
    match ui::update(state, &action) {
        UpdateResult::Handled(cmd) => return cmd,
        UpdateResult::NotHandled => {}
    }

    // 2. Main Reducer (Lifecycle, Results, Special Cases)
    match action {
        Action::Quit => {
            state.should_quit = true;
            return None;
        }

        Action::SelectContextMenuAction(idx) => {
            if let Some(menu) = &state.context_menu {
                if let Some((_, action)) = menu.actions.get(idx).cloned() {
                    state.context_menu = None;
                    state.mode = AppMode::Normal;
                    return update(state, action);
                }
            }
        }

        Action::CommandPaletteSelect => {
            if let Some(cp) = &state.command_palette {
                if let Some(&cmd_idx) = cp.matches.get(cp.selected_index) {
                    let cmd_def = crate::app::command_palette::get_commands().get(cmd_idx).cloned();
                    if let Some(cmd_def) = cmd_def {
                        state.mode = AppMode::Normal;
                        state.command_palette = None;
                        return update(state, cmd_def.action);
                    }
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
                if heads.is_empty() {
                    state.has_more = false;
                } else {
                    state.is_loading_more = true;
                    heads.sort_by(|a, b| a.0.cmp(&b.0));
                    heads.dedup();
                    return Some(Command::LoadRepo(None, 100, state.revset.clone()));
                }
            }
        }

        Action::RepoLoaded(repo_status) => {
            state.workspace_id = repo_status.workspace_id.clone();
            state.repo = Some(*repo_status);
            // Recompute graph layout for lane/connector rendering
            if let Some(repo) = &mut state.repo {
                graph_layout::calculate_graph_layout(&mut repo.graph);
            }
            state.is_loading_more = false;
            state.has_more = true;
            if state.mode == AppMode::Loading || state.mode == AppMode::NoRepo {
                state.mode = match state.focused_panel {
                    Panel::Graph => AppMode::Normal,
                    Panel::Diff => AppMode::Diff,
                };
            }
            state.active_tasks.retain(|t| !t.contains("Syncing"));
            update_repository_derived_state(state);
            if state.log.list_state.selected().is_none() {
                state.log.list_state.select(Some(0));
            }
            return navigation::handle_selection(state);
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
            // Recompute graph layout since repo graph was refreshed
            if let Some(repo) = &mut state.repo {
                graph_layout::calculate_graph_layout(&mut repo.graph);
            }
            state.is_loading_more = false;
            state.has_more = true;
            update_repository_derived_state(state);

            if let Some(id) = selected_commit_id {
                if let Some(repo) = &state.repo {
                    if let Some(new_idx) = repo.graph.iter().position(|r| r.commit_id == id) {
                        state.log.list_state.select(Some(new_idx));
                        return navigation::handle_selection(state);
                    }
                    state.log.list_state.select(Some(0));
                    state.log.current_diff = None;
                    return navigation::handle_selection(state);
                }
            } else if state.log.list_state.selected().is_none() {
                state.log.list_state.select(Some(0));
                return navigation::handle_selection(state);
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
                graph_layout::calculate_graph_layout(&mut repo.graph);
                update_repository_derived_state(state);
            }
        }

        Action::DiffLoaded(commit_id, diff) => {
            state.log.diff_cache.insert(commit_id.clone(), diff.clone());
            if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    if row.commit_id == commit_id {
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
            if !state.active_tasks.is_empty() {
                state.active_tasks.remove(0);
            }
            if state.mode == AppMode::Loading {
                state.mode = if state.repo.is_some() {
                    match state.focused_panel {
                        Panel::Graph => AppMode::Normal,
                        Panel::Diff => AppMode::Diff,
                    }
                } else {
                    AppMode::NoRepo
                };
            }
            match result {
                Ok(msg) => {
                    state.status_message = Some(msg);
                    state.status_clear_time = Some(Instant::now() + STATUS_CLEAR_DURATION);
                    state.log.diff_cache.clear();
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
            let err_lower = err.to_lowercase();
            let is_revset_error = state.revset.is_some()
                && (err_lower.contains("revset")
                    || err_lower.contains("parse error")
                    || (err_lower.contains("error") && err_lower.contains("function"))
                    || (err_lower.contains("invalid") && err_lower.contains("expression")));

            if is_revset_error {
                state.revset = None;
            }

            state.last_error = Some(ErrorState {
                suggestions: recovery::get_suggestions(&err),
                message: err,
                timestamp: Local::now(),
                severity: ErrorSeverity::Error,
            });
            if state.mode == AppMode::Loading {
                state.mode = if state.repo.is_some() {
                    match state.focused_panel {
                        Panel::Graph => AppMode::Normal,
                        Panel::Diff => AppMode::Diff,
                    }
                } else {
                    AppMode::NoRepo
                };
            }

            if is_revset_error {
                return Some(Command::LoadRepo(None, 100, None));
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

            if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                if idx + 20 >= repo.graph.len() && !state.is_loading_more && state.has_more {
                    return update(state, Action::LoadMoreGraph);
                }
            }
        }

        Action::Render | Action::Resize(_, _) => {}
        _ => {}
    }
    None
}

fn update_spinner(state: &mut AppState) {
    let spinner_frames = vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let idx = (state.frame_count / 5) as usize % spinner_frames.len();
    state.spinner = spinner_frames[idx].to_string();
}

fn update_repository_derived_state(state: &mut AppState) {
    if let Some(repo) = &state.repo {
        state.header_state.repo_text = format!(" {} ", repo.workspace_id);

        let wc = repo.graph.iter().find(|r| r.is_working_copy);
        if let Some(row) = wc {
            state.header_state.wc_text = format!(" {} ", row.commit_id);
            state.header_state.branch_text = row.bookmarks.join(", ");
        }

        state.header_state.stats_text = format!(
            " {} revs | {} selected ",
            repo.graph.len(),
            state.log.selected_ids.len()
        );
    }
}
