use crate::domain::models::{CommitId, RepoStatus};

#[derive(Debug, Clone)]
pub enum Action {
    // --- System / Terminal ---
    Tick,
    Render,
    Resize(u16, u16),
    Quit,

    // --- Navigation (Log View) ---
    SelectNext,
    SelectPrev,
    ScrollDiffUp(u16),
    ScrollDiffDown(u16),

    // --- JJ Domain Intents ---
    // These trigger async tasks
    SnapshotWorkingCopy,                // `jj snapshot`
    EditRevision(CommitId),             // `jj edit <rev>`
    SquashRevision(CommitId),           // `jj squash -r <rev>`
    NewRevision(CommitId),              // `jj new <rev>` (create child)
    DescribeRevisionIntent,             // Start describing the selected revision
    DescribeRevision(CommitId, String), // `jj describe <rev> -m "msg"`
    AbandonRevision(CommitId),          // `jj abandon <rev>`
    Undo,                               // `jj undo`
    Redo,                               // `jj redo`

    // --- UI Mode Transitions ---
    EnterCommandMode, // Open command palette (:)
    EnterSquashMode,  // Open squash selection modal
    CancelMode,       // ESC key (close modal/mode)

    // --- Async Results (The "Callback") ---
    // These are dispatched by your async workers back to the main thread
    RepoLoaded(Box<RepoStatus>),  // Fresh graph data arrived
    DiffLoaded(CommitId, String), // Diff content for the selected commit
    OperationStarted(String),     // "Squashing..." (sets loading state)
    OperationCompleted(Result<String, String>), // Success/Failure message
    ErrorOccurred(String),        // General error reporting
}
