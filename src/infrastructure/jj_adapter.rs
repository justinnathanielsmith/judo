use crate::domain::{
    models::{CommitId, FileChange, FileStatus, GraphRow, RepoStatus},
    vcs::VcsFacade,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use jj_lib::{
    backend::{CommitId as JjCommitId, TreeValue},
    repo::Repo,
    object_id::ObjectId,
    settings::UserSettings,
    working_copy::WorkingCopyFactory,
    local_working_copy::LocalWorkingCopyFactory,
    workspace::Workspace,
    repo::StoreFactories,
};

use futures::StreamExt;
use jj_lib::matchers::EverythingMatcher;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;

pub struct JjAdapter {
    workspace: Arc<Mutex<Workspace>>,
    _user_settings: UserSettings,
}

const _MAX_DIFF_SIZE: u64 = 10 * 1024 * 1024; // 10MB

impl JjAdapter {
    pub fn new() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        Self::load_at(cwd)
    }

    fn load_at(cwd: std::path::PathBuf) -> Result<Self> {
        let mut config = jj_lib::config::StackedConfig::with_defaults();

        if let Ok(layer) = jj_lib::config::ConfigLayer::parse(
            jj_lib::config::ConfigSource::Default,
            crate::infrastructure::defaults::DEFAULT_FALLBACK_CONFIG,
        ) {
            config.add_layer(layer);
        }

        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"));
        if let Ok(home_dir) = home {
            let paths = [
                std::path::PathBuf::from(&home_dir).join(".jjconfig"),
                std::path::PathBuf::from(&home_dir).join(".jj/config.toml"),
                std::path::PathBuf::from(&home_dir).join(".config/jj/config.toml"),
            ];
            for path in paths {
                if path.exists() {
                    if let Ok(text) = std::fs::read_to_string(&path) {
                        if let Ok(layer) = jj_lib::config::ConfigLayer::parse(jj_lib::config::ConfigSource::User, &text) {
                            config.add_layer(layer);
                        }
                    }
                }
            }
        }

        let mut current = Some(cwd.as_path());
        while let Some(path) = current {
            let jj_repo_config = path.join(".jj").join("repo").join("config.toml");
            if jj_repo_config.is_file() {
                if let Ok(text) = std::fs::read_to_string(&jj_repo_config) {
                    if let Ok(layer) = jj_lib::config::ConfigLayer::parse(jj_lib::config::ConfigSource::User, &text) {
                        config.add_layer(layer);
                    }
                }
                break;
            }
            current = path.parent();
        }

        let user_settings = UserSettings::from_config(config)?;
        let store_factories = StoreFactories::default();
        let mut working_copy_factories: HashMap<String, Box<dyn WorkingCopyFactory>> =
            HashMap::new();
        working_copy_factories.insert("local".to_string(), Box::new(LocalWorkingCopyFactory {}));

        let workspace = Workspace::load(
            &user_settings,
            &cwd,
            &store_factories,
            &working_copy_factories,
        )?;

        Ok(Self {
            workspace: Arc::new(Mutex::new(workspace)),
            _user_settings: user_settings,
        })
    }
}

