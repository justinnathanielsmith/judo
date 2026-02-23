use crate::domain::{
    models::{CommitId, FileChange, FileStatus, GraphRow, RepoStatus},
    vcs::VcsFacade,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use jj_lib::{
    backend::{CommitId as JjCommitId, TreeValue},
    local_working_copy::LocalWorkingCopyFactory,
    object_id::ObjectId,
    repo::Repo,
    repo::StoreFactories,
    settings::UserSettings,
    working_copy::WorkingCopyFactory,
    workspace::Workspace,
};

use futures::StreamExt;
use jj_lib::matchers::EverythingMatcher;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::sync::{Mutex, Semaphore};

pub struct JjAdapter {
    workspace: Arc<Mutex<Option<Workspace>>>,
    workspace_root: std::path::PathBuf,
    user_settings: UserSettings,
    diff_semaphore: Arc<Semaphore>,
}

const MAX_DIFF_SIZE: u64 = 1024 * 1024; // 1MB
const MAX_CONCURRENT_DIFFS: usize = 4;

impl JjAdapter {
    pub fn new() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        Self::for_path(cwd)
    }

    pub fn for_path(path: std::path::PathBuf) -> Result<Self> {
        let mut config = jj_lib::config::StackedConfig::with_defaults();

        let layer = jj_lib::config::ConfigLayer::parse(
            jj_lib::config::ConfigSource::Default,
            crate::infrastructure::defaults::DEFAULT_FALLBACK_CONFIG,
        )
        .context("Failed to parse internal fallback config (this is a Judo bug)")?;
        config.add_layer(layer);

        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"));
        if let Ok(home_dir) = home {
            let paths = [
                std::path::PathBuf::from(&home_dir).join(".jjconfig"),
                std::path::PathBuf::from(&home_dir).join(".jj/config.toml"),
                std::path::PathBuf::from(&home_dir).join(".config/jj/config.toml"),
            ];
            for config_path in paths {
                if config_path.exists() {
                    let text = std::fs::read_to_string(&config_path).with_context(|| {
                        format!("Failed to read user config at {config_path:?}")
                    })?;
                    let layer = jj_lib::config::ConfigLayer::parse(
                        jj_lib::config::ConfigSource::User,
                        &text,
                    )
                    .with_context(|| format!("Failed to parse user config at {config_path:?}"))?;
                    config.add_layer(layer);
                }
            }
        }

        let mut current = Some(path.as_path());
        let mut found_ws_root = None;
        while let Some(current_path) = current {
            let jj_repo_config = current_path.join(".jj").join("repo").join("config.toml");
            if jj_repo_config.is_file() {
                let text = std::fs::read_to_string(&jj_repo_config)
                    .with_context(|| format!("Failed to read repo config at {jj_repo_config:?}"))?;
                let layer =
                    jj_lib::config::ConfigLayer::parse(jj_lib::config::ConfigSource::User, &text)
                        .with_context(|| {
                        format!("Failed to parse user config at {jj_repo_config:?}")
                    })?;
                config.add_layer(layer);
                found_ws_root = Some(current_path.to_path_buf());
                break;
            }
            current = current_path.parent();
        }

        let user_settings = UserSettings::from_config(config)?;
        let store_factories = StoreFactories::default();
        let mut working_copy_factories: HashMap<String, Box<dyn WorkingCopyFactory>> =
            HashMap::new();
        working_copy_factories.insert("local".to_string(), Box::new(LocalWorkingCopyFactory {}));

        let ws_root = found_ws_root.unwrap_or_else(|| path.clone());

        let workspace = Workspace::load(
            &user_settings,
            &ws_root,
            &store_factories,
            &working_copy_factories,
        )
        .ok();

        let workspace_root = if let Some(ws) = &workspace {
            ws.workspace_root().to_path_buf()
        } else {
            ws_root
        };

        Ok(Self {
            workspace: Arc::new(Mutex::new(workspace)),
            workspace_root,
            user_settings,
            diff_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_DIFFS)),
        })
    }

    pub async fn check_version() -> Result<()> {
        let output = tokio::process::Command::new("jj")
            .arg("--version")
            .output()
            .await
            .context("Failed to execute 'jj --version'. Is 'jj' installed and in your PATH?")?;

        if !output.status.success() {
            return Err(anyhow!("'jj --version' failed"));
        }

        let version_str = String::from_utf8_lossy(&output.stdout);
        // Judo is built against jj-lib 0.38.0.
        // CLI version usually matches lib version (e.g. "jj 0.38.0")
        if !version_str.contains("0.38") {
            return Err(anyhow!(
                "Judo expects jj version 0.38.x, but found: {}. \
                 Using mismatched versions may lead to repository corruption or crashes.",
                version_str.trim()
            ));
        }
        Ok(())
    }

    async fn validate_commit(&self, commit_id: &CommitId) -> Result<JjCommitId> {
        let repo = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.repo_loader().load_at_head()?
        };
        let id = JjCommitId::try_from_hex(&commit_id.0)
            .ok_or_else(|| anyhow!("Invalid commit ID format: {}", commit_id.0))?;

        if !repo.view().heads().contains(&id) {
            // If it's not a head, it might be an ancestor.
            // We check if the store has it and if it's visible in the current index.
            if !repo.index().has_id(&id).map_err(|e| anyhow!(e))? {
                return Err(anyhow!(
                    "Commit {} is no longer valid or has been rewritten/abandoned.",
                    commit_id.0
                ));
            }
        }
        Ok(id)
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
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
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
        let ws_root_for_closure = ws_root.clone();

        // Phase 1: Blocking Graph Traversal (Pre-loading objects)
        let commit_infos = tokio::task::spawn_blocking(move || {
            let mut visited = HashSet::<jj_lib::backend::CommitId>::new();
            let mut queue = VecDeque::new();
            let mut results = Vec::new();

            if let Some(revset_str) = revset {
                let output = std::process::Command::new("jj")
                    .arg("--color")
                    .arg("never")
                    .arg("--no-pager")
                    .arg("--repository")
                    .arg(".")
                    .arg("log")
                    .arg("-r")
                    .arg(&revset_str)
                    .arg("-T")
                    .arg("commit_id ++ \"\\n\"")
                    .arg("--no-graph")
                    .current_dir(&ws_root_for_closure)
                    .output();

                match output {
                    Ok(output) => {
                        if output.status.success() {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let ids: Vec<String> = stdout
                                .lines()
                                .map(|l| l.trim().to_string())
                                .filter(|l| !l.is_empty())
                                .collect();

                            for id_hex in ids.iter().take(limit) {
                                if let Some(id) = JjCommitId::try_from_hex(id_hex) {
                                    let commit = match repo_arc.store().get_commit(&id) {
                                        Ok(c) => c,
                                        Err(_) => continue, // skip corrupt/unreadable commits
                                    };
                                    let mut parent_ids_domain = Vec::new();
                                    for parent_id in commit.parent_ids() {
                                        parent_ids_domain.push(CommitId(parent_id.hex()));
                                    }

                                    let first_parent: Option<jj_lib::commit::Commit> =
                                        commit.parents().next().transpose().unwrap_or_default();
                                    let tree = commit.tree();
                                    let parent_tree = first_parent.as_ref().map(jj_lib::commit::Commit::tree);
                                    let is_working_copy =
                                        Some(&id) == repo_arc.view().get_wc_commit_id(&ws_id_clone);
                                    let is_immutable = repo_arc.view().heads().contains(&id)
                                        || commit.parents().next().is_none();

                                    let bookmarks = repo_arc
                                        .view()
                                        .local_bookmarks()
                                        .filter(|(_, target)| {
                                            target.added_ids().any(|added_id| added_id == &id)
                                        })
                                        .map(|(name, _)| name.as_str().to_string())
                                        .collect::<Vec<_>>();

                                    results.push((
                                        commit,
                                        tree,
                                        parent_tree,
                                        parent_ids_domain,
                                        is_working_copy,
                                        is_immutable,
                                        bookmarks,
                                    ));
                                }
                            }
                            return Ok(results);
                        }
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        return Err(anyhow!("Jujutsu error: {}", stderr.trim()));
                    }
                    Err(e) => {
                        return Err(anyhow!("Failed to execute 'jj' command: {e}. Is 'jj' installed and in your PATH?"));
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
                if results.len() >= limit {
                    break;
                }
                if visited.contains(&id) {
                    continue;
                }
                visited.insert(id.clone());

                let commit = match repo_arc.store().get_commit(&id) {
                    Ok(c) => c,
                    Err(_) => continue, // skip corrupt/unreadable commits
                };
                let mut parent_ids_domain = Vec::new();
                for parent_id in commit.parent_ids() {
                    parent_ids_domain.push(CommitId(parent_id.hex()));
                    queue.push_back(parent_id.clone());
                }

                let first_parent: Option<jj_lib::commit::Commit> =
                    commit.parents().next().transpose().unwrap_or_default();
                let tree = commit.tree();
                let parent_tree = first_parent.as_ref().map(jj_lib::commit::Commit::tree);
                let is_working_copy = Some(&id) == repo_arc.view().get_wc_commit_id(&ws_id_clone);
                let is_immutable =
                    repo_arc.view().heads().contains(&id) || commit.parents().next().is_none();

                let bookmarks = repo_arc
                    .view()
                    .local_bookmarks()
                    .filter(|(_, target)| target.added_ids().any(|added_id| added_id == &id))
                    .map(|(name, _)| name.as_str().to_string())
                    .collect::<Vec<_>>();

                results.push((
                    commit,
                    tree,
                    parent_tree,
                    parent_ids_domain,
                    is_working_copy,
                    is_immutable,
                    bookmarks,
                ));
            }
            Ok::<_, anyhow::Error>(results)
        })
        .await??;

        let graph_rows = futures::stream::iter(commit_infos)
            .map(
                |(
                    commit,
                    tree,
                    parent_tree,
                    parent_ids,
                    is_working_copy,
                    is_immutable,
                    bookmarks,
                )| async move {
                    let description = commit.description().to_string();
                    let change_id = commit.change_id().hex();
                    let author = commit.author().email.clone();
                    let timestamp_secs = commit.author().timestamp.timestamp.0 / 1000;
                    let datetime = chrono::DateTime::from_timestamp(timestamp_secs, 0)
                        .unwrap_or_default()
                        .with_timezone(&chrono::Local);
                    let timestamp = datetime.format("%Y-%m-%d %H:%M").to_string();
                    let commit_id_str = commit.id().hex();
                    let commit_id_short = commit_id_str[..8.min(commit_id_str.len())].to_string();
                    let commit_id = CommitId(commit_id_str);

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

                            let path = entry.path.as_internal_file_string().to_string();
                            // SECURITY: Validate path to prevent path traversal
                            if path.contains("..") {
                                continue;
                            }

                            changed_files.push(FileChange { path, status });
                        }
                    }

                    let change_id_short = change_id[..8.min(change_id.len())].to_string();

                    GraphRow {
                        commit_id,
                        commit_id_short,
                        change_id,
                        change_id_short,
                        description,
                        author,
                        timestamp,
                        timestamp_secs,
                        is_working_copy,
                        is_immutable,
                        parents: parent_ids,
                        bookmarks,
                        changed_files,
                        visual: crate::domain::models::GraphRowVisual::default(),
                    }
                },
            )
            .buffered(50)
            .collect::<Vec<_>>()
            .await;

        let wc_id = match repo.view().get_wc_commit_id(&workspace_id) {
            Some(id) => CommitId(id.hex()),
            None => CommitId(String::new()),
        };

        let repo_name = ws_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(RepoStatus {
            repo_name,
            operation_id: op_id,
            workspace_id: workspace_id.as_str().to_string(),
            working_copy_id: wc_id,
            graph: graph_rows,
        })
    }

    async fn get_commit_diff(&self, commit_id: &CommitId) -> Result<String> {
        let repo = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.repo_loader().load_at_head()?
        };

        let id =
            JjCommitId::try_from_hex(&commit_id.0).ok_or_else(|| anyhow!("Invalid commit ID"))?;
        let commit = repo.store().get_commit(&id)?;

        let mut output = String::new();
        let author = commit.author();
        let timestamp_sec = author.timestamp.timestamp.0 / 1000;
        let datetime = chrono::DateTime::from_timestamp(timestamp_sec, 0)
            .unwrap_or_default()
            .with_timezone(&chrono::Local);
        let timestamp = datetime.format("%Y-%m-%d %H:%M").to_string();

        output.push_str(&format!("Commit ID: {}\n", commit.id().hex()));
        output.push_str(&format!("Change ID: {}\n", commit.change_id().hex()));

        let bookmarks = repo
            .view()
            .local_bookmarks()
            .filter(|(_, target)| target.added_ids().any(|added_id| added_id == &id))
            .map(|(name, _)| name.as_str().to_string())
            .collect::<Vec<_>>();
        if !bookmarks.is_empty() {
            output.push_str(&format!("Bookmarks: {}\n", bookmarks.join(", ")));
        }

        output.push_str(&format!(
            "Author   : {} <{}> ({})\n",
            author.name, author.email, timestamp
        ));
        output.push_str(&format!(
            "    {}\n\n",
            commit.description().replace('\n', "\n    ")
        ));

        let mut parents = commit.parents();
        let tree = commit.tree();
        let parent_tree = if let Some(parent) = parents.next() {
            parent?.tree()
        } else {
            tree.clone()
        };

        let mut stream = parent_tree.diff_stream(&tree, &EverythingMatcher);
        while let Some(entry) = stream.next().await {
            let _permit = self
                .diff_semaphore
                .acquire()
                .await
                .map_err(|e| anyhow!(e))?;
            let repo_path = entry.path;
            let path_str = repo_path.as_internal_file_string();
            // SECURITY: Validate path to prevent path traversal
            if path_str.contains("..") {
                continue;
            }
            let values = entry.values?;

            output.push_str(&format!("File: {path_str}\n"));

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
            let mut before_is_binary = false;
            for value in &values.before {
                if let Some(TreeValue::File { id, .. }) = value.as_ref() {
                    let mut reader = repo
                        .store()
                        .read_file(&repo_path, id)
                        .await?
                        .take(MAX_DIFF_SIZE);
                    let mut chunk = vec![0u8; 1024];
                    let n = reader.read(&mut chunk).await?;
                    chunk.truncate(n);
                    if is_binary(&chunk) {
                        before_is_binary = true;
                        break;
                    }
                    before_content.extend_from_slice(&chunk);
                    reader.read_to_end(&mut before_content).await?;
                }
            }

            let mut after_content = Vec::new();
            let mut after_is_binary = false;
            for value in &values.after {
                if let Some(TreeValue::File { id, .. }) = value.as_ref() {
                    let mut reader = repo
                        .store()
                        .read_file(&repo_path, id)
                        .await?
                        .take(MAX_DIFF_SIZE);
                    let mut chunk = vec![0u8; 1024];
                    let n = reader.read(&mut chunk).await?;
                    chunk.truncate(n);
                    if is_binary(&chunk) {
                        after_is_binary = true;
                        break;
                    }
                    after_content.extend_from_slice(&chunk);
                    reader.read_to_end(&mut after_content).await?;
                }
            }

            if before_is_binary || after_is_binary {
                output.push_str("    (binary file)\n\n");
                continue;
            }

            let before_str = String::from_utf8_lossy(&before_content);
            let after_str = String::from_utf8_lossy(&after_content);

            let diff = similar::TextDiff::from_lines(&before_str, &after_str);
            output.push_str(&diff.unified_diff().context_radius(3).to_string());
            output.push('\n');
        }

        Ok(output)
    }

    async fn describe_revision(&self, change_id: &str, message: &str) -> Result<()> {
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };
        let output = tokio::process::Command::new("jj")
            .arg("describe")
            .arg(change_id)
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

    async fn commit(&self, message: &str) -> Result<()> {
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };
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

    async fn snapshot(&self) -> Result<String> {
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };
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

    async fn edit(&self, commit_id: &CommitId) -> Result<()> {
        self.validate_commit(commit_id).await?;
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };
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

    async fn squash(&self, commit_ids: &[CommitId]) -> Result<()> {
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };
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

    async fn new_child(&self, commit_id: &CommitId) -> Result<()> {
        self.validate_commit(commit_id).await?;
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };
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

    async fn abandon(&self, commit_ids: &[CommitId]) -> Result<()> {
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };
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

    async fn absorb(&self) -> Result<()> {
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };
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

    async fn duplicate(&self, commit_ids: &[CommitId]) -> Result<()> {
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };
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

    async fn rebase(&self, commit_ids: &[CommitId], destination: &str) -> Result<()> {
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };
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

    async fn set_bookmark(&self, commit_id: &CommitId, name: &str) -> Result<()> {
        self.validate_commit(commit_id).await?;
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };
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

    async fn delete_bookmark(&self, name: &str) -> Result<()> {
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };
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

    async fn evolog(&self, commit_id: &CommitId) -> Result<String> {
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };

        let output = tokio::process::Command::new("jj")
            .arg("evolog")
            .arg("--color")
            .arg("never")
            .arg("--no-pager")
            .arg("-r")
            .arg(&commit_id.0)
            .current_dir(ws_root)
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj evolog failed: {}", stderr.trim()))
        }
    }

    async fn operation_log(&self) -> Result<String> {
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };

        let output = tokio::process::Command::new("jj")
            .arg("op")
            .arg("log")
            .arg("--color")
            .arg("never")
            .arg("--no-pager")
            .current_dir(ws_root)
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj op log failed: {}", stderr.trim()))
        }
    }

    async fn undo(&self) -> Result<()> {
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };
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

    async fn redo(&self) -> Result<()> {
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };
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

    async fn fetch(&self) -> Result<()> {
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };
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
            Err(anyhow!("jj git fetch failed: {}", stderr.trim()))
        }
    }

    async fn push(&self, bookmark: Option<String>) -> Result<()> {
        let ws_root = {
            let ws_opt = self.workspace.lock().await;
            let ws = ws_opt
                .as_ref()
                .ok_or_else(|| anyhow!("No repository found"))?;
            ws.workspace_root().to_path_buf()
        };
        let mut cmd = tokio::process::Command::new("jj");
        cmd.arg("git").arg("push").current_dir(ws_root);
        if let Some(b) = bookmark {
            cmd.arg("-b").arg(b);
        }
        let output = cmd.output().await?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj git push failed: {}", stderr.trim()))
        }
    }

    async fn init_repo(&self) -> Result<()> {
        let output = tokio::process::Command::new("jj")
            .arg("git")
            .arg("init")
            .arg("--colocate")
            .current_dir(&self.workspace_root)
            .output()
            .await?;
        if output.status.success() {
            // After successful init, try to load the workspace
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

    async fn is_valid(&self) -> bool {
        self.workspace.lock().await.is_some()
    }

    fn workspace_root(&self) -> std::path::PathBuf {
        self.workspace_root.clone()
    }
}

fn is_binary(chunk: &[u8]) -> bool {
    if chunk.is_empty() {
        return false;
    }
    if chunk.contains(&0) {
        return true;
    }
    let non_printable = chunk
        .iter()
        .filter(|&&b| (b < 32 && !b.is_ascii_whitespace()) || b == 127)
        .count();

    // If more than 10% are control characters (excluding whitespace), it's likely binary.
    non_printable * 100 / chunk.len() > 10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jj_adapter_new() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let path = temp_dir.path().to_path_buf();
        let config = jj_lib::config::StackedConfig::with_defaults();
        let user_settings = UserSettings::from_config(config)?;
        Workspace::init_simple(&user_settings, &path)?;
        let adapter = JjAdapter::for_path(path)?;
        assert!(adapter.is_valid().await);
        Ok(())
    }

    #[test]
    fn test_is_binary() {
        assert!(!is_binary(b"this is some text"));
        assert!(!is_binary(
            b"this is some text with \n newlines and \t tabs"
        ));
        assert!(is_binary(&[0, 1, 2, 3])); // Null byte
        assert!(is_binary(&[1, 2, 3, 4, 5, 6, 7, 8, 11, 12, 14, 15])); // Many control chars

        // UTF-8 should NOT be binary
        assert!(!is_binary("🦀 rust is great".as_bytes()));
    }
}
