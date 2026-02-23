use super::JjAdapter;
use crate::domain::models::{CommitId, FileChange, FileStatus, GraphRow, RepoStatus};
use anyhow::{anyhow, Result};
use futures::StreamExt;
use jj_lib::{
    backend::CommitId as JjCommitId, matchers::EverythingMatcher, object_id::ObjectId,
    ref_name::WorkspaceNameBuf,
};
use std::collections::{HashSet, VecDeque};

impl JjAdapter {
    pub(crate) async fn get_operation_log_impl(
        &self,
        heads: Option<Vec<CommitId>>,
        limit: usize,
        revset: Option<String>,
    ) -> Result<RepoStatus> {
        let (repo, ws_root) = self.get_repo_and_ws().await?;
        let op_id = repo.operation().id().clone().hex();

        let (workspace_id, _): (&WorkspaceNameBuf, &JjCommitId) = repo
            .view()
            .wc_commit_ids()
            .iter()
            .next()
            .ok_or_else(|| anyhow!("No working copy found"))?;

        let workspace_id_buf: WorkspaceNameBuf = workspace_id.clone();
        let repo_arc = repo.clone();
        let ws_id_clone = workspace_id_buf.clone();
        let ws_root_for_closure = ws_root.clone();

        let commit_infos = tokio::task::spawn_blocking(move || {
            let mut visited = HashSet::<JjCommitId>::new();
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
                                    if let Ok(info) = super::repo::build_commit_info(&repo_arc, &id, &ws_id_clone) {
                                        results.push(info);
                                    }
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

            if let Some(heads) = heads {
                for head in heads {
                    if let Some(id) = JjCommitId::try_from_hex(&head.0) {
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

                if let Ok(info) = super::repo::build_commit_info(&repo_arc, &id, &ws_id_clone) {
                    for parent_id in info.commit.parent_ids() {
                        queue.push_back(parent_id.clone());
                    }
                    results.push(info);
                }
            }
            Ok::<_, anyhow::Error>(results)
        })
        .await??;

        let graph_rows = futures::stream::iter(commit_infos)
            .map(|info| async move {
                let commit = info.commit;
                let parent_tree = info.parent_tree;
                let parent_ids = info.parent_ids;
                let is_working_copy = info.is_working_copy;
                let is_immutable = info.is_immutable;
                let has_conflict = info.has_conflict;
                let bookmarks = info.bookmarks;

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
                    let commit_tree = commit.tree();
                    let mut stream = p_tree.diff_stream(&commit_tree, &EverythingMatcher);
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
                    has_conflict,
                    parents: parent_ids,
                    bookmarks,
                    changed_files,
                    visual: crate::domain::models::GraphRowVisual::default(),
                }
            })
            .buffered(50)
            .collect::<Vec<_>>()
            .await;

        let wc_id = match repo.view().get_wc_commit_id(workspace_id) {
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

    pub(crate) async fn evolog_impl(&self, commit_id: &CommitId) -> Result<String> {
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let output = tokio::process::Command::new("jj")
            .arg("evolog")
            .arg("-r")
            .arg(&commit_id.0)
            .arg("--color")
            .arg("always")
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

    pub(crate) async fn operation_log_impl(&self) -> Result<String> {
        let (_, ws_root) = self.get_repo_and_ws().await?;
        let output = tokio::process::Command::new("jj")
            .arg("op")
            .arg("log")
            .arg("--color")
            .arg("always")
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
}