#[async_trait]
impl VcsFacade for JjAdapter {
    async fn get_operation_log(
        &self,
        heads: Option<Vec<CommitId>>,
        limit: usize,
        revset: Option<String>,
    ) -> Result<RepoStatus> {
        let (repo, ws_root) = {
            let ws = self.workspace.lock().await;
            let repo = ws.repo_loader().load_at_head()?;
            (repo, ws.workspace_root().to_path_buf())
        };
        let op_id = repo.operation().id().clone().hex();

        let (workspace_id, _) = repo
            .view()
            .wc_commit_ids()
            .iter()
            .next()
            .ok_or_else(|| anyhow!("No working copy found in view"))?;
        let workspace_id = workspace_id.clone();

        let repo_arc = repo.clone();
        let ws_id_clone = workspace_id.clone();

        // Phase 1: Blocking Graph Traversal (Pre-loading objects)
        let commit_infos = tokio::task::spawn_blocking(move || {
            let mut visited = HashSet::<jj_lib::backend::CommitId>::new();
            let mut queue = VecDeque::new();
            let mut results = Vec::new();

            if let Some(revset_str) = revset {
                let output = std::process::Command::new("jj")
                    .arg("log")
                    .arg("-r")
                    .arg(&revset_str)
                    .arg("-T")
                    .arg("commit_id + '\\n'")
                    .arg("--no-graph")
                    .current_dir(ws_root)
                    .output();

                if let Ok(output) = output {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let ids: Vec<String> = stdout.lines()
                            .map(|l| l.trim().to_string())
                            .filter(|l| !l.is_empty())
                            .collect();

                        for id_hex in ids.iter().take(limit) {
                            if let Some(id) = JjCommitId::try_from_hex(id_hex) {
                                let commit = repo_arc.store().get_commit(&id)?;
                                let mut parent_ids_domain = Vec::new();
                                for parent_id in commit.parent_ids() {
                                    parent_ids_domain.push(CommitId(parent_id.hex()));
                                }

                                let first_parent = commit.parents().next().transpose()?;
                                let tree = commit.tree();
                                let parent_tree = first_parent.as_ref().map(|p| p.tree());
                                let is_working_copy = Some(&id) == repo_arc.view().get_wc_commit_id(&ws_id_clone);
                                let is_immutable =
                                    repo_arc.view().heads().contains(&id) || commit.parents().next().is_none();

                                let bookmarks = repo_arc
                                    .view()
                                    .local_bookmarks()
                                    .filter(|(_, target)| target.added_ids().any(|added_id| added_id == &id))
                                    .map(|(name, _)| name.as_str().to_string())
                                    .collect::<Vec<_>>();

                                results.push((commit, tree, parent_tree, parent_ids_domain, is_working_copy, is_immutable, bookmarks));
                            }
                        }
                        return Ok(results);
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        return Err(anyhow!("Jujutsu error: {}", stderr.trim()));
                    }
                }
            }

            // Default traversal logic
            if let Some(heads) = heads {
                for head in heads {
                    if let Some(id) = jj_lib::backend::CommitId::try_from_hex(&head.0) {
                        queue.push_back(id);
                    }
                }
            } else {
                for head_id in repo_arc.view().heads() {
                    queue.push_back(head_id.clone());
                }
            }

            while let Some(id) = queue.pop_front() {
                if visited.contains(&id) || results.len() >= limit {
                    continue;
                }
                visited.insert(id.clone());

                let commit = repo_arc.store().get_commit(&id)?;
                let mut parent_ids_domain = Vec::new();
                for parent_id in commit.parent_ids() {
                    parent_ids_domain.push(CommitId(parent_id.hex()));
                    queue.push_back(parent_id.clone());
                }

                let first_parent = commit.parents().next().transpose()?;
                let tree = commit.tree();
                let parent_tree = first_parent.as_ref().map(|p| p.tree());
                let is_working_copy = Some(&id) == repo_arc.view().get_wc_commit_id(&ws_id_clone);
                let is_immutable =
                    repo_arc.view().heads().contains(&id) || commit.parents().next().is_none();

                let bookmarks = repo_arc
                    .view()
                    .local_bookmarks()
                    .filter(|(_, target)| target.added_ids().any(|added_id| added_id == &id))
                    .map(|(name, _)| name.as_str().to_string())
                    .collect::<Vec<_>>();

                results.push((commit, tree, parent_tree, parent_ids_domain, is_working_copy, is_immutable, bookmarks));
            }
            Ok::<_, anyhow::Error>(results)
        })
        .await??;

        let graph_rows = futures::stream::iter(commit_infos)
            .map(|(commit, tree, parent_tree, parent_ids, is_working_copy, is_immutable, bookmarks)| async move {
                 let description = commit.description().to_string();
                 let change_id = commit.change_id().hex();
                 let author = commit.author().email.clone();
                 let timestamp_sec = commit.author().timestamp.timestamp.0;
                 let datetime = chrono::DateTime::from_timestamp(timestamp_sec, 0).unwrap_or_default();
                 let timestamp = datetime.format("%Y-%m-%d %H:%M").to_string();
                 let commit_id = CommitId(commit.id().hex());

                 let mut changed_files = Vec::new();
                 if let Some(p_tree) = parent_tree {
                     let mut stream = p_tree.diff_stream(&tree, &EverythingMatcher);
                     while let Some(entry) = stream.next().await {
                         let status = if let Ok(values) = entry.values {
                            if !values.after.is_resolved() {
                                FileStatus::Conflicted
                            } else if values.before.is_absent() {
                                FileStatus::Added
                            } else if values.after.is_absent() {
                                FileStatus::Deleted
                            } else {
                                FileStatus::Modified
                            }
                        } else {
                            FileStatus::Modified
                        };

                        changed_files.push(FileChange {
                            path: entry.path.as_internal_file_string().to_string(),
                            status,
                        });
                     }
                 }

                 GraphRow {
                    commit_id,
                    change_id,
                    description,
                    author,
                    timestamp,
                    is_working_copy,
                    is_immutable,
                    parents: parent_ids,
                    bookmarks,
                    changed_files,
                    visual: crate::domain::models::GraphRowVisual::default(),
                }
            })
            .buffered(50) 
            .collect::<Vec<_>>()
            .await;

