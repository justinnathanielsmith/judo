use crate::domain::models::{CommitId, RepoStatus};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait VcsFacade: Send + Sync {
    // Returns the graph for the main view
    async fn get_operation_log(&self) -> Result<RepoStatus>;

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
}
