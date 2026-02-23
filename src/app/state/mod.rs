use super::keymap::{KeyConfig, KeyMap};
use crate::domain::models::{CommitId, RepoStatus};
use std::sync::Arc;
use std::time::Instant;

pub mod command_palette;
pub mod context_menu;
pub mod error;
pub mod extra;
pub mod header;
pub mod input;
pub mod log;
pub mod revset;
pub mod theme;

// Re-exports
pub use command_palette::CommandPaletteState;
pub use context_menu::ContextMenuState;
pub use error::{ErrorSeverity, ErrorState};
pub use extra::{EvologState, OperationLogState};
pub use header::HeaderState;
pub use input::{AppTextArea, InputState};
pub use log::{LogState, Panel};
pub use revset::{get_revset_reference, RevsetCategory, RevsetEntry};
pub use theme::ThemeSelectionState;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AppMode {
    Normal,         // Navigating the log
    SquashSelect,   // Selecting a target to squash into
    BookmarkInput,  // Inputting a bookmark name
    CommitInput,    // Inputting a commit message
    Input,          // A generic text input modal (e.g., for commit messages)
    Loading,        // Blocking interaction (optional, often better handled with a flag)
    Diff,           // Focusing the diff window for scrolling
    ContextMenu,    // Right-click menu for actions
    FilterInput,    // Inputting a revset filter
    Help,           // Showing the help overlay
    NoRepo,         // No repository found, showing welcome screen
    CommandPalette, // Fuzzy finder for commands
    ThemeSelection, // Choosing a UI theme
    RebaseInput,    // Inputting rebase destination
    RebaseSelect,   // Selecting rebase destination
    Evolog,         // Viewing commit evolution log
    OperationLog,   // Viewing operation log
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppState<'a> {
    // --- Connectivity & Status ---
    pub should_quit: bool,
    pub mode: AppMode,
    pub last_error: Option<ErrorState>,
    pub status_message: Option<String>, // "Snapshot created."
    pub status_clear_time: Option<Instant>,
    pub workspace_id: String,
    pub active_tasks: Vec<String>,

    // --- JJ Data (The "Source of Truth") ---
    // We wrap this in Option because we might start before the repo is loaded.
    pub repo: Option<RepoStatus>,
    pub revset: Option<String>,
    pub is_loading_more: bool,
    pub has_more: bool,

    // --- UI State (Selection, Scroll, Diff) ---
    pub log: LogState,

    // --- Derived/Cached Data ---
    pub show_diffs: bool,
    pub header_state: HeaderState,
    pub spinner: String,

    // --- Input Handling ---
    pub input: Option<InputState<'a>>,

    // --- Click Tracking ---
    pub last_click_time: Option<Instant>,
    pub last_click_pos: Option<(u16, u16)>,

    // --- Context Menu ---
    pub context_menu: Option<ContextMenuState>,

    // --- Command Palette ---
    pub command_palette: Option<CommandPaletteState>,

    // --- Theme Selection ---
    pub theme_selection: Option<ThemeSelectionState>,

    // --- Animation ---
    pub frame_count: u64,
    pub hunk_highlight_time: Option<Instant>,

    // --- Layout ---
    pub diff_ratio: u16,
    pub focused_panel: Panel,

    // --- Config ---
    pub keymap: Arc<KeyMap>,
    pub palette_type: crate::theme::PaletteType,
    pub theme: crate::theme::Theme,

    // --- Filters ---
    pub recent_filters: Vec<String>,
    pub preset_filters: Vec<String>,
    pub selected_filter_index: Option<usize>,
    pub is_selecting_presets: bool,

    // --- Evolog ---
    pub evolog_state: Option<EvologState>,

    // --- Operation Log ---
    pub operation_log_state: Option<OperationLogState>,

    // --- Rebase State ---
    pub rebase_sources: Vec<CommitId>,
}

impl AppState<'_> {
    #[must_use]
    pub fn new(config: KeyConfig) -> Self {
        Self {
            keymap: Arc::new(KeyMap::from_config(&config)),
            recent_filters: super::persistence::load_recent_filters(),
            preset_filters: default_preset_filters(),
            ..Default::default()
        }
    }

    pub fn get_selected_file(&self) -> Option<&crate::domain::models::FileChange> {
        if let (Some(repo), Some(idx)) = (&self.repo, self.log.list_state.selected()) {
            if let Some(row) = repo.graph.get(idx) {
                if let Some(file_idx) = self.log.selected_file_index {
                    return row.changed_files.get(file_idx);
                }
            }
        }
        None
    }

    pub fn is_selected_file_conflicted(&self) -> bool {
        self.get_selected_file()
            .is_some_and(|f| f.status == crate::domain::models::FileStatus::Conflicted)
    }

    pub fn get_selected_commit_ids(&self) -> Vec<CommitId> {
        if !self.log.selected_ids.is_empty() {
            self.log.selected_ids.iter().cloned().collect()
        } else if let (Some(repo), Some(idx)) = (&self.repo, self.log.list_state.selected()) {
            if let Some(row) = repo.graph.get(idx) {
                return vec![row.commit_id.clone()];
            }
            Vec::new()
        } else {
            Vec::new()
        }
    }
}

impl Default for AppState<'_> {
    fn default() -> Self {
        Self {
            should_quit: false,
            mode: AppMode::Normal,
            last_error: None,
            status_message: None,
            status_clear_time: None,
            workspace_id: String::new(),
            active_tasks: Vec::new(),
            repo: None,
            revset: None,
            is_loading_more: false,
            has_more: true,
            log: LogState::default(),
            show_diffs: false,
            header_state: HeaderState::default(),
            spinner: "⠋".to_string(),
            input: None,
            last_click_time: None,
            last_click_pos: None,
            context_menu: None,
            command_palette: None,
            theme_selection: None,
            frame_count: 0,
            hunk_highlight_time: None,
            diff_ratio: 50,
            focused_panel: Panel::Graph,
            keymap: Arc::new(KeyMap::from_config(&KeyConfig::default())),
            palette_type: crate::theme::PaletteType::CatppuccinMocha,
            theme: crate::theme::Theme::default(),
            recent_filters: Vec::new(),
            preset_filters: default_preset_filters(),
            selected_filter_index: None,
            is_selecting_presets: false,
            evolog_state: None,
            operation_log_state: None,
            rebase_sources: Vec::new(),
        }
    }
}

fn default_preset_filters() -> Vec<String> {
    vec![
        // Scope
        "all()".to_string(),
        "mine()".to_string(),
        "trunk()".to_string(),
        "mutable()".to_string(),
        "immutable()".to_string(),
        "visible_heads()".to_string(),
        // Bookmarks & Tags
        "bookmarks()".to_string(),
        "remote_bookmarks()".to_string(),
        "tracked_remote_bookmarks()".to_string(),
        "tags()".to_string(),
        // Conflicts & State
        "conflicts()".to_string(),
        "divergent()".to_string(),
        "empty()".to_string(),
        "merges()".to_string(),
        "signed()".to_string(),
        // Ancestry & DAG
        "heads(all())".to_string(),
        "roots(all())".to_string(),
        "ancestors(@)".to_string(),
        "descendants(@)".to_string(),
        "working_copies()".to_string(),
    ]
}
