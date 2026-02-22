use crate::domain::models::{CommitId, RepoStatus};

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    // --- System / Terminal ---
    Tick,
    Render,
    Resize(u16, u16),
    Quit,

    // --- Navigation (Log View) ---
    SelectNext,
    SelectPrev,
    SelectIndex(usize),
    SelectFile(usize),
    SelectFileByPath(String),
    SelectNextFile,
    SelectPrevFile,
    ScrollDiffUp(u16),
    ScrollDiffDown(u16),
    NextHunk,
    PrevHunk,
    ToggleDiffs,
    ToggleSelection(CommitId),
    ClearSelection,

    // --- JJ Domain Intents ---
    // These trigger async tasks
    SnapshotWorkingCopy,                // `jj snapshot`
    EditRevision(CommitId),             // `jj edit <rev>`
    SquashRevision(CommitId),           // `jj squash -r <rev>`
    NewRevision(CommitId),              // `jj new <rev>` (create child)
    DescribeRevisionIntent,             // Start describing the selected revision
    DescribeRevision(CommitId, String), // `jj describe <rev> -m "msg"`
    AbandonRevision(CommitId),          // `jj abandon <rev>`
    SetBookmarkIntent,                  // Start setting a bookmark
    SetBookmark(CommitId, String),      // `jj bookmark set <name> -r <rev>`
    DeleteBookmark(String),             // `jj bookmark delete <name>`
    Undo,                               // `jj undo`
    Redo,                               // `jj redo`
    Fetch,                              // `jj git fetch`
    PushIntent,                         // Trigger push (may prompt)
    Push(Option<String>),               // `jj git push [-b <bookmark>]`
    ResolveConflict(String),            // `jj resolve --tool ... <path>`
    LoadMoreGraph,                      // Trigger pagination
    InitRepo,                           // `jj git init --colocate`

    // --- UI Mode Transitions ---
    EnterCommandMode,                          // Open command palette (:)
    EnterFilterMode,                           // Open filter bar (/)
    ApplyFilter(String),                       // Apply a revset filter
    FilterMine,                                // Quick filter: mine()
    FilterTrunk,                               // Quick filter: trunk()
    FilterConflicts,                           // Quick filter: conflicts()
    EnterSquashMode,                           // Open squash selection modal
    FocusDiff,                                 // Switch focus to diff window
    FocusGraph,                                // Switch focus to revision graph
    CancelMode,                                // ESC key (close modal/mode)
    ToggleHelp,                                // Toggle the help overlay (?)
    EnterThemeSelection,                       // Open theme selection modal (T)
    SwitchTheme(crate::theme::PaletteType),    // Apply a new theme
    TextAreaInput(crossterm::event::KeyEvent), // Handle text area input
    OpenContextMenu(CommitId, (u16, u16)),     // Open menu at position
    SelectContextMenuAction(usize),            // Select action by index
    SelectContextMenuNext,                     // Next item in menu
    SelectContextMenuPrev,                     // Prev item in menu
    CloseContextMenu,                          // Close the menu
    CommandPaletteNext,                        // Next item in palette
    CommandPalettePrev,                        // Prev item in palette
    CommandPaletteSelect,                      // Execute selected command

    // --- Async Results (The "Callback") ---
    // These are dispatched by your async workers back to the main thread
    RepoLoaded(Box<RepoStatus>),       // Fresh graph data arrived
    GraphBatchLoaded(Box<RepoStatus>), // Additional graph data arrived
    DiffLoaded(CommitId, String),      // Diff content for the selected commit
    OperationStarted(String),          // "Squashing..." (sets loading state)
    OperationCompleted(Result<String, String>), // Success/Failure message
    ErrorOccurred(String),             // General error reporting
    ExternalChangeDetected,            // External change to the repo (jj op heads)
}
