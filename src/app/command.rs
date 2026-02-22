use crate::domain::models::CommitId;

#[derive(Debug, Clone)]
pub enum Command {
    LoadRepo(Option<Vec<CommitId>>, usize, Option<String>),
    LoadRepoBackground(usize, Option<String>),
    LoadDiff(CommitId),
    DescribeRevision(CommitId, String),
    Snapshot,
    Edit(CommitId),
    Squash(Vec<CommitId>),
    New(CommitId),
    Abandon(Vec<CommitId>),
    Absorb,
    Duplicate(Vec<CommitId>),
    Rebase(Vec<CommitId>, String),
    SetBookmark(CommitId, String),
    DeleteBookmark(String),
    Undo,
    Redo,
    Fetch,
    Push(Option<String>),
    ResolveConflict(String),
    InitRepo,
}
