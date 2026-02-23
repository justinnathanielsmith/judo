use super::{JjAdapter, MAX_DIFF_SIZE};
use crate::domain::models::CommitId;
use anyhow::{anyhow, Result};
use futures::StreamExt;
use jj_lib::{
    backend::{CommitId as JjCommitId, TreeValue},
    matchers::EverythingMatcher,
    object_id::ObjectId,
    repo::Repo,
};
use tokio::io::AsyncReadExt;

impl JjAdapter {
    pub(crate) async fn get_commit_diff_impl(&self, commit_id: &CommitId) -> Result<String> {
        let (repo, _) = self.get_repo_and_ws().await?;

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
            .filter(|(_, target)| target.added_ids().any(|added_id| *added_id == id))
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
        let parent_tree = if let Some(parent_res) = parents.next() {
            let parent: jj_lib::commit::Commit = parent_res?;
            parent.tree()
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
                    if super::is_binary(&chunk) {
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
                    if super::is_binary(&chunk) {
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
}
