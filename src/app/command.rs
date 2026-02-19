use crate::domain::models::CommitId;

#[derive(Debug, Clone)]
pub enum Command {
    LoadRepo,
    LoadDiff(CommitId),
    DescribeRevision(CommitId, String),
    // ... we can add more as we implement them
}
