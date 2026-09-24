use {
    color_eyre::Result,
    colored::*,
    std::{path::PathBuf, time::Duration},
};

pub async fn handle(
    file: PathBuf,
    output_dir: Option<PathBuf>,
    max_concurrent: usize,
    show_progress: bool,
    timeout: Duration,
    verbose: bool,
) -> Result<()> {
    let content = tokio::fs::read_to_string(&file).await?;
    let urls: Vec<String> = content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| line.trim().to_string())
        .collect();

    if urls.is_empty() {
        eprintln!("{}", "No URLs found in file".red());
        return Ok(());
    }

    if verbose {
        println!("{}", "Starting batch download...".cyan());
        println!("  Source: {}", file.display());
        println!("  URLs found: {}", urls.len());
        println!();
    }

    super::multi::handle(
        urls,
        output_dir,
        max_concurrent,
        false,
        show_progress,
        timeout,
        None,
        verbose,
    )
    .await
}
