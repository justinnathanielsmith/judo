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
    SnapshotWorkingCopy,                   // `jj snapshot`
    EditRevision(CommitId),                // `jj edit <rev>`
    SquashRevision(CommitId),              // `jj squash -r <rev>`
    NewRevision(CommitId),                 // `jj new <rev>` (create child)
    DescribeRevisionIntent,                // Start describing the selected revision
    DescribeRevision(CommitId, String),    // `jj describe <rev> -m "msg"`
    AbandonRevision(CommitId),             // `jj abandon <rev>`
    Absorb,                                // `jj absorb`
    DuplicateRevision(CommitId),           // `jj duplicate -r <rev>`
    RebaseRevisionIntent,                  // Start rebase (open destination input)
    RebaseRevision(Vec<CommitId>, String), // `jj rebase -r <revs> -d <dest>`
    SetBookmarkIntent,                     // Start setting a bookmark
    SetBookmark(CommitId, String),         // `jj bookmark set <name> -r <rev>`
    DeleteBookmarkIntent,                  // Start deleting a bookmark (may prompt)
    DeleteBookmark(String),                // `jj bookmark delete <name>`
    SplitRevision(CommitId),               // `jj split -r <rev>`
    Undo,                                  // `jj undo`
    Redo,                                  // `jj redo`
    Fetch,                                 // `jj git fetch`
    PushIntent,                            // Trigger push (may prompt)
    Push(Option<String>),                  // `jj git push [-b <bookmark>]`
    ResolveConflict(String),               // `jj resolve --tool ... <path>`
    LoadMoreGraph,                         // Trigger pagination
    InitRepo,                              // `jj git init --colocate`

    // --- UI Mode Transitions ---
    EnterCommandMode,                          // Open command palette (:)
    EnterFilterMode,                           // Open filter bar (/)
    ApplyFilter(String),                       // Apply a revset filter
    FilterMine,                                // Quick filter: mine()
    FilterTrunk,                               // Quick filter: trunk()
    FilterConflicts,                           // Quick filter: conflicts()
    FilterAll,                                 // Quick filter: all()
    FilterHeads,                               // Quick filter: heads(all())
    FilterBookmarks,                           // Quick filter: bookmarks()
    FilterImmutable,                           // Quick filter: immutable()
    FilterMutable,                             // Quick filter: mutable()
    FilterEmpty,                               // Quick filter: empty()
    FilterDivergent,                           // Quick filter: divergent()
    FilterMerges,                              // Quick filter: merges()
    FilterTags,                                // Quick filter: tags()
    FilterRemoteBookmarks,                     // Quick filter: remote_bookmarks()
    FilterWorking,                             // Quick filter: working_copies()
    ClearFilter,                               // Clear the active revset filter
    FilterNext,                                // Next recent filter
    FilterPrev,                                // Previous recent filter
    ToggleFilterSource,                        // Toggle between recent and preset filters
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
    RepoLoaded(Box<RepoStatus>),             // Fresh graph data arrived
    RepoReloadedBackground(Box<RepoStatus>), // Background refresh data arrived
    GraphBatchLoaded(Box<RepoStatus>),       // Additional graph data arrived
    DiffLoaded(CommitId, String),            // Diff content for the selected commit
    OperationStarted(String),                // "Squashing..." (sets loading state)
    OperationCompleted(Result<String, String>), // Success/Failure message
    ErrorOccurred(String),                   // General error reporting
    ExternalChangeDetected,                  // External change to the repo (jj op heads)

    // --- Evolog ---
    EvologRevision(CommitId), // Trigger `jj evolog -r <rev>`
    OpenEvolog(String),       // Open evolog modal with content
    CloseEvolog,              // Close evolog modal
    ScrollEvologUp(u16),      // Scroll evolog up
    ScrollEvologDown(u16),    // Scroll evolog down

    // --- Operation Log ---
    OperationLog,                // Trigger `jj op log`
    OpenOperationLog(String),    // Open operation log modal with content
    CloseOperationLog,           // Close operation log modal
    ScrollOperationLogUp(u16),   // Scroll operation log up
    ScrollOperationLogDown(u16), // Scroll operation log down
}
