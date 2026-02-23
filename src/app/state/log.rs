use crate::domain::models::CommitId;
use ratatui::widgets::TableState;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Panel {
    Graph,
    Diff,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LogState {
    pub list_state: TableState,
    pub selected_file_index: Option<usize>,
    pub current_diff: Option<String>,
    pub is_loading_diff: bool,
    pub diff_scroll: u16,
    pub diff_cache: HashMap<CommitId, String>,
    pub selected_ids: HashSet<CommitId>,
}

impl LogState {
    #[must_use]
    pub fn is_selected(&self, id: &CommitId) -> bool {
        self.selected_ids.contains(id)
    }
}
