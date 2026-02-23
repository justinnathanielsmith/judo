use crate::app::{action::Action, command::Command};
use crate::domain::vcs::VcsFacade;
use anyhow::Result;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::mpsc;

pub fn handle_command(
    command: Command,
    adapter: Arc<dyn VcsFacade>,
    tx: mpsc::Sender<Action>,
) -> Result<()> {
    match command {
        Command::LoadRepoBackground(limit, revset) => {
            tokio::spawn(async move {
                match adapter.get_operation_log(None, limit, revset).await {
                    Ok(repo) => {
                        let _ = tx
                            .send(Action::RepoReloadedBackground(Box::new(repo)))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::ErrorOccurred(format!(
                                "Background sync failed: {e}"
                            )))
                            .await;
                    }
                }
            });
        }
        Command::LoadRepo(heads, limit, revset) => {
            let is_batch = heads.is_some();
            tokio::spawn(async move {
                match adapter.get_operation_log(heads, limit, revset).await {
                    Ok(repo) => {
                        if is_batch {
                            let _ = tx.send(Action::GraphBatchLoaded(Box::new(repo))).await;
                        } else {
                            let _ = tx.send(Action::RepoLoaded(Box::new(repo))).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::ErrorOccurred(format!("Failed to load repo: {e}")))
                            .await;
                    }
                }
            });
        }
        Command::LoadDiff(commit_id) => {
            let commit_id_clone = commit_id.clone();
            tokio::spawn(async move {
                match adapter.get_commit_diff(&commit_id).await {
                    Ok(diff) => {
                        let _ = tx.send(Action::DiffLoaded(commit_id_clone, diff)).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::DiffLoaded(commit_id_clone, format!("Error: {e}")))
                            .await;
                    }
                }
            });
        }
        Command::DescribeRevision(commit_id, message) => {
            tokio::spawn(async move {
                run_operation(
                    tx,
                    format!("Describing {commit_id}..."),
                    "Described",
                    move || async move { adapter.describe_revision(&commit_id.0, &message).await },
                )
                .await;
            });
        }
        Command::Commit(message) => {
            tokio::spawn(async move {
                run_operation(
                    tx,
                    "Committing...".to_string(),
                    "Committed",
                    move || async move { adapter.commit(&message).await },
                )
                .await;
            });
        }
        Command::Snapshot => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted("Snapshotting...".to_string()))
                    .await;
                match adapter.snapshot().await {
                    Ok(msg) => {
                        let _ = tx.send(Action::OperationCompleted(Ok(msg))).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {e}"))))
                            .await;
                    }
                }
            });
        }
        Command::Edit(commit_id) => {
            tokio::spawn(async move {
                run_operation(
                    tx,
                    format!("Editing {commit_id}..."),
                    "Edit successful",
                    move || async move { adapter.edit(&commit_id).await },
                )
                .await;
            });
        }
        Command::Squash(commit_ids) => {
            tokio::spawn(async move {
                let msg = if commit_ids.len() == 1 {
                    format!("Squashing {}...", commit_ids[0])
                } else {
                    format!("Squashing {} revisions...", commit_ids.len())
                };
                run_operation(tx, msg, "Squash successful", move || async move {
                    adapter.squash(&commit_ids).await
                })
                .await;
            });
        }
        Command::New(commit_id) => {
            tokio::spawn(async move {
                run_operation(
                    tx,
                    format!("Creating child of {commit_id}..."),
                    "New revision created",
                    move || async move { adapter.new_child(&commit_id).await },
                )
                .await;
            });
        }
        Command::Absorb => {
            tokio::spawn(async move {
                run_operation(
                    tx,
                    "Absorbing changes...".to_string(),
                    "Absorb successful",
                    move || async move { adapter.absorb().await },
                )
                .await;
            });
        }
        Command::Duplicate(commit_ids) => {
            tokio::spawn(async move {
                let msg = if commit_ids.len() == 1 {
                    format!("Duplicating {}...", commit_ids[0])
                } else {
                    format!("Duplicating {} revisions...", commit_ids.len())
                };
                run_operation(tx, msg, "Revision(s) duplicated", move || async move {
                    adapter.duplicate(&commit_ids).await
                })
                .await;
            });
        }
        Command::Rebase(commit_ids, destination) => {
            tokio::spawn(async move {
                let msg = if commit_ids.len() == 1 {
                    format!("Rebasing {} onto {}...", commit_ids[0], destination)
                } else {
                    format!(
                        "Rebasing {} revisions onto {}...",
                        commit_ids.len(),
                        destination
                    )
                };
                run_operation(tx, msg, "Rebase successful", move || async move {
                    adapter.rebase(&commit_ids, &destination).await
                })
                .await;
            });
        }
        Command::Parallelize(commit_ids) => {
            tokio::spawn(async move {
                let msg = if commit_ids.len() == 1 {
                    format!("Parallelizing {}...", commit_ids[0])
                } else {
                    format!("Parallelizing {} revisions...", commit_ids.len())
                };
                run_operation(tx, msg, "Revision(s) parallelized", move || async move {
                    adapter.parallelize(&commit_ids).await
                })
                .await;
            });
        }
        Command::Abandon(commit_ids) => {
            tokio::spawn(async move {
                let msg = if commit_ids.len() == 1 {
                    format!("Abandoning {}...", commit_ids[0])
                } else {
                    format!("Abandoning {} revisions...", commit_ids.len())
                };
                run_operation(tx, msg, "Revision(s) abandoned", move || async move {
                    adapter.abandon(&commit_ids).await
                })
                .await;
            });
        }
        Command::Revert(commit_ids) => {
            tokio::spawn(async move {
                let msg = if commit_ids.len() == 1 {
                    format!("Reverting {}...", commit_ids[0])
                } else {
                    format!("Reverting {} revisions...", commit_ids.len())
                };
                run_operation(tx, msg, "Revision(s) reverted", move || async move {
                    adapter.revert(&commit_ids).await
                })
                .await;
            });
        }
        Command::Split(_commit_id) => {
            // Handled directly in run_loop because it requires suspending TUI
        }
        Command::SetBookmark(commit_id, name) => {
            let name_clone = name.clone();
            tokio::spawn(async move {
                run_operation(
                    tx,
                    format!("Setting bookmark {name_clone}..."),
                    "Bookmark set",
                    move || async move { adapter.set_bookmark(&commit_id, &name_clone).await },
                )
                .await;
            });
        }
        Command::DeleteBookmark(name) => {
            let name_clone = name.clone();
            tokio::spawn(async move {
                run_operation(
                    tx,
                    format!("Deleting bookmark {name_clone}..."),
                    "Bookmark deleted",
                    move || async move { adapter.delete_bookmark(&name_clone).await },
                )
                .await;
            });
        }
        Command::Undo => {
            tokio::spawn(async move {
                run_operation(
                    tx,
                    "Undoing...".to_string(),
                    "Undo successful",
                    move || async move { adapter.undo().await },
                )
                .await;
            });
        }
        Command::Redo => {
            tokio::spawn(async move {
                run_operation(
                    tx,
                    "Redoing...".to_string(),
                    "Redo successful",
                    move || async move { adapter.redo().await },
                )
                .await;
            });
        }
        Command::Fetch => {
            tokio::spawn(async move {
                run_operation(
                    tx,
                    "Fetching...".to_string(),
                    "Fetch successful",
                    move || async move { adapter.fetch().await },
                )
                .await;
            });
        }
        Command::Push(bookmark_opt) => {
            let bookmark_clone = bookmark_opt.clone();
            tokio::spawn(async move {
                let msg = if let Some(ref b) = bookmark_clone {
                    format!("Pushing {b}...")
                } else {
                    "Pushing...".to_string()
                };
                run_operation(tx, msg, "Push successful", move || async move {
                    adapter.push(bookmark_clone).await
                })
                .await;
            });
        }
        Command::ResolveConflict(_) => {
            // Handled specially in run_loop to allow TUI suspension
        }
        Command::InitRepo => {
            tokio::spawn(async move {
                run_operation(
                    tx,
                    "Initializing repository...".to_string(),
                    "Repository initialized",
                    move || async move { adapter.init_repo().await },
                )
                .await;
            });
        }
        Command::Evolog(commit_id) => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted(format!(
                        "Fetching evolog for {}...",
                        commit_id.0
                    )))
                    .await;
                match adapter.evolog(&commit_id).await {
                    Ok(content) => {
                        let _ = tx.send(Action::OpenEvolog(content)).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {e}"))))
                            .await;
                    }
                }
            });
        }
        Command::OperationLog => {
            tokio::spawn(async move {
                let _ = tx
                    .send(Action::OperationStarted(
                        "Fetching operation log...".to_string(),
                    ))
                    .await;
                match adapter.operation_log().await {
                    Ok(content) => {
                        let _ = tx.send(Action::OpenOperationLog(content)).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::OperationCompleted(Err(format!("Error: {e}"))))
                            .await;
                    }
                }
            });
        }
    }
    Ok(())
}

async fn run_operation<F, Fut>(
    tx: mpsc::Sender<Action>,
    start_msg: String,
    success_msg: &'static str,
    action: F,
) where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<()>> + Send,
{
    let _ = tx.send(Action::OperationStarted(start_msg)).await;
    match action().await {
        Ok(()) => {
            let _ = tx
                .send(Action::OperationCompleted(Ok(success_msg.to_string())))
                .await;
        }
        Err(e) => {
            let _ = tx
                .send(Action::OperationCompleted(Err(format!("Error: {e}"))))
                .await;
        }
    }
}
