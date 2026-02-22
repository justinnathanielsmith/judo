use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(Default)]
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
    pub parent_min: usize,
    pub parent_max: usize,
    pub continuing_lanes: Vec<(usize, usize)>, // (from_lane, to_lane)
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GraphRow {
    pub commit_id: CommitId,
    pub commit_id_short: String,
    pub change_id: String,
    pub change_id_short: String,
    pub description: String, // Full description now
    pub author: String,
    pub timestamp: String,
    pub timestamp_secs: i64,
    pub is_working_copy: bool,
    pub is_immutable: bool,
    pub parents: Vec<CommitId>,
    pub bookmarks: Vec<String>,
    pub changed_files: Vec<FileChange>,
    pub visual: GraphRowVisual,
}


#[derive(Debug, Clone, PartialEq)]
pub struct RepoStatus {
    pub repo_name: String,
    pub operation_id: String,
    pub workspace_id: String,
    pub working_copy_id: CommitId,
    pub graph: Vec<GraphRow>,
}
