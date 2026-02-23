use super::action::Action;
use super::keymap::{KeyConfig, KeyMap};
use crate::domain::models::{CommitId, RepoStatus};
use chrono::{DateTime, Local};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{TableState, Widget};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Instant;
use tui_textarea::{CursorMove, TextArea};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Panel {
    Graph,
    Diff,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AppMode {
    Normal,         // Navigating the log
    SquashSelect,   // Selecting a target to squash into
    BookmarkInput,  // Inputting a bookmark name
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
    Evolog,         // Viewing commit evolution log
    OperationLog,   // Viewing operation log
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorState {
    pub message: String,
    pub timestamp: DateTime<Local>,
    pub severity: ErrorSeverity,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextMenuState {
    pub commit_id: CommitId,
    pub x: u16,
    pub y: u16,
    pub selected_index: usize,
    pub actions: Vec<(String, Action)>,
}

impl ContextMenuState {
    #[must_use]
    pub fn calculate_rect(&self, terminal_area: Rect) -> Rect {
        let longest_action = self
            .actions
            .iter()
            .map(|(name, _)| name.len())
            .max()
            .unwrap_or(0) as u16;
        let menu_width = (longest_action + 6).min(terminal_area.width);
        let menu_height = (self.actions.len() as u16 + 2).min(terminal_area.height);

        let mut x = self.x;
        let mut y = self.y;

        if x + menu_width > terminal_area.width {
            x = terminal_area.width.saturating_sub(menu_width);
        }

        if y + menu_height > terminal_area.height {
            y = y.saturating_sub(menu_height);
        }

        Rect::new(x, y, menu_width, menu_height)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CommandPaletteState {
    pub query: String,
    pub matches: Vec<usize>, // Indices into predefined command list
    pub selected_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeSelectionState {
    pub selected_index: usize,
    pub themes: Vec<crate::theme::PaletteType>,
}

impl Default for ThemeSelectionState {
    fn default() -> Self {
        Self {
            selected_index: 0,
            themes: crate::theme::PaletteType::all().to_vec(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EvologState {
    pub content: Vec<String>,
    pub scroll: u16,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OperationLogState {
    pub content: Vec<String>,
    pub scroll: u16,
}

#[derive(Default)]
pub struct AppTextArea<'a>(pub TextArea<'a>);

impl Clone for AppTextArea<'_> {
    fn clone(&self) -> Self {
        let mut area = TextArea::new(self.0.lines().to_vec());
        let (row, col) = self.0.cursor();
        area.move_cursor(CursorMove::Jump(row as u16, col as u16));
        Self(area)
    }
}

impl std::fmt::Debug for AppTextArea<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppTextArea")
            .field("lines", &self.0.lines())
            .field("cursor", &self.0.cursor())
            .finish()
    }
}

impl PartialEq for AppTextArea<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0.lines() == other.0.lines() && self.0.cursor() == other.0.cursor()
    }
}

impl<'a> Deref for AppTextArea<'a> {
    type Target = TextArea<'a>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AppTextArea<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Widget for &AppTextArea<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Widget::render(&self.0, area, buf);
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LogState {
    pub list_state: TableState,
    pub selected_file_index: Option<usize>,
    pub current_diff: Option<String>,
    pub is_loading_diff: bool,
    pub diff_scroll: u16,
    pub diff_cache: HashMap<CommitId, String>,
    pub selected_ids: std::collections::HashSet<CommitId>,
}

impl LogState {
    #[must_use]
    pub fn is_selected(&self, id: &CommitId) -> bool {
        self.selected_ids.contains(id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputState<'a> {
    pub text_area: AppTextArea<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeaderState {
    pub repo_text: String,
    pub branch_text: String,
    pub stats_text: String,
    pub wc_text: String,
    pub op_text: String,
}

impl Default for HeaderState {
    fn default() -> Self {
        Self {
            repo_text: " no repo ".to_string(),
            branch_text: " (detached) ".to_string(),
            stats_text: String::new(),
            wc_text: " Loading... ".to_string(),
            op_text: " OP: ........ ".to_string(),
        }
    }
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

pub struct RevsetEntry {
    pub name: &'static str,
    pub description: &'static str,
}

pub struct RevsetCategory {
    pub name: &'static str,
    pub entries: Vec<RevsetEntry>,
}

#[must_use]
pub fn get_revset_reference() -> Vec<RevsetCategory> {
    vec![
        RevsetCategory {
            name: "Operators",
            entries: vec![
                RevsetEntry {
                    name: "x-",
                    description: "Parents of x",
                },
                RevsetEntry {
                    name: "x+",
                    description: "Children of x",
                },
                RevsetEntry {
                    name: "::x",
                    description: "Ancestors of x",
                },
                RevsetEntry {
                    name: "x::",
                    description: "Descendants of x",
                },
                RevsetEntry {
                    name: "x::y",
                    description: "Descendants of x that are ancestors of y",
                },
                RevsetEntry {
                    name: "x..y",
                    description: "Ancestors of y not ancestors of x",
                },
                RevsetEntry {
                    name: "x & y",
                    description: "Intersection (both)",
                },
                RevsetEntry {
                    name: "x | y",
                    description: "Union (either)",
                },
                RevsetEntry {
                    name: "~x",
                    description: "Complement (not in x)",
                },
                RevsetEntry {
                    name: "x ~ y",
                    description: "Difference (x but not y)",
                },
            ],
        },
        RevsetCategory {
            name: "Scope & Identity",
            entries: vec![
                RevsetEntry {
                    name: "all()",
                    description: "All visible commits",
                },
                RevsetEntry {
                    name: "none()",
                    description: "Empty set",
                },
                RevsetEntry {
                    name: "root()",
                    description: "Oldest ancestor of all commits",
                },
                RevsetEntry {
                    name: "@",
                    description: "Working copy commit",
                },
                RevsetEntry {
                    name: "mine()",
                    description: "Your authored commits",
                },
                RevsetEntry {
                    name: "trunk()",
                    description: "Default bookmark on remote",
                },
                RevsetEntry {
                    name: "mutable()",
                    description: "Mutable commits",
                },
                RevsetEntry {
                    name: "immutable()",
                    description: "Immutable commits",
                },
                RevsetEntry {
                    name: "working_copies()",
                    description: "Working copies across workspaces",
                },
                RevsetEntry {
                    name: "visible_heads()",
                    description: "All visible head commits",
                },
            ],
        },
        RevsetCategory {
            name: "Bookmarks & Tags",
            entries: vec![
                RevsetEntry {
                    name: "bookmarks([p])",
                    description: "Local bookmark targets",
                },
                RevsetEntry {
                    name: "remote_bookmarks()",
                    description: "Remote bookmark targets",
                },
                RevsetEntry {
                    name: "tracked_remote_bookmarks()",
                    description: "Tracked remote bookmarks",
                },
                RevsetEntry {
                    name: "untracked_remote_bookmarks()",
                    description: "Untracked remote bookmarks",
                },
                RevsetEntry {
                    name: "tags([p])",
                    description: "Tag targets",
                },
                RevsetEntry {
                    name: "remote_tags()",
                    description: "Remote tag targets",
                },
            ],
        },
        RevsetCategory {
            name: "Ancestry & DAG",
            entries: vec![
                RevsetEntry {
                    name: "parents(x, [d])",
                    description: "Parents of x (optional depth)",
                },
                RevsetEntry {
                    name: "children(x, [d])",
                    description: "Children of x (optional depth)",
                },
                RevsetEntry {
                    name: "ancestors(x, [d])",
                    description: "Ancestors of x",
                },
                RevsetEntry {
                    name: "descendants(x, [d])",
                    description: "Descendants of x",
                },
                RevsetEntry {
                    name: "heads(x)",
                    description: "Commits with no descendants in x",
                },
                RevsetEntry {
                    name: "roots(x)",
                    description: "Commits with no ancestors in x",
                },
                RevsetEntry {
                    name: "connected(x)",
                    description: "x::x – fill in gaps",
                },
                RevsetEntry {
                    name: "reachable(s, d)",
                    description: "Reachable from s within domain d",
                },
                RevsetEntry {
                    name: "fork_point(x)",
                    description: "Common ancestor(s) of x",
                },
                RevsetEntry {
                    name: "first_parent(x)",
                    description: "First parent only (for merges)",
                },
                RevsetEntry {
                    name: "first_ancestors(x)",
                    description: "Ancestors via first parent only",
                },
            ],
        },
        RevsetCategory {
            name: "Search & Metadata",
            entries: vec![
                RevsetEntry {
                    name: "description(p)",
                    description: "Match commit description",
                },
                RevsetEntry {
                    name: "subject(p)",
                    description: "Match first line of description",
                },
                RevsetEntry {
                    name: "author(p)",
                    description: "Match author name or email",
                },
                RevsetEntry {
                    name: "author_date(p)",
                    description: "Match author date",
                },
                RevsetEntry {
                    name: "committer(p)",
                    description: "Match committer name or email",
                },
                RevsetEntry {
                    name: "committer_date(p)",
                    description: "Match committer date",
                },
                RevsetEntry {
                    name: "files(expr)",
                    description: "Commits modifying matching paths",
                },
                RevsetEntry {
                    name: "diff_lines(t, [f])",
                    description: "Commits with matching diff text",
                },
            ],
        },
        RevsetCategory {
            name: "State & Filters",
            entries: vec![
                RevsetEntry {
                    name: "conflicts()",
                    description: "Commits with conflicts",
                },
                RevsetEntry {
                    name: "divergent()",
                    description: "Divergent commits",
                },
                RevsetEntry {
                    name: "empty()",
                    description: "Commits modifying no files",
                },
                RevsetEntry {
                    name: "merges()",
                    description: "Merge commits",
                },
                RevsetEntry {
                    name: "signed()",
                    description: "Cryptographically signed",
                },
                RevsetEntry {
                    name: "latest(x, [n])",
                    description: "Latest n commits by date",
                },
                RevsetEntry {
                    name: "present(x)",
                    description: "x, or none() if missing",
                },
                RevsetEntry {
                    name: "exactly(x, n)",
                    description: "x if exactly n commits",
                },
            ],
        },
        RevsetCategory {
            name: "String Patterns",
            entries: vec![
                RevsetEntry {
                    name: "exact:\"str\"",
                    description: "Exact string match",
                },
                RevsetEntry {
                    name: "glob:\"pat\"",
                    description: "Unix-style wildcard (default)",
                },
                RevsetEntry {
                    name: "regex:\"pat\"",
                    description: "Regular expression match",
                },
                RevsetEntry {
                    name: "substring:\"str\"",
                    description: "Substring match",
                },
                RevsetEntry {
                    name: "-i suffix",
                    description: "Case-insensitive (e.g. glob-i:)",
                },
            ],
        },
        RevsetCategory {
            name: "Date Patterns",
            entries: vec![
                RevsetEntry {
                    name: "after:\"date\"",
                    description: "At or after the given date",
                },
                RevsetEntry {
                    name: "before:\"date\"",
                    description: "Before the given date",
                },
                RevsetEntry {
                    name: "\"2 days ago\"",
                    description: "Relative date example",
                },
                RevsetEntry {
                    name: "\"yesterday 5pm\"",
                    description: "Natural language date",
                },
            ],
        },
    ]
}
