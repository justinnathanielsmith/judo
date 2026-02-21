use crate::domain::models::{CommitId, RepoStatus};
use anyhow::Result;
use async_trait::async_trait;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait VcsFacade: Send + Sync {
    // Returns the graph for the main view
    async fn get_operation_log(
        &self,
        heads: Option<Vec<CommitId>>,
        limit: usize,
        revset: Option<String>,
    ) -> Result<RepoStatus>;

    // Get diff for a specific commit
    async fn get_commit_diff(&self, commit_id: &CommitId) -> Result<String>;

    // JJ specific: "Describe"
    async fn describe_revision(&self, change_id: &str, message: &str) -> Result<()>;

    // Snapshot
    async fn snapshot(&self) -> Result<String>;

    async fn edit(&self, commit_id: &CommitId) -> Result<()>;
    async fn squash(&self, commit_id: &CommitId) -> Result<()>;
    async fn new_child(&self, commit_id: &CommitId) -> Result<()>;
    async fn abandon(&self, commit_id: &CommitId) -> Result<()>;
    async fn set_bookmark(&self, commit_id: &CommitId, name: &str) -> Result<()>;
    async fn delete_bookmark(&self, name: &str) -> Result<()>;

    async fn undo(&self) -> Result<()>;
    async fn redo(&self) -> Result<()>;

    async fn fetch(&self) -> Result<()>;
    async fn push(&self, bookmark: Option<String>) -> Result<()>;

    fn workspace_root(&self) -> std::path::PathBuf;
}
