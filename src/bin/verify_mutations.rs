use anyhow::Result;
use judo::domain::vcs::VcsFacade;
use judo::infrastructure::jj_adapter::JjAdapter;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Initializing JjAdapter...");
    let adapter = JjAdapter::new()?;

    println!("Testing snapshot...");
    let snap_res = adapter.snapshot().await?;
    println!("Snapshot result: {}", snap_res);

    println!("Fetching operation log for WC...");
    let log = adapter.get_operation_log(None, 100, None).await?;
    let wc_id = &log.working_copy_id;
    println!("Working Copy ID: {}", wc_id);

    // Find description
    let current_desc = log
        .graph
        .iter()
        .find(|r| r.commit_id == *wc_id)
        .map(|r| r.description.clone())
        .unwrap_or_default();

    println!("Current description: '{}'", current_desc);

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let new_desc = format!("{} [TEST {}]", current_desc, now);

    println!("Describing revision as: '{}'", new_desc);
    // Use the hex ID of the working copy commit
    adapter.describe_revision(&wc_id.0, &new_desc).await?;

    println!("Verifying change...");
    let log_new = adapter.get_operation_log(None, 100, None).await?;
    let new_wc_id = &log_new.working_copy_id;
    println!("New Working Copy ID: {}", new_wc_id);

    let new_entry_desc = log_new
        .graph
        .iter()
        .find(|r| r.commit_id == *new_wc_id)
        .map(|r| r.description.clone())
        .unwrap_or_default();

    println!("New description: '{}'", new_entry_desc);

    if new_entry_desc == new_desc {
        println!("SUCCESS: Description updated!");
    } else {
        println!(
            "FAILURE: Description mismatch! Expected '{}', got '{}'",
            new_desc, new_entry_desc
        );
    }

    Ok(())
}
