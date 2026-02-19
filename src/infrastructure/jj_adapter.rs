use crate::domain::{vcs::VcsFacade, models::{RepoStatus, CommitId}};
use anyhow::Result;
use async_trait::async_trait;

pub struct JjAdapter;

impl JjAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl VcsFacade for JjAdapter {
    async fn get_operation_log(&self) -> Result<RepoStatus> {
        // Stubbed data for now
        Ok(RepoStatus {
            operation_id: "stub-op-id".to_string(),
            working_copy_id: CommitId("abcdef".to_string()),
            graph: vec![],
        })
    }

    async fn get_commit_diff(&self, _commit_id: &CommitId) -> Result<String> {
        Ok("Stub diff content...".to_string())
    }

    async fn describe_revision(&self, _change_id: &str, _message: &str) -> Result<()> {
        Ok(())
    }

    async fn snapshot(&self) -> Result<String> {
        Ok("Snapshot created (stub)".to_string())
    }
}
