use crate::domain::models::CommitId;

#[derive(Debug, Clone)]
pub enum Command {
    LoadRepo(Option<Vec<CommitId>>, usize),
    LoadDiff(CommitId),
    DescribeRevision(CommitId, String),
    Snapshot,
    Edit(CommitId),
    Squash(CommitId),
    New(CommitId),
    Abandon(CommitId),
    SetBookmark(CommitId, String),
    DeleteBookmark(String),
    Undo,
    Redo,
    Fetch,
}
