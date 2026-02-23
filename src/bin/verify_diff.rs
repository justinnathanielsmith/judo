use anyhow::Result;
use judo::domain::vcs::VcsFacade;
use judo::infrastructure::JjAdapter;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Initializing JjAdapter...");
    let adapter = JjAdapter::new()?;

    println!("Fetching operation log...");
    let log = adapter.get_operation_log(None, 100, None).await?;
    let wc_id = &log.working_copy_id;
    println!("Working Copy ID: {wc_id}");

    // Also try to get the parent of the working copy to see a committed change diff
    // The graph is somewhat ordered, let's just picking the second one if available
    let commit_to_diff = if log.graph.len() > 1 {
        let parent = &log.graph[1];
        println!(
            "Diffing parent commit: {} ({})",
            parent.commit_id, parent.description
        );
        &parent.commit_id
    } else {
        println!("Diffing working copy commit");
        wc_id
    };

    println!("Computing diff...");
    let diff_output = adapter.get_commit_diff(commit_to_diff).await?;

    println!("--- Diff Output ---");
    println!("{diff_output}");
    println!("-------------------");

    Ok(())
}