        let wc_id = match repo.view().get_wc_commit_id(&workspace_id) {
            Some(id) => CommitId(id.hex()),
            None => CommitId("".to_string()),
        };

        Ok(RepoStatus {
            operation_id: op_id,
            workspace_id: workspace_id.as_str().to_string(),
            working_copy_id: wc_id,
            graph: graph_rows,
        })
    }

    async fn get_commit_diff(&self, commit_id: &CommitId) -> Result<String> {
        let repo = {
            let ws = self.workspace.lock().await;
            ws.repo_loader().load_at_head()?
        };

        let id = JjCommitId::try_from_hex(&commit_id.0).ok_or_else(|| anyhow!("Invalid commit ID"))?;
        let commit = repo.store().get_commit(&id)?;

        let mut output = String::new();
        let author = commit.author();
        let timestamp_sec = author.timestamp.timestamp.0;
        let datetime = chrono::DateTime::from_timestamp(timestamp_sec, 0).unwrap_or_default();
        let timestamp = datetime.format("%Y-%m-%d %H:%M").to_string();

        output.push_str(&format!("Commit ID: {}\n", commit.id().hex()));
        output.push_str(&format!("Change ID: {}\n", commit.change_id().hex()));
        output.push_str(&format!("Author   : {} <{}> ({})\n", author.name, author.email, timestamp));
        output.push_str(&format!("    {}\n\n", commit.description().replace('\n', "\n    ")));

        let mut parents = commit.parents();
        let tree = commit.tree();
        let parent_tree = if let Some(parent) = parents.next() {
            parent?.tree()
        } else {
            tree.clone()
        };

        let mut stream = parent_tree.diff_stream(&tree, &EverythingMatcher);
        while let Some(entry) = stream.next().await {
            let repo_path = entry.path;
            let path_str = repo_path.as_internal_file_string();
            let values = entry.values?;

            output.push_str(&format!("File: {}\n", path_str));

            if !values.after.is_resolved() {
                output.push_str("Status: Conflicted\n");
            } else if values.before.is_absent() {
                output.push_str("Status: Added\n");
            } else if values.after.is_absent() {
                output.push_str("Status: Deleted\n");
            } else {
                output.push_str("Status: Modified\n");
            }

            let mut before_content = Vec::new();
            for value in values.before.iter() {
                if let Some(TreeValue::File { id, .. }) = value.as_ref() {
                    let mut reader = repo.store().read_file(&repo_path, id).await?;
                    reader.read_to_end(&mut before_content).await?;
                }
            }

            let mut after_content = Vec::new();
            for value in values.after.iter() {
                if let Some(TreeValue::File { id, .. }) = value.as_ref() {
                    let mut reader = repo.store().read_file(&repo_path, id).await?;
                    reader.read_to_end(&mut after_content).await?;
                }
            }

            let before_str = String::from_utf8_lossy(&before_content);
            let after_str = String::from_utf8_lossy(&after_content);

            let diff = similar::TextDiff::from_lines(&before_str, &after_str);
            for change in diff.iter_all_changes() {
                let sign = match change.tag() {
                    similar::ChangeTag::Delete => "-",
                    similar::ChangeTag::Insert => "+",
                    similar::ChangeTag::Equal => " ",
                };
                output.push_str(&format!("{}{}", sign, change));
            }
            output.push_str("\n");
        }

        Ok(output)
    }

    async fn describe_revision(&self, change_id: &str, message: &str) -> Result<()> {
        let ws_root = self.workspace.lock().await.workspace_root().to_path_buf();
        let output = tokio::process::Command::new("jj")
            .arg("describe").arg(change_id).arg("-m").arg(message)
            .current_dir(ws_root).output().await?;
        if output.status.success() { Ok(()) } else { Err(anyhow!("jj describe failed")) }
    }

    async fn snapshot(&self) -> Result<String> {
        let ws_root = self.workspace.lock().await.workspace_root().to_path_buf();
        let output = tokio::process::Command::new("jj").arg("snapshot")
            .current_dir(&ws_root).output().await?;
        if output.status.success() { Ok("Snapshot created.".to_string()) } else { Err(anyhow!("jj snapshot failed")) }
    }

    async fn edit(&self, commit_id: &CommitId) -> Result<()> {
        let ws_root = self.workspace.lock().await.workspace_root().to_path_buf();
        let output = tokio::process::Command::new("jj").arg("edit").arg(&commit_id.0)
            .current_dir(ws_root).output().await?;
        if output.status.success() { Ok(()) } else { Err(anyhow!("jj edit failed")) }
    }

    async fn squash(&self, commit_id: &CommitId) -> Result<()> {
        let ws_root = self.workspace.lock().await.workspace_root().to_path_buf();
        let output = tokio::process::Command::new("jj").arg("squash").arg("-r").arg(&commit_id.0)
            .current_dir(ws_root).output().await?;
        if output.status.success() { Ok(()) } else { Err(anyhow!("jj squash failed")) }
    }

    async fn new_child(&self, commit_id: &CommitId) -> Result<()> {
        let ws_root = self.workspace.lock().await.workspace_root().to_path_buf();
        let output = tokio::process::Command::new("jj").arg("new").arg(&commit_id.0)
            .current_dir(ws_root).output().await?;
        if output.status.success() { Ok(()) } else { Err(anyhow!("jj new failed")) }
    }

    async fn abandon(&self, commit_id: &CommitId) -> Result<()> {
        let ws_root = self.workspace.lock().await.workspace_root().to_path_buf();
        let output = tokio::process::Command::new("jj").arg("abandon").arg(&commit_id.0)
            .current_dir(ws_root).output().await?;
        if output.status.success() { Ok(()) } else { Err(anyhow!("jj abandon failed")) }
    }

    async fn set_bookmark(&self, commit_id: &CommitId, name: &str) -> Result<()> {
        let ws_root = self.workspace.lock().await.workspace_root().to_path_buf();
        let output = tokio::process::Command::new("jj")
            .arg("bookmark").arg("set").arg(name).arg("-r").arg(&commit_id.0)
            .current_dir(ws_root).output().await?;
        if output.status.success() { Ok(()) } else { Err(anyhow!("jj bookmark set failed")) }
    }

    async fn delete_bookmark(&self, name: &str) -> Result<()> {
        let ws_root = self.workspace.lock().await.workspace_root().to_path_buf();
        let output = tokio::process::Command::new("jj").arg("bookmark").arg("delete").arg(name)
            .current_dir(ws_root).output().await?;
        if output.status.success() { Ok(()) } else { Err(anyhow!("jj bookmark delete failed")) }
    }

    async fn undo(&self) -> Result<()> {
        let ws_root = self.workspace.lock().await.workspace_root().to_path_buf();
        let output = tokio::process::Command::new("jj").arg("undo")
            .current_dir(ws_root).output().await?;
        if output.status.success() { Ok(()) } else { Err(anyhow!("jj undo failed")) }
    }

    async fn redo(&self) -> Result<()> {
        let ws_root = self.workspace.lock().await.workspace_root().to_path_buf();
        let output = tokio::process::Command::new("jj").arg("redo")
            .current_dir(ws_root).output().await?;
        if output.status.success() { Ok(()) } else { Err(anyhow!("jj redo failed")) }
    }

    async fn fetch(&self) -> Result<()> {
        let ws_root = self.workspace.lock().await.workspace_root().to_path_buf();
        let output = tokio::process::Command::new("jj").arg("git").arg("fetch")
            .current_dir(ws_root).output().await?;
        if output.status.success() { Ok(()) } else { Err(anyhow!("jj git fetch failed")) }
    }

    async fn push(&self, bookmark: Option<String>) -> Result<()> {
        let ws_root = self.workspace.lock().await.workspace_root().to_path_buf();
        let mut cmd = tokio::process::Command::new("jj");
        cmd.arg("git").arg("push").current_dir(ws_root);
        if let Some(b) = bookmark { cmd.arg("-b").arg(b); }
        let output = cmd.output().await?;
        if output.status.success() { Ok(()) } else { Err(anyhow!("jj git push failed")) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jj_adapter_new() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let path = temp_dir.path();
        let config = jj_lib::config::StackedConfig::with_defaults();
        let user_settings = UserSettings::from_config(config)?;
        Workspace::init_simple(&user_settings, path)?;
        let adapter = JjAdapter::load_at(path.to_path_buf());
        assert!(adapter.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_new_child() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let path = temp_dir.path();
        let config = jj_lib::config::StackedConfig::with_defaults();
        let user_settings = UserSettings::from_config(config)?;
        Workspace::init_simple(&user_settings, path)?;
        let adapter = JjAdapter::load_at(path.to_path_buf())?;
        let status = adapter.get_operation_log(None, 100, None).await?;
        let parent_commit = status.graph.first().ok_or_else(|| anyhow!("Graph is empty"))?;
        let parent_id = parent_commit.commit_id.clone();
        adapter.new_child(&parent_id).await?;
        let new_status = adapter.get_operation_log(None, 100, None).await?;
        let child_commit = new_status.graph.iter().find(|row| row.parents.contains(&parent_id));
        assert!(child_commit.is_some());
        Ok(())
    }
}
