use super::JjAdapter;
use crate::domain::models::CommitId;
use anyhow::{anyhow, Result};

impl JjAdapter {
    pub(crate) async fn set_bookmark_impl(&self, commit_id: &CommitId, name: &str) -> Result<()> {
        self.validate_commit(commit_id).await?;
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let output = tokio::process::Command::new("jj")
            .arg("bookmark")
            .arg("set")
            .arg(name)
            .arg("-r")
            .arg(&commit_id.0)
            .current_dir(ws_root)
            .output()
            .await?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj bookmark set failed: {}", stderr.trim()))
        }
    }

    pub(crate) async fn delete_bookmark_impl(&self, name: &str) -> Result<()> {
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let output = tokio::process::Command::new("jj")
            .arg("bookmark")
            .arg("delete")
            .arg(name)
            .current_dir(ws_root)
            .output()
            .await?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj bookmark delete failed: {}", stderr.trim()))
        }
    }
}
