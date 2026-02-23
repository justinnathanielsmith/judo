# AGENTS.md

This file guides AI agents working on the Judo codebase. Follow these instructions to ensure consistency, stability, and high performance.

## Mission

Build a high-performance, idiomatic Rust TUI for [Jujutsu (jj)](https://github.com/martinvonz/jj). The goal is to provide a fast, intuitive interface for version control operations.

## Tech Stack

* **Language**: Rust (2021 edition)
* **TUI Framework**: [ratatui](https://github.com/ratatui-org/ratatui) (latest stable)
* **Async Runtime**: [tokio](https://tokio.rs/)
* **VCS Backend**: `jj-lib` (Jujutsu library)
* **Error Handling**: `anyhow` for app-level errors

## Architecture: The Elm Architecture (TEA)

We strictly follow The Elm Architecture pattern to manage state and side effects.

### 1. Model (`AppState`)

* Located in `src/app/state/mod.rs` (modular structure).
* Must remain the **Single Source of Truth** for the entire application.
* Modularized into sub-modules: `command_palette`, `context_menu`, `error`, `extra`, `header`, `input`, `log`, `revset`, `theme`.
* Contains UI state (selection, mode) and Domain state (RepoStatus).

### 2. Update (`reducer`)

* Located in `src/app/reducer.rs`.
* Handles `Action`s to modify `AppState`.
* **Feature Delegation**: The main reducer delegates to feature-specific reducers in `src/app/features/`.
  * E.g., `src/app/features/vcs/actions.rs`, `src/app/features/navigation.rs`.
* **Pure Functions**: Reducers must be pure; logic should only calculate the next state from the current state and an action.
* **Side Effects**: Async operations or commands (like jj operations) must return an `Action` to be processed by the reducer rather than mutating state directly inside an async block.

### 3. View (`ui`)

* Located in `src/app/ui.rs` or component modules.
* **Declarative**: The UI must be rendered *solely* based on the current `AppState`.
* **Logic-Free**: No business or domain logic is permitted in the view layer.

### 4. External Tools & TUI Suspension

* **jj resolve / jj split**: Some operations require user interaction in the terminal.
* The `run_loop` in `src/app/loop.rs` handles this by:
  1. Suspending the TUI (disabling raw mode, leaving alternate screen).
  2. Spawning the external `jj` command.
  3. Resuming the TUI after the command exits.
  4. Dispatching an `Action::OperationCompleted` to trigger a state refresh.

## Coding Standards

### Rust Idioms & Performance

* **Error Handling**: Use `Result<T, anyhow::Error>` for all fallible functions in the app layer.
* **Panic Prevention**: Do not use `unwrap()` unless it is mathematically or logically impossible to fail. You must document the reasoning for any `unwrap()` used.
* **Responsive UI**: Use `tokio::task::spawn_blocking` for any heavy computation or blocking I/O, particularly for `jj-lib` calls, to keep the UI thread responsive.
* **Clippy**: Ensure code is clippy-clean (`cargo clippy`).

### File Structure

* `src/domain/`: Core business logic and VCS abstractions.
* `src/infrastructure/`: Adapters for external systems like `jj-lib`.
* `src/app/`: Application-specific logic (state, reducer, loop, keymap, recovery).
* `src/components/`: Reusable UI widgets (header, footer, revision graph, diff view, modals).

## Workflow

1. **Understand First**: Read `AppState` and related `Action`s before modifying code.
2. **Atomic Changes**: Keep changes focused on exactly one feature or fix per task.
3. **Verify**:
    * Run `cargo check` to catch compilation errors.
    * Run `cargo test` to ensure no regressions.
    * **Graph Layout**: Verify that any repo-load or reload action calls `graph_layout::calculate_graph_layout` to ensure the revision graph is rendered correctly.
    * **Modularity**: When refactoring, ensure no duplicate module definitions (`mod.rs` vs `filename.rs`) and use `cargo check` to verify type inference after moving code to modular sub-packages.
    * Verify that the `Esc` key correctly cancels current modes and clears errors in the UI.

## Revset Filter System

The revset filter system provides comprehensive support for jj's revset language.

### State (`state.rs`)
* `revset: Option<String>` — the currently active revset expression.
* `recent_filters: Vec<String>` — persisted history of user-entered filters.
* `preset_filters: Vec<String>` — 21 built-in presets covering scope, bookmarks/tags, state, and DAG.
* `is_selecting_presets: bool` — toggles between recent and preset list sources.
* `get_revset_reference()` — returns categorized reference data (8 categories, 70+ entries) used by the filter modal.

### Actions & Reducer
* Preset filter actions (`FilterMine`, `FilterTrunk`, etc.) set `state.revset` and dispatch `LoadRepo`.
* `ApplyFilter(String)` handles custom revset input, manages recent filter history.
* `ClearFilter` resets `state.revset` to `None`.

### Error Handling
* **Auto-revert**: When `ErrorOccurred` fires with a revset-related error AND a filter is active, the reducer auto-clears `state.revset` and dispatches `LoadRepo(None)` to prevent infinite error loops.
* **Recovery suggestions** (`recovery.rs`): Pattern-matches revset errors (keywords: `revset`, `parse error`, `function`, `invalid expression`) and suggests corrective actions.

## Conflict Resolution

Judo supports resolving merge conflicts directly from the TUI.

* **Detection**: Conflicted files are detected in `RepoStatus` and flagged with `FileStatus::Conflicted`.
* **Visualization**: Conflicts appear in the diff view with a `!` prefix and distinct styling.
* **Resolution**: The `m` key (or `Enter`) on a conflicted file triggers `jj resolve`, which suspends the TUI to open the user's configured merge tool.
* **Safety**: Judo blocks `Commit` operations if the working copy contains unresolved conflict markers.

## VCS Safety & Integrity

* **Operation Guards**: Actions like `CommitWorkingCopyIntent` must check the current `RepoStatus` for conflicts or illegal states before transitioning to an input mode.
* **Error Severity**: `ErrorState` includes `ErrorSeverity` (Warning, Error, Critical) to guide UI presentation and recovery logic.
* **Path Validation**: When executing external tools (like `jj resolve`), always validate file paths to prevent traversal vulnerabilities.

## Lessons Learned

### Clippy & Formatting
* **Sequential Tools**: Always run `cargo fmt` after `cargo clippy --fix`. Clippy can introduce formatting artifacts (like trailing whitespace or long lines) that will fail strict CI formatting checks.
* **Async Cleanup**: Be mindful of Clippy's `unused_async` lint. Removing `async` from functions that spawn background tasks (rather than awaiting them) requires surgical updates to all call sites and tests to remove unnecessary `.await` calls.

### Modularity & Refactoring
* **Duplicate Modules**: Rust forbids having both `src/module.rs` and `src/module/mod.rs`. This causes `E0761`. When modularizing, always delete the monolithic file after moving its contents to the new directory structure.
* **Type Inference**: Moving code between modules can break type inference (E0282), especially for generic results or TEA `UpdateResult`. Explicitly annotate types if the compiler becomes ambiguous during a refactor.

### Version Control Workflow
* **Jujutsu (jj) Integration**: This project uses `jj` as the primary VCS.
  * Use `jj describe -m "message"` to update commit descriptions.
  * Use `jj bookmark set main -r @` to move the `main` branch to the current working copy.
  * Use `jj git push --branch main` to synchronize with the remote.
  * **Conflict Markers**: Avoid committing files that contain JJ/Git conflict markers (e.g., `<<<<<<< conflict`). Judo's automated tests and safety guards are designed to prevent this.
  * When tagging for releases, ensure the tag is placed on the immutable commit (after pushing or describing) and pushed to `origin` to trigger GitHub Actions.
