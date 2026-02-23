use crate::domain::{models::CommitId, vcs::VcsFacade};
use anyhow::{Context, Result};
use async_trait::async_trait;
use jj_lib::{
    local_working_copy::LocalWorkingCopyFactory, object_id::ObjectId, repo::StoreFactories,
    settings::UserSettings, working_copy::WorkingCopyFactory, workspace::Workspace,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

pub mod bookmarks;
pub mod diff;
pub mod log;
pub mod ops;
pub mod remote;
pub mod repo;

pub struct JjAdapter {
    pub(crate) workspace: Arc<Mutex<Option<Workspace>>>,
    pub(crate) workspace_root: PathBuf,
    pub(crate) user_settings: UserSettings,
    pub(crate) diff_semaphore: Arc<Semaphore>,
}

pub(crate) const MAX_DIFF_SIZE: u64 = 1024 * 1024; // 1MB
pub(crate) const MAX_CONCURRENT_DIFFS: usize = 4;

impl JjAdapter {
    pub fn new() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        Self::for_path(cwd)
    }

    pub fn for_path(path: PathBuf) -> Result<Self> {
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
                PathBuf::from(&home_dir).join(".jjconfig"),
                PathBuf::from(&home_dir).join(".jj/config.toml"),
                PathBuf::from(&home_dir).join(".config/jj/config.toml"),
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
            return Err(anyhow::anyhow!("'jj --version' failed"));
        }

        let version_str = String::from_utf8_lossy(&output.stdout);
        if !version_str.contains("0.38") {
            return Err(anyhow::anyhow!(
                "Judo expects jj version 0.38.x, but found: {}. \
                 Using mismatched versions may lead to repository corruption or crashes.",
                version_str.trim()
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl VcsFacade for JjAdapter {
    async fn get_operation_log(
        &self,
        heads: Option<Vec<CommitId>>,
        limit: usize,
        revset: Option<String>,
    ) -> Result<crate::domain::models::RepoStatus> {
        self.get_operation_log_impl(heads, limit, revset).await
    }

    async fn get_commit_diff(&self, commit_id: &CommitId) -> Result<String> {
        self.get_commit_diff_impl(commit_id).await
    }

    async fn describe_revision(&self, commit_id: &str, message: &str) -> Result<()> {
        self.describe_revision_impl(commit_id, message).await
    }

    async fn commit(&self, message: &str) -> Result<()> {
        self.commit_impl(message).await
    }

    async fn snapshot(&self) -> Result<String> {
        self.snapshot_impl().await
    }

    async fn edit(&self, commit_id: &CommitId) -> Result<()> {
        self.edit_impl(commit_id).await
    }

    async fn squash(&self, commit_ids: &[CommitId]) -> Result<()> {
        self.squash_impl(commit_ids).await
    }

    async fn new_child(&self, commit_id: &CommitId) -> Result<()> {
        self.new_child_impl(commit_id).await
    }

    async fn abandon(&self, commit_ids: &[CommitId]) -> Result<()> {
        self.abandon_impl(commit_ids).await
    }

    async fn revert(&self, commit_ids: &[CommitId]) -> Result<()> {
        self.revert_impl(commit_ids).await
    }

    async fn absorb(&self) -> Result<()> {
        self.absorb_impl().await
    }

    async fn duplicate(&self, commit_ids: &[CommitId]) -> Result<()> {
        self.duplicate_impl(commit_ids).await
    }

    async fn parallelize(&self, commit_ids: &[CommitId]) -> Result<()> {
        self.parallelize_impl(commit_ids).await
    }

    async fn rebase(&self, commit_ids: &[CommitId], destination: &str) -> Result<()> {
        self.rebase_impl(commit_ids, destination).await
    }

    async fn set_bookmark(&self, commit_id: &CommitId, name: &str) -> Result<()> {
        self.set_bookmark_impl(commit_id, name).await
    }

    async fn delete_bookmark(&self, name: &str) -> Result<()> {
        self.delete_bookmark_impl(name).await
    }

    async fn evolog(&self, commit_id: &CommitId) -> Result<String> {
        self.evolog_impl(commit_id).await
    }

    async fn operation_log(&self) -> Result<String> {
        self.operation_log_impl().await
    }

    async fn undo(&self) -> Result<()> {
        self.undo_impl().await
    }

    async fn redo(&self) -> Result<()> {
        self.redo_impl().await
    }

    async fn fetch(&self) -> Result<()> {
        self.fetch_impl().await
    }

    async fn push(&self, bookmark: Option<String>) -> Result<()> {
        self.push_impl(bookmark).await
    }

    async fn init_repo(&self) -> Result<()> {
        self.init_repo_impl().await
    }

    async fn is_valid(&self) -> bool {
        self.is_valid_impl().await
    }

    fn workspace_root(&self) -> PathBuf {
        self.workspace_root.clone()
    }
}

pub(crate) fn is_binary(chunk: &[u8]) -> bool {
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

pub(crate) fn format_change_id(id: &dyn ObjectId) -> String {
    id.hex()
        .chars()
        .map(|c: char| {
            let v = c.to_digit(16).unwrap_or(0);
            (b'k' + v as u8) as char
        })
        .collect()
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

    #[test]
    fn test_format_change_id() {
        use jj_lib::backend::CommitId;
        // 52fb4284e4449c413fe5f8a952b1a2f8e1d48d2b
        let id = CommitId::try_from_hex("52fb4284e4449c413fe5f8a952b1a2f8e1d48d2b").unwrap();
        let formatted = format_change_id(&id);
        // 5->p, 2->m, f->z, b->v, 4->o, 2->m, 8->s, 4->o, e->y, 4->o, 4->o, 4->o, 9->t, c->w, 4->o, 1->l, 3->n, f->z, e->y, 5->p, f->z, 8->s, a->u, 9->t, 5->p, 2->m, b->v, 1->l, a->u, 2->m, f->z, 8->s, e->y, 1->l, d->x, 4->o, 8->s, d->x, 2->m, b->v
        // Wait, it's 40 characters for 20 bytes.
        assert_eq!(formatted.len(), 40);
        assert!(formatted.chars().all(|c| c >= 'k' && c <= 'z'));
    }
}
