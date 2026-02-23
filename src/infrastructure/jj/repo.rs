use super::JjAdapter;
use crate::domain::models::CommitId;
use anyhow::{anyhow, Result};
use jj_lib::{
    backend::CommitId as JjCommitId,
    object_id::ObjectId,
    ref_name::WorkspaceName,
    repo::{ReadonlyRepo, Repo},
};
use std::path::PathBuf;
use std::sync::Arc;

impl JjAdapter {
    pub(crate) async fn validate_commit(&self, commit_id: &CommitId) -> Result<JjCommitId> {
        let (repo, _): (Arc<ReadonlyRepo>, _) = self.get_repo_and_ws().await?;
        let id = JjCommitId::try_from_hex(&commit_id.0)
            .ok_or_else(|| anyhow!("Invalid commit ID format: {}", commit_id.0))?;

        if !repo.index().has_id(&id).map_err(|e| anyhow!(e))? {
            return Err(anyhow!(
                "Commit {} is no longer valid or has been rewritten/abandoned.",
                commit_id.0
            ));
        }
        Ok(id)
    }

    pub(crate) async fn get_repo_and_ws(&self) -> Result<(Arc<ReadonlyRepo>, PathBuf)> {
        let ws_opt = self.workspace.lock().await;
        let ws = ws_opt
            .as_ref()
            .ok_or_else(|| anyhow!("No repository found"))?;
        let repo = ws.repo_loader().load_at_head()?;
        Ok((repo, ws.workspace_root().to_path_buf()))
    }

    pub(crate) async fn is_valid_impl(&self) -> bool {
        self.workspace.lock().await.is_some()
    }
}

pub(crate) struct CommitInfo {
    pub commit: jj_lib::commit::Commit,
    pub parent_tree: Option<jj_lib::merged_tree::MergedTree>,
    pub parent_ids: Vec<CommitId>,
    pub is_working_copy: bool,
    pub is_immutable: bool,
    pub has_conflict: bool,
    pub bookmarks: Vec<String>,
}

pub(crate) fn build_commit_info(
    repo: &ReadonlyRepo,
    id: &JjCommitId,
    ws_id: &WorkspaceName,
) -> Result<CommitInfo> {
    let commit = repo.store().get_commit(id)?;
    let mut parent_ids_domain = Vec::new();
    for parent_id in commit.parent_ids() {
        parent_ids_domain.push(CommitId(parent_id.hex()));
    }

    let first_parent: Option<jj_lib::commit::Commit> =
        commit.parents().next().transpose().unwrap_or_default();
    let parent_tree = first_parent.as_ref().map(jj_lib::commit::Commit::tree);
    let is_working_copy = Some(id) == repo.view().get_wc_commit_id(ws_id);
    // Heuristic: root commit is immutable.
    let is_immutable = commit.parents().next().is_none();
    let has_conflict = commit.tree().has_conflict();

    let bookmarks = repo
        .view()
        .local_bookmarks()
        .filter(|(_, target)| target.added_ids().any(|added_id| added_id == id))
        .map(|(name, _)| name.as_str().to_string())
        .collect::<Vec<_>>();

    Ok(CommitInfo {
        commit,
        parent_tree,
        parent_ids: parent_ids_domain,
        is_working_copy,
        is_immutable,
        has_conflict,
        bookmarks,
    })
}
