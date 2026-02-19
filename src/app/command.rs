use crate::domain::models::CommitId;

#[derive(Debug, Clone)]
pub enum Command {
    LoadRepo,
    LoadDiff(CommitId),
    DescribeRevision(CommitId, String),
    Snapshot,
    Edit(CommitId),
    Squash(CommitId),
    New(CommitId),
    Abandon(CommitId),
}
