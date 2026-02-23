use super::JjAdapter;
use crate::domain::models::CommitId;
use anyhow::{anyhow, Context, Result};
use jj_lib::{
    local_working_copy::LocalWorkingCopyFactory, repo::StoreFactories,
    working_copy::WorkingCopyFactory, workspace::Workspace,
};
use std::collections::HashMap;

impl JjAdapter {
    pub(crate) async fn describe_revision_impl(
        &self,
        commit_id: &str,
        message: &str,
    ) -> Result<()> {
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let output = tokio::process::Command::new("jj")
            .arg("describe")
            .arg(commit_id)
            .arg("-m")
            .arg(message)
            .current_dir(ws_root)
            .output()
            .await?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj describe failed: {}", stderr.trim()))
        }
    }

    pub(crate) async fn commit_impl(&self, message: &str) -> Result<()> {
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let output = tokio::process::Command::new("jj")
            .arg("commit")
            .arg("-m")
            .arg(message)
            .current_dir(ws_root)
            .output()
            .await?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj commit failed: {}", stderr.trim()))
        }
    }

    pub(crate) async fn snapshot_impl(&self) -> Result<String> {
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let output = tokio::process::Command::new("jj")
            .arg("status")
            .current_dir(&ws_root)
            .output()
            .await?;
        if output.status.success() {
            Ok("Snapshot created.".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj snapshot failed: {}", stderr.trim()))
        }
    }

    pub(crate) async fn edit_impl(&self, commit_id: &CommitId) -> Result<()> {
        self.validate_commit(commit_id).await?;
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let output = tokio::process::Command::new("jj")
            .arg("edit")
            .arg(&commit_id.0)
            .current_dir(ws_root)
            .output()
            .await?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj edit failed: {}", stderr.trim()))
        }
    }

    pub(crate) async fn squash_impl(&self, commit_ids: &[CommitId]) -> Result<()> {
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let mut cmd = tokio::process::Command::new("jj");
        cmd.arg("squash");
        for id in commit_ids {
            self.validate_commit(id).await?;
            cmd.arg("-r").arg(&id.0);
        }
        let output = cmd.current_dir(ws_root).output().await?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj squash failed: {}", stderr.trim()))
        }
    }

    pub(crate) async fn new_child_impl(&self, commit_id: &CommitId) -> Result<()> {
        self.validate_commit(commit_id).await?;
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let output = tokio::process::Command::new("jj")
            .arg("new")
            .arg(&commit_id.0)
            .current_dir(ws_root)
            .output()
            .await?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj new failed: {}", stderr.trim()))
        }
    }

    pub(crate) async fn abandon_impl(&self, commit_ids: &[CommitId]) -> Result<()> {
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let mut cmd = tokio::process::Command::new("jj");
        cmd.arg("abandon");
        for id in commit_ids {
            self.validate_commit(id).await?;
            cmd.arg("-r").arg(&id.0);
        }
        let output = cmd.current_dir(ws_root).output().await?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj abandon failed: {}", stderr.trim()))
        }
    }

    pub(crate) async fn revert_impl(&self, commit_ids: &[CommitId]) -> Result<()> {
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let mut cmd = tokio::process::Command::new("jj");
        cmd.arg("revert");
        for id in commit_ids {
            self.validate_commit(id).await?;
            cmd.arg("-r").arg(&id.0);
        }
        let output = cmd.current_dir(ws_root).output().await?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj revert failed: {}", stderr.trim()))
        }
    }

    pub(crate) async fn absorb_impl(&self) -> Result<()> {
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let mut cmd = tokio::process::Command::new("jj");
        cmd.arg("absorb");
        let output = cmd.current_dir(ws_root).output().await?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj absorb failed: {}", stderr.trim()))
        }
    }

    pub(crate) async fn duplicate_impl(&self, commit_ids: &[CommitId]) -> Result<()> {
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let mut cmd = tokio::process::Command::new("jj");
        cmd.arg("duplicate");
        for id in commit_ids {
            self.validate_commit(id).await?;
            cmd.arg("-r").arg(&id.0);
        }
        let output = cmd.current_dir(ws_root).output().await?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj duplicate failed: {}", stderr.trim()))
        }
    }

    pub(crate) async fn parallelize_impl(&self, commit_ids: &[CommitId]) -> Result<()> {
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let mut cmd = tokio::process::Command::new("jj");
        cmd.arg("parallelize");
        for id in commit_ids {
            self.validate_commit(id).await?;
            cmd.arg("-r").arg(&id.0);
        }
        let output = cmd.current_dir(ws_root).output().await?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj parallelize failed: {}", stderr.trim()))
        }
    }

    pub(crate) async fn rebase_impl(
        &self,
        commit_ids: &[CommitId],
        destination: &str,
    ) -> Result<()> {
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let mut cmd = tokio::process::Command::new("jj");
        cmd.arg("rebase");
        for id in commit_ids {
            self.validate_commit(id).await?;
            cmd.arg("-r").arg(&id.0);
        }
        cmd.arg("-d").arg(destination);
        let output = cmd.current_dir(ws_root).output().await?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj rebase failed: {}", stderr.trim()))
        }
    }

    pub(crate) async fn undo_impl(&self) -> Result<()> {
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let output = tokio::process::Command::new("jj")
            .arg("undo")
            .current_dir(ws_root)
            .output()
            .await?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj undo failed: {}", stderr.trim()))
        }
    }

    pub(crate) async fn redo_impl(&self) -> Result<()> {
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let output = tokio::process::Command::new("jj")
            .arg("redo")
            .current_dir(ws_root)
            .output()
            .await?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj redo failed: {}", stderr.trim()))
        }
    }

    pub(crate) async fn init_repo_impl(&self) -> Result<()> {
        let output = tokio::process::Command::new("jj")
            .arg("git")
            .arg("init")
            .arg("--colocate")
            .current_dir(&self.workspace_root)
            .output()
            .await?;
        if output.status.success() {
            let mut ws_opt = self.workspace.lock().await;

            let store_factories = StoreFactories::default();
            let mut working_copy_factories: HashMap<String, Box<dyn WorkingCopyFactory>> =
                HashMap::new();
            working_copy_factories
                .insert("local".to_string(), Box::new(LocalWorkingCopyFactory {}));

            let workspace = Workspace::load(
                &self.user_settings,
                &self.workspace_root,
                &store_factories,
                &working_copy_factories,
            )
            .context("Failed to load workspace after init")?;
            *ws_opt = Some(workspace);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj git init failed: {}", stderr.trim()))
        }
    }
}
