use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommitId(pub String);

impl fmt::Display for CommitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Conflicted,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileChange {
    pub path: String,
    pub status: FileStatus,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GraphRowVisual {
    pub column: usize,
    pub active_lanes: Vec<bool>,
    pub connector_lanes: Vec<bool>,
    // New fields for advanced rendering
    pub parent_columns: Vec<usize>,
    pub continuing_lanes: Vec<(usize, usize)>, // (from_lane, to_lane)
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GraphRow {
    pub commit_id: CommitId,
    pub change_id: String,
    pub description: String, // Full description now
    pub author: String,
    pub timestamp: String,
    pub is_working_copy: bool,
    pub is_immutable: bool,
    pub parents: Vec<CommitId>,
    pub bookmarks: Vec<String>,
    pub changed_files: Vec<FileChange>,
    pub visual: GraphRowVisual,
}

impl Default for CommitId {
    fn default() -> Self {
        Self("".to_string())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepoStatus {
    pub repo_name: String,
    pub operation_id: String,
    pub workspace_id: String,
    pub working_copy_id: CommitId,
    pub graph: Vec<GraphRow>,
}
