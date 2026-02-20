use crate::domain::{
    models::{CommitId, FileChange, FileStatus, GraphRow, RepoStatus},
    vcs::VcsFacade,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use jj_lib::{
    backend::CommitId as JjCommitId,
    config::{ConfigLayer, ConfigSource, StackedConfig},
    local_working_copy::LocalWorkingCopyFactory,
    object_id::ObjectId,
    repo::{Repo, StoreFactories},
    settings::UserSettings,
    working_copy::WorkingCopyFactory,
    workspace::Workspace,
};

use jj_lib::gitignore::GitIgnoreFile;

use futures::StreamExt;
use jj_lib::matchers::{EverythingMatcher, NothingMatcher};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;

pub struct JjAdapter {
    workspace: Arc<Mutex<Workspace>>,
}

const MAX_DIFF_SIZE: u64 = 10 * 1024 * 1024; // 10MB
const MAX_NEW_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB

impl JjAdapter {
    pub fn new() -> Result<Self> {
        let cwd = std::env::current_dir().context("Failed to get current working directory")?;
        Self::load_at(cwd)
    }

    fn load_at(cwd: std::path::PathBuf) -> Result<Self> {
        let mut config = StackedConfig::with_defaults();

        // Layer 1: Judo Fallbacks (Lowest priority above library defaults)
        // These provide sensible defaults for TUI performance and prevent crashes
        // if the user hasn't configured basic identity yet.
        let fallback_config_str = r#"
            [user]
            name = "Judo User"
            email = "judo@example.com"
            [operation]
            hostname = "judo-host"
            username = "judo-user"
            [fsmonitor]
            backend = "none"
        "#;

        if let Ok(layer) = ConfigLayer::parse(ConfigSource::Default, fallback_config_str) {
            config.add_layer(layer);
        }

        // Layer 2: User Config (Higher priority)
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
                        if let Ok(layer) = ConfigLayer::parse(ConfigSource::User, &text) {
                            config.add_layer(layer);
                        }
                    }
                }
            }
        }

        // Layer 3: Repo Config
        // Walk up from CWD to find the .jj directory and load repo-level config
        let mut current = Some(cwd.as_path());
        while let Some(path) = current {
            let jj_repo_config = path.join(".jj").join("repo").join("config.toml");
            if jj_repo_config.is_file() {
                if let Ok(text) = std::fs::read_to_string(&jj_repo_config) {
                    // Using ConfigSource::User for repo config as a safe fallback
                    // if ConfigSource::Repo is not available in this version.
                    if let Ok(layer) = ConfigLayer::parse(ConfigSource::User, &text) {
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
        )
        .context("Failed to load workspace")?;

        Ok(Self {
            workspace: Arc::new(Mutex::new(workspace)),
        })
    }
}

#[async_trait]
impl VcsFacade for JjAdapter {
    async fn get_operation_log(&self) -> Result<RepoStatus> {
        // Workspace is !Sync, so we lock it to access loader
        let repo = {
            let ws = self.workspace.lock().await;
            ws.repo_loader().load_at_head()?
        };

        // operation() returns &Operation. id() returns &OperationId.
        let op_id = repo.operation().id().clone().hex();

        // Find workspace_id from repo view (first available)
        // wc_commit_ids() returns &BTreeMap<WorkspaceId, CommitId>
        let (workspace_id, _) = repo
            .view()
            .wc_commit_ids()
            .iter()
            .next()
            .ok_or_else(|| anyhow!("No working copy found in view"))?;
        let workspace_id = workspace_id.clone();

        // Manual Simple Walk (BFS)
        let mut graph_rows = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start from heads
        for head_id in repo.view().heads() {
            queue.push_back(head_id.clone());
        }

        while let Some(id) = queue.pop_front() {
            if visited.contains(&id) {
                continue;
            }
            visited.insert(id.clone());

            if graph_rows.len() >= 100 {
                break;
            }

            let commit = repo.store().get_commit(&id)?;

            let mut parents = Vec::new();
            for parent in commit.parents().flatten() {
                parents.push(CommitId(parent.id().clone().hex()));
                queue.push_back(parent.id().clone());
            }

            let description = commit.description().to_string();
            let change_id = commit.change_id().hex();
            let author = commit.author().email.clone();
            let timestamp_sec = commit.author().timestamp.timestamp.0;
            let datetime = chrono::DateTime::from_timestamp(timestamp_sec, 0)
                .unwrap_or_default();
            let timestamp = datetime.format("%Y-%m-%d %H:%M").to_string();

            let is_working_copy = Some(&id) == repo.view().get_wc_commit_id(&workspace_id);
            let is_immutable = repo.view().heads().contains(&id) || commit.parents().next().is_none();

            let mut changed_files = Vec::new();
            if let Some(parent) = commit.parents().next() {
                if let Ok(parent_commit) = parent {
                    let parent_tree = parent_commit.tree();
                    let tree = commit.tree();
                    let mut stream = parent_tree.diff_stream(&tree, &EverythingMatcher);
                    while let Some(entry) = stream.next().await {
                        let status = if let Ok(values) = entry.values {
                            if values.before.is_absent() {
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
            }

            let bookmarks = repo
                .view()
                .local_bookmarks()
                .filter(|(_, target)| target.added_ids().any(|added_id| added_id == &id))
                .map(|(name, _)| name.as_str().to_string())
                .collect::<Vec<_>>();

            graph_rows.push(GraphRow {
                commit_id: CommitId(id.hex()),
                change_id,
                description,
                author,
                timestamp,
                is_working_copy,
                is_immutable,
                parents,
                bookmarks,
                changed_files,
            });
        }

        // Sort by timestamp desc or similar? BFS might be mixed.
        // For MVP, raw list is fine.

        // Get working copy ID
        let wc_id = match repo.view().get_wc_commit_id(&workspace_id) {
            Some(id) => CommitId(id.hex()),
            None => CommitId("".to_string()),
        };

        Ok(RepoStatus {
            operation_id: op_id,
            working_copy_id: wc_id,
            graph: graph_rows,
        })
    }

    async fn get_commit_diff(&self, commit_id: &CommitId) -> Result<String> {
        let repo = {
            let ws = self.workspace.lock().await;
            ws.repo_loader().load_at_head()?
        };

        let id =
            JjCommitId::try_from_hex(&commit_id.0).ok_or_else(|| anyhow!("Invalid commit ID"))?;
        let commit = repo.store().get_commit(&id)?;

        let mut output = String::new();

        // Commit Header
        let author = commit.author();
        let committer = commit.committer();
        let timestamp_sec = author.timestamp.timestamp.0;
        let datetime = chrono::DateTime::from_timestamp(timestamp_sec, 0).unwrap_or_default();
        let timestamp = datetime.format("%Y-%m-%d %H:%M").to_string();

        output.push_str(&format!("Commit ID: {}\n", commit.id().hex()));
        output.push_str(&format!("Change ID: {}\n", commit.change_id().hex()));
        output.push_str(&format!(
            "Author   : {} <{}> ({})\n",
            author.name, author.email, timestamp
        ));
        output.push_str(&format!(
            "Committer: {} <{}> ({})\n",
            committer.name, committer.email, timestamp
        ));
        output.push_str(&format!(
            "    {}\n\n",
            commit.description().replace('\n', "\n    ")
        ));

        let mut parents = commit.parents();

        let parent_tree = if let Some(parent) = parents.next() {
            parent?.tree()
        } else {
            return Ok("Root commit - diff not supported yet".to_string());
        };

        let tree = commit.tree();

        let mut stream = parent_tree.diff_stream(&tree, &EverythingMatcher);
        while let Some(entry) = stream.next().await {
            let path_str = entry.path.as_internal_file_string();

            let diff = entry.values?;

            let mut old_content = Vec::new();
            if let Ok(Some(jj_lib::backend::TreeValue::File { id, .. })) =
                diff.before.into_resolved()
            {
                let mut reader = repo.store().read_file(&entry.path, &id).await?.take(MAX_DIFF_SIZE + 1);
                reader.read_to_end(&mut old_content).await?;
                if old_content.len() as u64 > MAX_DIFF_SIZE {
                    output.push_str(&format!("File {} is too large to diff\n\n", path_str));
                    continue;
                }
            }

            let mut new_content = Vec::new();
            if let Ok(Some(jj_lib::backend::TreeValue::File { id, .. })) =
                diff.after.into_resolved()
            {
                let mut reader = repo.store().read_file(&entry.path, &id).await?.take(MAX_DIFF_SIZE + 1);
                reader.read_to_end(&mut new_content).await?;
                if new_content.len() as u64 > MAX_DIFF_SIZE {
                    output.push_str(&format!("File {} is too large to diff\n\n", path_str));
                    continue;
                }
            }

            // Simple binary check: check for null bytes in the first 1KB
            let is_binary = old_content
                .iter()
                .chain(new_content.iter())
                .take(1024)
                .any(|&b| b == 0);

            if is_binary {
                output.push_str(&format!("Binary file {}\n\n", path_str));
            } else {
                let old_text = String::from_utf8_lossy(&old_content);
                let new_text = String::from_utf8_lossy(&new_content);

                use similar::{ChangeTag, TextDiff};

                let diff = TextDiff::from_lines(&old_text, &new_text);

                if old_content.is_empty() {
                    output.push_str(&format!("+ Added regular file {}:\n", path_str));
                } else if new_content.is_empty() {
                    output.push_str(&format!("- Deleted regular file {}:\n", path_str));
                } else {
                    output.push_str(&format!("~ Modified regular file {}:\n", path_str));
                }

                if diff.ratio() < 1.0 || old_text != new_text {
                    let mut first_group = true;
                    for group in diff.grouped_ops(3) {
                        if !first_group {
                            output.push_str("    ...\n");
                        }
                        first_group = false;

                        for op in group {
                            for change in diff.iter_changes(&op) {
                                match change.tag() {
                                    ChangeTag::Delete => {
                                        output.push_str(&format!(
                                            "{:4}     : {}",
                                            change.old_index().unwrap() + 1,
                                            change.value()
                                        ));
                                    }
                                    ChangeTag::Insert => {
                                        output.push_str(&format!(
                                            "    {:5}: {}",
                                            change.new_index().unwrap() + 1,
                                            change.value()
                                        ));
                                    }
                                    ChangeTag::Equal => {
                                        output.push_str(&format!(
                                            "{:4}{:5}: {}",
                                            change.old_index().unwrap() + 1,
                                            change.new_index().unwrap() + 1,
                                            change.value()
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            output.push('\n');
        }

        if output.trim().is_empty() {
            Ok("(No changes found)".to_string())
        } else {
            Ok(output)
        }
    }

    async fn describe_revision(&self, change_id: &str, message: &str) -> Result<()> {
        let workspace = self.workspace.lock().await;
        let repo = workspace.repo_loader().load_at_head()?;

        // Assume change_id param is actually a Commit ID hex for now.
        let commit_id =
            JjCommitId::try_from_hex(change_id).ok_or_else(|| anyhow!("Invalid commit ID"))?;

        let commit = repo.store().get_commit(&commit_id)?;

        let mut tx = repo.start_transaction();
        let mut_repo = tx.repo_mut();

        mut_repo
            .rewrite_commit(&commit)
            .set_description(message)
            .write()?;

        mut_repo.rebase_descendants()?;

        tx.commit("describe revision")?;

        Ok(())
    }

    async fn snapshot(&self) -> Result<String> {
        let mut workspace = self.workspace.lock().await;

        let repo = workspace.repo_loader().load_at_head()?;
        let op_id = repo.operation().id().clone();

        let mut locked_ws = workspace.start_working_copy_mutation()?;

        let options = jj_lib::working_copy::SnapshotOptions {
            base_ignores: GitIgnoreFile::empty(),
            progress: None,
            start_tracking_matcher: &EverythingMatcher,
            force_tracking_matcher: &NothingMatcher,
            max_new_file_size: MAX_NEW_FILE_SIZE,
        };

        let (_tree, _stats) = locked_ws.locked_wc().snapshot(&options).await?;

        locked_ws.finish(op_id)?;

        Ok("Snapshot created".to_string())
    }

    async fn edit(&self, commit_id: &CommitId) -> Result<()> {
        let workspace = self.workspace.lock().await;
        let repo = workspace.repo_loader().load_at_head()?;

        let id = JjCommitId::try_from_hex(&commit_id.0).ok_or_else(|| anyhow!("Invalid commit ID"))?;
        let commit = repo.store().get_commit(&id)?;

        let mut tx = repo.start_transaction();
        let mut_repo = tx.repo_mut();

        // In jj, "editing" a commit means making it the working copy
        // Find workspace_id from repo view
        let (workspace_id, _) = repo
            .view()
            .wc_commit_ids()
            .iter()
            .next()
            .ok_or_else(|| anyhow!("No working copy found in view"))?;
        
        mut_repo.set_wc_commit(workspace_id.clone(), commit.id().clone())?;

        tx.commit("edit revision")?;
        Ok(())
    }

    async fn squash(&self, commit_id: &CommitId) -> Result<()> {
        let workspace = self.workspace.lock().await;
        let repo = workspace.repo_loader().load_at_head()?;

        let id = JjCommitId::try_from_hex(&commit_id.0).ok_or_else(|| anyhow!("Invalid commit ID"))?;
        let commit = repo.store().get_commit(&id)?;

        let mut parents = commit.parents();
        let parent = parents.next().ok_or_else(|| anyhow!("Cannot squash root commit"))??;

        // Merge the trees
        let merged_tree = {
            let parent_commit = repo.store().get_commit(parent.id())?;
            let parent_tree = parent_commit.tree();
            let commit_tree = commit.tree();
            jj_lib::merged_tree::MergedTree::merge(
                jj_lib::merge::Merge::from_removes_adds(
                    vec![(parent_tree.clone(), "".to_string())],
                    vec![
                        (parent_tree.clone(), "".to_string()),
                        (commit_tree.clone(), "".to_string()),
                    ],
                )
            ).await?
        };

        let mut tx = repo.start_transaction();
        let mut_repo = tx.repo_mut();

        // Squash commit into its parent
        let parent_commit = repo.store().get_commit(parent.id())?;
        
        // Combine descriptions
        let mut new_description = parent_commit.description().to_string();
        if !new_description.ends_with('\n') && !new_description.is_empty() {
            new_description.push('\n');
        }
        new_description.push_str(commit.description());

        mut_repo.rewrite_commit(&parent_commit)
            .set_tree(merged_tree)
            .set_description(new_description)
            .write()?;

        mut_repo.rebase_descendants()?;
        // Abandon the squashed commit
        mut_repo.record_abandoned_commit(&commit);

        tx.commit("squash revision")?;
        Ok(())
    }

    async fn new_child(&self, commit_id: &CommitId) -> Result<()> {
        let workspace = self.workspace.lock().await;
        let repo = workspace.repo_loader().load_at_head()?;

        let id = JjCommitId::try_from_hex(&commit_id.0).ok_or_else(|| anyhow!("Invalid commit ID"))?;
        let commit = repo.store().get_commit(&id)?;

        let mut tx = repo.start_transaction();
        let mut_repo = tx.repo_mut();

        // Resolve tree
        let tree = jj_lib::merged_tree::MergedTree::new(
            repo.store().clone(),
            commit.tree_ids().clone(),
            jj_lib::conflict_labels::ConflictLabels::unlabeled(),
        );

        mut_repo.new_commit(vec![id], tree).write()?;

        tx.commit("new revision")?;
        Ok(())
    }

    async fn abandon(&self, commit_id: &CommitId) -> Result<()> {
        let workspace = self.workspace.lock().await;
        let repo = workspace.repo_loader().load_at_head()?;

        let id = JjCommitId::try_from_hex(&commit_id.0).ok_or_else(|| anyhow!("Invalid commit ID"))?;
        let commit = repo.store().get_commit(&id)?;

        let mut tx = repo.start_transaction();
        let mut_repo = tx.repo_mut();

        mut_repo.record_abandoned_commit(&commit);
        mut_repo.rebase_descendants()?;

        tx.commit("abandon revision")?;
        Ok(())
    }

    async fn set_bookmark(&self, commit_id: &CommitId, name: &str) -> Result<()> {
        let _ = (commit_id, name);
        // Implementation would go here
        Ok(())
    }

    async fn delete_bookmark(&self, name: &str) -> Result<()> {
        let _ = name;
        // Implementation would go here
        Ok(())
    }

    async fn undo(&self) -> Result<()> {
        // TBD: Implementation of operation undo
        Ok(())
    }

    async fn redo(&self) -> Result<()> {
        // TBD: Implementation of operation redo
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jj_adapter_new() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let path = temp_dir.path();

        let config = StackedConfig::with_defaults();
        let user_settings = UserSettings::from_config(config)?;

        // Initialize a simple workspace
        Workspace::init_simple(&user_settings, path)?;

        // Instantiate JjAdapter using the temp dir
        let adapter = JjAdapter::load_at(path.to_path_buf());

        assert!(adapter.is_ok(), "JjAdapter should be initialized successfully");

        Ok(())
    }

    #[tokio::test]
    async fn test_snapshot_normal_file() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let path = temp_dir.path();

        let config = StackedConfig::with_defaults();
        let user_settings = UserSettings::from_config(config)?;

        Workspace::init_simple(&user_settings, path)?;
        let adapter = JjAdapter::load_at(path.to_path_buf())?;

        // Create a normal file
        let file_path = path.join("normal.txt");
        tokio::fs::write(&file_path, "Hello World").await?;

        // Snapshot
        let result = adapter.snapshot().await;
        assert!(result.is_ok(), "Snapshot should succeed for normal file");

        Ok(())
    }
}
