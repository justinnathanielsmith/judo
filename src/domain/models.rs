use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommitId(pub String);

impl fmt::Display for CommitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphRow {
    pub commit_id: CommitId,
    pub change_id: String,
    pub description: String, // Full description now
    pub author: String,
    pub timestamp: String,
    pub is_working_copy: bool,
    // For graph rendering: parents, children, etc.
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepoStatus {
    pub operation_id: String,
    pub working_copy_id: CommitId,
    pub graph: Vec<GraphRow>,
}
