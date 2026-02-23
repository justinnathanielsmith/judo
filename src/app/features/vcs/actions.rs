use crate::app::{
    action::{Action, UpdateResult},
    command::Command,
    state::{AppMode, AppState, AppTextArea, ErrorSeverity, ErrorState},
};
use crate::domain::models::FileStatus;
use chrono::Local;

pub fn update(state: &mut AppState, action: &Action) -> UpdateResult {
    match action {
        Action::SnapshotWorkingCopy => UpdateResult::Handled(Some(Command::Snapshot)),
        Action::EditRevision(commit_id_opt) => {
            let id = commit_id_opt.clone().or_else(|| {
                let repo = state.repo.as_ref()?;
                let idx = state.log.list_state.selected()?;
                repo.graph.get(idx).map(|r| r.commit_id.clone())
            });
            UpdateResult::Handled(id.map(Command::Edit))
        }
        Action::SquashRevision => {
            let ids = state.get_selected_commit_ids();
            if ids.is_empty() {
                UpdateResult::Handled(None)
            } else {
                state.log.selected_ids.clear();
                UpdateResult::Handled(Some(Command::Squash(ids)))
            }
        }
        Action::NewRevision(commit_id_opt) => {
            let id = commit_id_opt.clone().or_else(|| {
                let repo = state.repo.as_ref()?;
                let idx = state.log.list_state.selected()?;
                repo.graph.get(idx).map(|r| r.commit_id.clone())
            });
            UpdateResult::Handled(id.map(Command::New))
        }
        Action::DescribeRevisionIntent => {
            state.mode = AppMode::Input;
            let mut text_area = AppTextArea::default();
            if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    text_area.insert_str(&row.description);
                }
            }
            state.input = Some(crate::app::state::InputState { text_area });
            UpdateResult::Handled(None)
        }
        Action::DescribeRevision(commit_id, message) => {
            state.mode = AppMode::Normal;
            state.input = None;
            UpdateResult::Handled(Some(Command::DescribeRevision(
                commit_id.clone(),
                message.clone(),
            )))
        }
        Action::CommitWorkingCopyIntent => {
            if let Some(repo) = &state.repo {
                if let Some(row) = repo.graph.iter().find(|r| r.is_working_copy) {
                    if row
                        .changed_files
                        .iter()
                        .any(|f| f.status == FileStatus::Conflicted)
                    {
                        state.last_error = Some(ErrorState {
                            suggestions: vec![
                                "Resolve conflicts using 'm' (or 'jj resolve') before committing."
                                    .to_string(),
                            ],
                            message: "Cannot commit: there are unresolved merge conflicts."
                                .to_string(),
                            timestamp: Local::now(),
                            severity: ErrorSeverity::Error,
                        });
                        return UpdateResult::Handled(None);
                    }
                }
            }
            state.mode = AppMode::CommitInput;
            let mut text_area = AppTextArea::default();
            if let Some(repo) = &state.repo {
                if let Some(row) = repo.graph.iter().find(|r| r.is_working_copy) {
                    text_area.insert_str(&row.description);
                }
            }
            state.input = Some(crate::app::state::InputState { text_area });
            UpdateResult::Handled(None)
        }
        Action::CommitWorkingCopy(message) => {
            state.mode = AppMode::Normal;
            state.input = None;
            UpdateResult::Handled(Some(Command::Commit(message.clone())))
        }
        Action::AbandonRevision(commit_id_opt) => {
            let ids = if let Some(id) = commit_id_opt {
                vec![id.clone()]
            } else {
                state.get_selected_commit_ids()
            };
            if ids.is_empty() {
                UpdateResult::Handled(None)
            } else {
                state.log.selected_ids.clear();
                UpdateResult::Handled(Some(Command::Abandon(ids)))
            }
        }
        Action::RevertRevision(ids) => {
            state.log.selected_ids.clear();
            UpdateResult::Handled(Some(Command::Revert(ids.clone())))
        }
        Action::Absorb => UpdateResult::Handled(Some(Command::Absorb)),
        Action::DuplicateRevision => {
            let ids = state.get_selected_commit_ids();
            if ids.is_empty() {
                UpdateResult::Handled(None)
            } else {
                state.log.selected_ids.clear();
                UpdateResult::Handled(Some(Command::Duplicate(ids)))
            }
        }
        Action::ParallelizeRevision => {
            let ids = state.get_selected_commit_ids();
            if ids.is_empty() {
                UpdateResult::Handled(None)
            } else {
                state.log.selected_ids.clear();
                UpdateResult::Handled(Some(Command::Parallelize(ids)))
            }
        }
        Action::RebaseRevisionIntent => {
            let ids = state.get_selected_commit_ids();
            if ids.is_empty() {
                UpdateResult::Handled(None)
            } else {
                state.rebase_sources = ids;
                state.mode = AppMode::RebaseSelect;
                state.log.selected_ids.clear();
                UpdateResult::Handled(None)
            }
        }
        Action::RebaseRevision(sources, destination) => {
            state.mode = AppMode::Normal;
            state.rebase_sources.clear();
            UpdateResult::Handled(Some(Command::Rebase(
                sources.clone(),
                destination.clone(),
            )))
        }
        Action::SetBookmarkIntent => {
            state.mode = AppMode::BookmarkInput;
            state.input = Some(crate::app::state::InputState {
                text_area: AppTextArea::default(),
            });
            UpdateResult::Handled(None)
        }
        Action::SetBookmark(commit_id, name) => {
            state.mode = AppMode::Normal;
            UpdateResult::Handled(Some(Command::SetBookmark(commit_id.clone(), name.clone())))
        }
        Action::DeleteBookmark(name) => {
            state.mode = AppMode::Normal;
            UpdateResult::Handled(Some(Command::DeleteBookmark(name.clone())))
        }
        Action::SplitRevision(commit_id_opt) => {
            let id = commit_id_opt.clone().or_else(|| {
                let repo = state.repo.as_ref()?;
                let idx = state.log.list_state.selected()?;
                repo.graph.get(idx).map(|r| r.commit_id.clone())
            });
            UpdateResult::Handled(id.map(Command::Split))
        }
        Action::Undo => UpdateResult::Handled(Some(Command::Undo)),
        Action::Redo => UpdateResult::Handled(Some(Command::Redo)),
        Action::Fetch => UpdateResult::Handled(Some(Command::Fetch)),
        Action::PushIntent => {
            if let (Some(repo), Some(idx)) = (&state.repo, state.log.list_state.selected()) {
                if let Some(row) = repo.graph.get(idx) {
                    if row.bookmarks.is_empty() {
                        return UpdateResult::Handled(Some(Command::Push(None)));
                    } else if row.bookmarks.len() == 1 {
                        return UpdateResult::Handled(Some(Command::Push(Some(
                            row.bookmarks[0].clone(),
                        ))));
                    }
                    // Multiple bookmarks: open context menu for selection
                    let mut actions = Vec::new();
                    for bookmark in &row.bookmarks {
                        actions.push((
                            format!("Push bookmark: {bookmark}"),
                            Action::Push(Some(bookmark.clone())),
                        ));
                    }
                    actions.push(("Push All".to_string(), Action::Push(None)));

                    state.mode = AppMode::ContextMenu;
                    state.context_menu = Some(crate::app::state::ContextMenuState {
                        commit_id: row.commit_id.clone(),
                        x: 10,
                        y: 10,
                        selected_index: 0,
                        actions,
                    });
                }
            }
            UpdateResult::Handled(None)
        }
        Action::Push(bookmark) => UpdateResult::Handled(Some(Command::Push(bookmark.clone()))),
        Action::ResolveConflict(path) => {
            UpdateResult::Handled(Some(Command::ResolveConflict(path.clone())))
        }
        Action::InitRepo => UpdateResult::Handled(Some(Command::InitRepo)),
        _ => UpdateResult::NotHandled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::action::Action;
    use crate::app::keymap::KeyConfig;
    use crate::app::state::{AppMode, AppState};
    use crate::domain::models::{CommitId, FileChange, FileStatus, GraphRow, RepoStatus};

    #[test]
    fn test_commit_intent_blocked_by_conflicts() {
        let mut state = AppState::new(KeyConfig::default());

        // Setup a repo with a conflicted working copy
        let mut repo = RepoStatus {
            repo_name: "test".to_string(),
            operation_id: "op".to_string(),
            workspace_id: "ws".to_string(),
            working_copy_id: CommitId("abc".to_string()),
            graph: vec![],
        };
        let mut row = GraphRow::default();
        row.is_working_copy = true;
        row.changed_files.push(FileChange {
            path: "conflict.txt".to_string(),
            status: FileStatus::Conflicted,
        });
        repo.graph.push(row);
        state.repo = Some(repo);

        let result = update(&mut state, &Action::CommitWorkingCopyIntent);

        assert!(matches!(result, UpdateResult::Handled(None)));
        assert_ne!(state.mode, AppMode::CommitInput);
        assert!(state.last_error.is_some());
        assert!(state
            .last_error
            .as_ref()
            .unwrap()
            .message
            .contains("conflicts"));
    }
}
