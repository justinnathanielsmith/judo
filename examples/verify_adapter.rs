use judo::infrastructure::jj_adapter::JjAdapter;
use judo::domain::vcs::VcsFacade;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Initializing JjAdapter...");
    let adapter = JjAdapter::new()?;
    println!("Adapter initialized.");

    println!("Fetching operation log...");
    let log = adapter.get_operation_log().await?;
    println!("Operation ID: {}", log.operation_id);
    println!("Working Copy ID: {}", log.working_copy_id);
    println!("Graph has {} entries.", log.graph.len());
    
    for entry in &log.graph {
        println!("Checking diff for {} - {}", entry.commit_id, entry.description);
        let diff = adapter.get_commit_diff(&entry.commit_id).await?;
        if diff != "(No changes or diff not implemented)" && !diff.contains("Root commit") {
            println!("SUCCESS: Found non-empty diff!");
            println!("Diff length: {}", diff.len());
            println!("Diff preview: {:.200}", diff);
            break;
        }
    }

    Ok(())
}
