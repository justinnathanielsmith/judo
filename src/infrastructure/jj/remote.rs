use super::JjAdapter;
use anyhow::{anyhow, Result};

impl JjAdapter {
    pub(crate) async fn fetch_impl(&self) -> Result<()> {
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let output = tokio::process::Command::new("jj")
            .arg("git")
            .arg("fetch")
            .current_dir(ws_root)
            .output()
            .await?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj fetch failed: {}", stderr.trim()))
        }
    }

    pub(crate) async fn push_impl(&self, bookmark: Option<String>) -> Result<()> {
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let mut cmd = tokio::process::Command::new("jj");
        cmd.arg("git").arg("push");
        if let Some(bm) = bookmark {
            cmd.arg("-b").arg(bm);
        }
        let output = cmd.current_dir(ws_root).output().await?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj push failed: {}", stderr.trim()))
        }
    }
}
