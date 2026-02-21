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
    op_store::RefTarget,
    op_walk,
    repo::{Repo, StoreFactories},
    settings::UserSettings,
    working_copy::WorkingCopyFactory,
    workspace::Workspace,
};
use jj_lib::ref_name::RefName;

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
        if let Ok(layer) = ConfigLayer::parse(
            ConfigSource::Default,
            crate::infrastructure::defaults::DEFAULT_FALLBACK_CONFIG,
        ) {
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
    async fn get_operation_log(
        &self,
        heads: Option<Vec<CommitId>>,
        limit: usize,
    ) -> Result<RepoStatus> {
        // Workspace is !Sync, so we lock it to access loader
        let repo = {
            let ws = self.workspace.lock().await;
            let op_id = ws.working_copy().operation_id();
            let op = ws.repo_loader().load_operation(&op_id)?;
            ws.repo_loader().load_at(&op)?
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

                // Pre-load trees and metadata needed for Phase 2
                let tree = commit.tree();
                let parent_tree = first_parent.as_ref().map(|p| p.tree());
                let is_working_copy = Some(&id) == repo_arc.view().get_wc_commit_id(&ws_id_clone);

                // Logic from refactor: heads or root commits are immutable
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

        // Phase 2: Async Detail Expansion (Parallel Diffing)
        let graph_rows = futures::stream::iter(commit_infos)
            .map(|(commit, tree, parent_tree, parent_ids, is_working_copy, is_immutable, bookmarks)| async move {
                 let description = commit.description().to_string();
                 let change_id = commit.change_id().hex();
                 let author = commit.author().email.clone();
                 let timestamp_sec = commit.author().timestamp.timestamp.0;
                 let datetime = chrono::DateTime::from_timestamp(timestamp_sec, 0)
                    .unwrap_or_default();
                 let timestamp = datetime.format("%Y-%m-%d %H:%M").to_string();
                 let commit_id = CommitId(commit.id().hex());

                 let mut changed_files = Vec::new();
                 if let Some(p_tree) = parent_tree {
                     let mut stream = p_tree.diff_stream(&tree, &EverythingMatcher);
                     while let Some(entry) = stream.next().await {
                         let status = if let Ok(values) = entry.values {
                            if !values.after.is_resolved() {
                                FileStatus::Conflicted
                            } else {
                                if values.before.is_absent() {
                                    FileStatus::Added
                                } else if values.after.is_absent() {
                                    FileStatus::Deleted
                                } else {
                                    FileStatus::Modified
                                }
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

        let entries = parent_tree
            .diff_stream(&tree, &EverythingMatcher)
            .collect::<Vec<_>>()
            .await;

        let diff_outputs = futures::stream::iter(entries)
            .map(|entry| {
                let repo = repo.clone();
                async move {
                    let entry = entry;
                    let path_str = entry.path.as_internal_file_string().to_string();

                    let diff = match entry.values {
                        Ok(v) => v,
                        Err(e) => {
                            return Ok::<String, anyhow::Error>(format!(
                                "Error reading diff values for {}: {}\n",
                                path_str, e
                            ))
                        }
                    };

                    let old_id = if let Ok(Some(jj_lib::backend::TreeValue::File { id, .. })) =
                        diff.before.into_resolved()
                    {
                        Some(id)
                    } else {
                        None
                    };

                    let new_id = if let Ok(Some(jj_lib::backend::TreeValue::File { id, .. })) =
                        diff.after.into_resolved()
                    {
                        Some(id)
                    } else {
                        None
                    };

                    if old_id.is_none() && new_id.is_none() {
                        return Ok(String::new());
                    }

                    // Read contents in parallel
                    let (old_content_res, new_content_res) = tokio::join!(
                        async {
                            if let Some(id) = old_id {
                                let mut content = Vec::new();
                                let mut reader = repo
                                    .store()
                                    .read_file(&entry.path, &id)
                                    .await?
                                    .take(MAX_DIFF_SIZE + 1);
                                reader.read_to_end(&mut content).await?;
                                Ok::<_, anyhow::Error>(Some(content))
                            } else {
                                Ok(None)
                            }
                        },
                        async {
                            if let Some(id) = new_id {
                                let mut content = Vec::new();
                                let mut reader = repo
                                    .store()
                                    .read_file(&entry.path, &id)
                                    .await?
                                    .take(MAX_DIFF_SIZE + 1);
                                reader.read_to_end(&mut content).await?;
                                Ok::<_, anyhow::Error>(Some(content))
                            } else {
                                Ok(None)
                            }
                        }
                    );

                    let old_content = old_content_res?.unwrap_or_default();
                    let new_content = new_content_res?.unwrap_or_default();

                    if old_content.len() as u64 > MAX_DIFF_SIZE
                        || new_content.len() as u64 > MAX_DIFF_SIZE
                    {
                        return Ok(format!("File {} is too large to diff\n\n", path_str));
                    }

                    let diff_output = tokio::task::spawn_blocking(move || {
                        let mut file_output = String::new();
                        // Simple binary check: check for null bytes in the first 1KB
                        let is_binary = old_content
                            .iter()
                            .chain(new_content.iter())
                            .take(1024)
                            .any(|&b| b == 0);

                        if is_binary {
                            file_output.push_str(&format!("Binary file {}\n\n", path_str));
                        } else {
                            let old_text = String::from_utf8_lossy(&old_content);
                            let new_text = String::from_utf8_lossy(&new_content);

                            use similar::{ChangeTag, TextDiff};

                            let diff = TextDiff::from_lines(&old_text, &new_text);

                            if old_content.is_empty() {
                                file_output
                                    .push_str(&format!("+ Added regular file {}:\n", path_str));
                            } else if new_content.is_empty() {
                                file_output
                                    .push_str(&format!("- Deleted regular file {}:\n", path_str));
                            } else {
                                file_output
                                    .push_str(&format!("~ Modified regular file {}:\n", path_str));
                            }

                            if diff.ratio() < 1.0 || old_text != new_text {
                                let mut first_group = true;
                                for group in diff.grouped_ops(3) {
                                    if !first_group {
                                        file_output.push_str("    ...\n");
                                    }
                                    first_group = false;

                                    for op in group {
                                        for change in diff.iter_changes(&op) {
                                            match change.tag() {
                                                ChangeTag::Delete => {
                                                    file_output.push_str(&format!(
                                                        "{:4}     : {}",
                                                        change.old_index().unwrap() + 1,
                                                        change.value()
                                                    ));
                                                }
                                                ChangeTag::Insert => {
                                                    file_output.push_str(&format!(
                                                        "    {:5}: {}",
                                                        change.new_index().unwrap() + 1,
                                                        change.value()
                                                    ));
                                                }
                                                ChangeTag::Equal => {
                                                    file_output.push_str(&format!(
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
                        file_output
                    })
                    .await?;

                    Ok(diff_output)
                }
            })
            .buffered(8)
            .collect::<Vec<Result<String>>>()
            .await;

        for res in diff_outputs {
            let chunk = res?;
            if !chunk.is_empty() {
                output.push_str(&chunk);
                output.push('\n');
            }
        }

        if output.trim().is_empty() {
            Ok("(No changes found)".to_string())
        } else {
            Ok(output)
        }
    }

    async fn describe_revision(&self, change_id: &str, message: &str) -> Result<()> {
        let mut ws = self.workspace.lock().await;
        let repo = ws.repo_loader().load_at_head()?;

        let change_id = change_id.to_string();
        let message = message.to_string();

        let new_repo = tokio::task::spawn_blocking(move || {
            let commit_id = JjCommitId::try_from_hex(&change_id)
                .ok_or_else(|| anyhow!("Invalid commit ID"))?;

            let commit = repo.store().get_commit(&commit_id)?;

            let mut tx = repo.start_transaction();
            let mut_repo = tx.repo_mut();

            mut_repo
                .rewrite_commit(&commit)
                .set_description(message)
                .write()?;

            mut_repo.rebase_descendants()?;

            Ok::<_, anyhow::Error>(tx.commit("describe revision")?)
        })
        .await??;

        let locked_ws = ws.start_working_copy_mutation()?;
        locked_ws.finish(new_repo.operation().id().clone())?;
        Ok(())
    }

    async fn snapshot(&self) -> Result<String> {
        let mut workspace = self.workspace.lock().await;

        let repo = workspace.repo_loader().load_at_head()?;

        let mut locked_ws = workspace.start_working_copy_mutation()?;

        let options = jj_lib::working_copy::SnapshotOptions {
            base_ignores: GitIgnoreFile::empty(),
            progress: None,
            start_tracking_matcher: &EverythingMatcher,
            force_tracking_matcher: &NothingMatcher,
            max_new_file_size: MAX_NEW_FILE_SIZE,
        };

        let (tree, _stats) = locked_ws.locked_wc().snapshot(&options).await?;

        let (_workspace_id, wc_commit_id) = repo
            .view()
            .wc_commit_ids()
            .iter()
            .next()
            .ok_or_else(|| anyhow!("No working copy found in view"))?;

        let wc_commit = repo.store().get_commit(wc_commit_id)?;

        if wc_commit.tree_ids() != tree.tree_ids() {
            let mut tx = repo.start_transaction();
            tx.repo_mut()
                .rewrite_commit(&wc_commit)
                .set_tree(tree)
                .write()?;
            tx.repo_mut().rebase_descendants()?;
            let repo = tx.commit("snapshot")?;
            locked_ws.finish(repo.operation().id().clone())?;
            Ok("Snapshot created".to_string())
        } else {
            locked_ws.finish(repo.operation().id().clone())?;
            Ok("Snapshot created".to_string())
        }
    }

    async fn edit(&self, commit_id: &CommitId) -> Result<()> {
        let mut ws = self.workspace.lock().await;
        let repo = ws.repo_loader().load_at_head()?;

        let commit_id_hex = commit_id.0.clone();

        let new_repo = tokio::task::spawn_blocking(move || {
            let id = JjCommitId::try_from_hex(&commit_id_hex)
                .ok_or_else(|| anyhow!("Invalid commit ID"))?;
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

            Ok::<_, anyhow::Error>(tx.commit("edit revision")?)
        })
        .await??;

        let locked_ws = ws.start_working_copy_mutation()?;
        locked_ws.finish(new_repo.operation().id().clone())?;
        Ok(())
    }

    async fn squash(&self, commit_id: &CommitId) -> Result<()> {
        let mut ws = self.workspace.lock().await;
        let repo = ws.repo_loader().load_at_head()?;

        let commit_id_hex = commit_id.0.clone();

        let (commit, parent_id) = tokio::task::spawn_blocking({
            let repo = repo.clone();
            move || {
                let id = JjCommitId::try_from_hex(&commit_id_hex)
                    .ok_or_else(|| anyhow!("Invalid commit ID"))?;
                let commit = repo.store().get_commit(&id)?;

                let parent_id = {
                    let mut parents = commit.parents();
                    let parent = parents
                        .next()
                        .ok_or_else(|| anyhow!("Cannot squash root commit"))??;
                    parent.id().clone()
                };

                Ok::<_, anyhow::Error>((commit, parent_id))
            }
        })
        .await??;

        // Merge the trees
        let merged_tree = {
            let parent_commit = repo.store().get_commit(&parent_id)?;
            let parent_tree = parent_commit.tree();
            let commit_tree = commit.tree();
            jj_lib::merged_tree::MergedTree::merge(jj_lib::merge::Merge::from_removes_adds(
                vec![(parent_tree.clone(), "".to_string())],
                vec![
                    (parent_tree.clone(), "".to_string()),
                    (commit_tree.clone(), "".to_string()),
                ],
            ))
            .await?
        };

        let new_repo = tokio::task::spawn_blocking(move || {
            let mut tx = repo.start_transaction();
            let mut_repo = tx.repo_mut();

            // Squash commit into its parent
            let parent_commit = mut_repo.store().get_commit(&parent_id)?;

            // Combine descriptions
            let mut new_description = parent_commit.description().to_string();
            if !new_description.ends_with('\n') && !new_description.is_empty() {
                new_description.push('\n');
            }
            new_description.push_str(commit.description());

            mut_repo
                .rewrite_commit(&parent_commit)
                .set_tree(merged_tree)
                .set_description(new_description)
                .write()?;

            mut_repo.rebase_descendants()?;
            // Abandon the squashed commit
            mut_repo.record_abandoned_commit(&commit);

            Ok::<_, anyhow::Error>(tx.commit("squash revision")?)
        })
        .await??;

        let locked_ws = ws.start_working_copy_mutation()?;
        locked_ws.finish(new_repo.operation().id().clone())?;
        Ok(())
    }

    async fn new_child(&self, commit_id: &CommitId) -> Result<()> {
        let mut ws = self.workspace.lock().await;
        let repo = ws.repo_loader().load_at_head()?;

        let commit_id_hex = commit_id.0.clone();

        let new_repo = tokio::task::spawn_blocking(move || {
            let id = JjCommitId::try_from_hex(&commit_id_hex)
                .ok_or_else(|| anyhow!("Invalid commit ID"))?;
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

            Ok::<_, anyhow::Error>(tx.commit("new revision")?)
        })
        .await??;

        let locked_ws = ws.start_working_copy_mutation()?;
        locked_ws.finish(new_repo.operation().id().clone())?;
        Ok(())
    }

    async fn abandon(&self, commit_id: &CommitId) -> Result<()> {
        let mut ws = self.workspace.lock().await;
        let repo = ws.repo_loader().load_at_head()?;

        let commit_id_hex = commit_id.0.clone();

        let new_repo = tokio::task::spawn_blocking(move || {
            let id = JjCommitId::try_from_hex(&commit_id_hex)
                .ok_or_else(|| anyhow!("Invalid commit ID"))?;
            let commit = repo.store().get_commit(&id)?;

            let mut tx = repo.start_transaction();
            let mut_repo = tx.repo_mut();

            mut_repo.record_abandoned_commit(&commit);
            mut_repo.rebase_descendants()?;

            Ok::<_, anyhow::Error>(tx.commit("abandon revision")?)
        })
        .await??;

        let locked_ws = ws.start_working_copy_mutation()?;
        locked_ws.finish(new_repo.operation().id().clone())?;
        Ok(())
    }

    async fn set_bookmark(&self, commit_id: &CommitId, name: &str) -> Result<()> {
        let mut ws = self.workspace.lock().await;
        let repo = ws.repo_loader().load_at_head()?;

        let commit_id_hex = commit_id.0.clone();
        let name = name.to_string();

        let new_repo = tokio::task::spawn_blocking(move || {
            let id = JjCommitId::try_from_hex(&commit_id_hex)
                .ok_or_else(|| anyhow!("Invalid commit ID"))?;

            let mut tx = repo.start_transaction();
            tx.repo_mut()
                .set_local_bookmark_target(RefName::new(&name), RefTarget::normal(id));

            Ok::<_, anyhow::Error>(tx.commit(format!("set bookmark {}", name))?)
        })
        .await??;

        let locked_ws = ws.start_working_copy_mutation()?;
        locked_ws.finish(new_repo.operation().id().clone())?;
        Ok(())
    }

    async fn delete_bookmark(&self, name: &str) -> Result<()> {
        let mut ws = self.workspace.lock().await;
        let repo = ws.repo_loader().load_at_head()?;

        let name = name.to_string();

        let new_repo = tokio::task::spawn_blocking(move || {
            let mut tx = repo.start_transaction();
            tx.repo_mut()
                .set_local_bookmark_target(RefName::new(&name), RefTarget::absent());

            Ok::<_, anyhow::Error>(tx.commit(format!("delete bookmark {}", name))?)
        })
        .await??;

        let locked_ws = ws.start_working_copy_mutation()?;
        locked_ws.finish(new_repo.operation().id().clone())?;
        Ok(())
    }

    async fn undo(&self) -> Result<()> {
        let mut ws = self.workspace.lock().await;
        let op_id = ws.working_copy().operation_id();
        let op = ws.repo_loader().load_operation(&op_id)?;
        let parent_op = op
            .parents()
            .next()
            .ok_or_else(|| anyhow!("No operation to undo"))??;

        let locked_ws = ws.start_working_copy_mutation()?;
        locked_ws.finish(parent_op.id().clone())?;
        Ok(())
    }

    async fn redo(&self) -> Result<()> {
        let mut ws = self.workspace.lock().await;
        let op_id = ws.working_copy().operation_id();

        let op_heads_store = ws.repo_loader().op_heads_store();
        let heads = op_heads_store.get_op_heads().await?;
        let loader = ws.repo_loader();
        let head_ops: Vec<_> = heads
            .iter()
            .map(|id| loader.load_operation(id))
            .collect::<Result<Vec<_>, _>>()?;

        let mut walk = op_walk::walk_ancestors(&head_ops);
        while let Some(op_result) = walk.next() {
            let op = op_result?;
            if op
                .parents()
                .any(|p_res| p_res.as_ref().map(|p| p.id()).ok() == Some(&op_id))
            {
                let locked_ws = ws.start_working_copy_mutation()?;
                locked_ws.finish(op.id().clone())?;
                return Ok(());
            }
        }

        Err(anyhow!("No operation to redo"))
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

        assert!(
            adapter.is_ok(),
            "JjAdapter should be initialized successfully"
        );

        Ok(())
    }

#[tokio::test]
    async fn test_new_child() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let path = temp_dir.path();

        let config = StackedConfig::with_defaults();
        let user_settings = UserSettings::from_config(config)?;

        // Initialize a simple workspace
        Workspace::init_simple(&user_settings, path)?;

        // Instantiate JjAdapter using the temp dir
        let adapter = JjAdapter::load_at(path.to_path_buf())?;

        // Get initial state
        let status = adapter.get_operation_log(None, 100).await?;

        let parent_commit = status.graph.first().ok_or_else(|| anyhow!("Graph is empty"))?;
        let parent_id = parent_commit.commit_id.clone();

        // Create new child
        adapter.new_child(&parent_id).await?;

        // Verify
        let new_status = adapter.get_operation_log(None, 100).await?;

        // Find a commit that has parent_id as parent
        let child_commit = new_status.graph.iter().find(|row| {
             row.parents.contains(&parent_id)
        });

        assert!(child_commit.is_some(), "Should have created a child commit of {}", parent_id);

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

    #[tokio::test]
    async fn test_get_operation_log_capped() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let path = temp_dir.path();

        let config = StackedConfig::with_defaults();
        let user_settings = UserSettings::from_config(config)?;

        // Initialize a simple workspace
        Workspace::init_simple(&user_settings, path)?;

        let adapter = JjAdapter::load_at(path.to_path_buf())?;

        // Setup 150 commits
        {
            let mut workspace = adapter.workspace.lock().await;
            let repo = workspace.repo_loader().load_at_head()?;

            let mut tx = repo.start_transaction();
            let mut_repo = tx.repo_mut();

            let mut parent_id = repo.store().root_commit_id().clone();

            // Create 150 linear commits
            let root_commit = repo.store().get_commit(&parent_id)?;
            let tree = root_commit.tree();
            for i in 0..150 {
                let commit = mut_repo
                    .new_commit(vec![parent_id.clone()], tree.clone())
                    .set_description(format!("Commit {}", i))
                    .write()?;
                parent_id = commit.id().clone();
            }

            let (workspace_id, _) = repo
                .view()
                .wc_commit_ids()
                .iter()
                .next()
                .ok_or_else(|| anyhow!("No working copy found in view"))?;

            mut_repo.set_wc_commit(workspace_id.clone(), parent_id.clone())?;

            let new_repo = tx.commit("create 150 commits")?;
            let locked_ws = workspace.start_working_copy_mutation()?;
            locked_ws.finish(new_repo.operation().id().clone())?;
        }

        let log = adapter.get_operation_log(None, 100).await?;

        assert_eq!(log.graph.len(), 100, "Should return 100 commits (capped)");

        Ok(())
    }

    #[tokio::test]
    async fn test_bookmark_management() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let path = temp_dir.path();

        let config = StackedConfig::with_defaults();
        let user_settings = UserSettings::from_config(config)?;

        Workspace::init_simple(&user_settings, path)?;
        let adapter = JjAdapter::load_at(path.to_path_buf())?;

        let status = adapter.get_operation_log(None, 100).await?;
        let commit_id = status.graph.first().unwrap().commit_id.clone();

        // Set bookmark
        adapter.set_bookmark(&commit_id, "test-bookmark").await?;

        // Verify bookmark exists
        let status = adapter.get_operation_log(None, 100).await?;
        let commit = status.graph.first().unwrap();
        assert!(commit.bookmarks.contains(&"test-bookmark".to_string()));

        // Delete bookmark
        adapter.delete_bookmark("test-bookmark").await?;

        // Verify bookmark is gone
        let status = adapter.get_operation_log(None, 100).await?;
        let commit = status.graph.first().unwrap();
        assert!(!commit.bookmarks.contains(&"test-bookmark".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_undo_redo() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let path = temp_dir.path();

        let config = StackedConfig::with_defaults();
        let user_settings = UserSettings::from_config(config)?;

        Workspace::init_simple(&user_settings, path)?;
        let adapter = JjAdapter::load_at(path.to_path_buf())?;

        let initial_status = adapter.get_operation_log(None, 100).await?;
        let initial_op_id = initial_status.operation_id;

        // Perform an operation (e.g., set a bookmark)
        let commit_id = initial_status.graph.first().unwrap().commit_id.clone();
        adapter.set_bookmark(&commit_id, "undo-test").await?;

        let mid_status = adapter.get_operation_log(None, 100).await?;
        let mid_op_id = mid_status.operation_id;
        assert_ne!(initial_op_id, mid_op_id);

        // Undo
        adapter.undo().await?;
        let undo_status = adapter.get_operation_log(None, 100).await?;
        assert_eq!(undo_status.operation_id, initial_op_id);

        // Redo
        adapter.redo().await?;
        let redo_status = adapter.get_operation_log(None, 100).await?;
        assert_eq!(redo_status.operation_id, mid_op_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_commit_diff() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let path = temp_dir.path();

        let config = StackedConfig::with_defaults();
        let user_settings = UserSettings::from_config(config)?;

        Workspace::init_simple(&user_settings, path)?;
        let adapter = JjAdapter::load_at(path.to_path_buf())?;

        // 1. Test Added File
        let file_path = path.join("test.txt");
        tokio::fs::write(&file_path, "Line 1\nLine 2\n").await?;

        adapter.snapshot().await?;

        let status = adapter.get_operation_log(None, 100).await?;
        let commit_id = status.graph.first().unwrap().commit_id.clone();

        let diff = adapter.get_commit_diff(&commit_id).await?;

        assert!(diff.contains("test.txt"));
        assert!(diff.contains("+ Added regular file test.txt"));
        assert!(diff.contains("Line 1"));
        assert!(diff.contains("Line 2"));

        // 2. Test Modified File
        // Create a new child commit so the parent has the file
        adapter.new_child(&commit_id).await?;
        
        // In jj, new_child doesn't necessarily move the working copy.
        // We need to edit the new child to make it the working copy.
        let status = adapter.get_operation_log(None, 100).await?;
        let new_commit_id = status.graph.first().unwrap().commit_id.clone();
        adapter.edit(&new_commit_id).await?;
        
        tokio::fs::write(&file_path, "Line 1\nLine 2 modified\n").await?;
        adapter.snapshot().await?;

        let status = adapter.get_operation_log(None, 100).await?;
        let commit_id = status.graph.first().unwrap().commit_id.clone();

        let diff = adapter.get_commit_diff(&commit_id).await?;
        
        assert!(diff.contains("test.txt"));
        assert!(diff.contains("~ Modified regular file test.txt"));
        assert!(diff.contains("Line 2"));
        assert!(diff.contains("Line 2 modified"));

        Ok(())
    }
}
