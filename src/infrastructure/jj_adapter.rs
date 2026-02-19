use crate::domain::{vcs::VcsFacade, models::{RepoStatus, CommitId, GraphRow}};
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use jj_lib::{
    backend::CommitId as JjCommitId,
    repo::{Repo, StoreFactories}, 
    settings::{UserSettings},
    workspace::{Workspace},
    object_id::ObjectId,
    merged_tree::MergedTree,
    config::{StackedConfig, ConfigLayer, ConfigSource}, 
    local_working_copy::LocalWorkingCopyFactory,
    working_copy::WorkingCopyFactory,
};

use jj_lib::gitignore::GitIgnoreFile;
use jj_lib::matchers::{EverythingMatcher, NothingMatcher};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::{HashMap, HashSet, VecDeque};
use futures::StreamExt; 

pub struct JjAdapter {
    workspace: Arc<Mutex<Workspace>>,
}

impl JjAdapter {
    pub fn new() -> Result<Self> {
        let cwd = std::env::current_dir().context("Failed to get current working directory")?;
        
        let mut config = StackedConfig::empty(); 
        
        // Layer 1: Internal Defaults (Lowest priority fallback)
        // This prevents initialization failures when mandatory jj config is missing.
        let default_config_str = r#"
            [fsmonitor]
            backend = "none"
            [fsmonitor.watchman]
            register-snapshot-trigger = false
            [git]
            abandon-unreachable-commits = true
            auto-local-bookmark = false
            executable-path = "git"
            write-change-id-header = true
            [merge]
            hunk-level = "line"
            same-change = "accept"
            [operation]
            hostname = "judo-host"
            username = "judo-user"
            [signing]
            backend = "none"
            behavior = "own"
            [signing.backends.gpg]
            allow-expired-keys = false
            program = "gpg"
            [signing.backends.gpgsm]
            allow-expired-keys = false
            program = "gpgsm"
            [signing.backends.ssh]
            program = "ssh-keygen"
            [ui]
            conflict-marker-style = "diff"
            [user]
            name = "Judo User"
            email = "judo@example.com"
            [working-copy]
            eol-conversion = "none"
            exec-bit-change = "auto"
            [experimental]
            record-predecessors-in-commit = true
        "#;
        
        if let Ok(layer) = ConfigLayer::parse(ConfigSource::Default, default_config_str) {
            config.add_layer(layer);
        }

        // Layer 2: User Config (Higher priority)
        if let Ok(home) = std::env::var("HOME") {
             let paths = [
                 std::path::PathBuf::from(&home).join(".jj/config.toml"),
                 std::path::PathBuf::from(&home).join(".config/jj/config.toml"),
             ];
             for path in paths {
                 if path.exists() {
                     if let Ok(text) = std::fs::read_to_string(&path) {
                         if let Ok(layer) = ConfigLayer::parse(
                             ConfigSource::User, 
                             &text
                         ) {
                              config.add_layer(layer);
                         }
                     }
                 }
             }
        }

        let user_settings = UserSettings::from_config(config)?;
        
        let store_factories = StoreFactories::default();
        let mut working_copy_factories: HashMap<String, Box<dyn WorkingCopyFactory>> = HashMap::new();
        working_copy_factories.insert("local".to_string(), Box::new(LocalWorkingCopyFactory {}));
        
        let workspace = Workspace::load(&user_settings, &cwd, &store_factories, &working_copy_factories)
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
        let (workspace_id, _) = repo.view().wc_commit_ids().iter().next()
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

        // Also add working copy parent? 
        // For now just heads is enough to see something.
        
        while let Some(id) = queue.pop_front() {
            if visited.contains(&id) { continue; }
            visited.insert(id.clone());
            
            if graph_rows.len() >= 100 { break; } // Hard limit for MVP
            
            let commit = repo.store().get_commit(&id)?;
            
            for parent in commit.parents() {
                // commit.parents() yields Result<Commit>
                if let Ok(parent) = parent {
                     queue.push_back(parent.id().clone());
                }
            }

            let description = commit.description().lines().next().unwrap_or("").to_string();
            let change_id = commit.change_id().hex(); 
            let author = commit.author().email.clone();
            // Timestamp fix: timestamp struct -> timestamp field -> 0 (millis)
            let timestamp = commit.author().timestamp.timestamp.0.to_string(); 
            
            graph_rows.push(GraphRow {
                commit_id: CommitId(id.hex()),
                change_id,
                description,
                author,
                timestamp,
                is_working_copy: false, 
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
        
        let id = JjCommitId::try_from_hex(&commit_id.0).ok_or_else(|| anyhow!("Invalid commit ID"))?;
        let commit = repo.store().get_commit(&id)?;
        let mut parents = commit.parents();
        
        let parent_tree = if let Some(parent) = parents.next() {
            // parent is Result<Commit>
            parent?.tree() // Infallible MergedTree
        } else {
             return Ok("Root commit (no parent)".to_string());
        };
        
        let tree: MergedTree = commit.tree(); // Infallible
        
        let mut output = String::new();
        
        // try diff_stream
        let mut stream = tree.diff_stream(&parent_tree, &jj_lib::matchers::EverythingMatcher);
        while let Some(chunk) = stream.next().await {
             // chunk is (Vec<PathNode>, DiffNode)? 
             // We'll debug print it content for now
             // Or format it.
             // chunk is Result usually?
             // Since I can't check, I'll use format!("{:?}", chunk)
             output.push_str(&format!("{:?}\n", chunk));
        }
        
        if output.is_empty() {
             Ok("(No changes or diff not implemented)".to_string())
        } else {
            Ok(output)
        }
    }

    async fn describe_revision(&self, change_id: &str, message: &str) -> Result<()> {
        let workspace = self.workspace.lock().await;
        let repo = workspace.repo_loader().load_at_head()?;
        
        // Assume change_id param is actually a Commit ID hex for now.
        let commit_id = JjCommitId::try_from_hex(change_id)
            .ok_or_else(|| anyhow!("Invalid commit ID"))?;
            
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
            max_new_file_size: u64::MAX,
        };

        let (_tree, _stats) = locked_ws.locked_wc().snapshot(&options).await?;
        
        locked_ws.finish(op_id)?;

        Ok("Snapshot created".to_string())
    }
}
