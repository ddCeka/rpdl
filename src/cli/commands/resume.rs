use {
    crate::{
        chunked::{ChunkedDownload, ChunkedDownloadConfig},
        progress::ProgressConfig,
        resume::DownloadState,
    },
    color_eyre::Result,
    colored::*,
    std::{path::PathBuf, time::Instant},
};

pub async fn handle(file: PathBuf, force: bool, verbose: bool) -> Result<()> {
    if verbose {
        println!("{}", "Attempting to resume download...".cyan());
        println!("  File: {}", file.display());
    }

    let mut state = match DownloadState::load(&file).await? {
        Some(state) => state,
        None => {
            eprintln!("{}", "No resume state found for this file".red());
            eprintln!(
                "  Looking for: {}",
                DownloadState::state_file_path(&file).display()
            );
            return Ok(());
        }
    };

    if !force && !state.validate_integrity() {
        eprintln!("{}", "Resume state appears corrupted!".red());
        eprintln!("  Use --force to attempt resume anyway");
        return Ok(());
    }

    if state.is_complete() {
        println!("{}", "Download is already complete!".green());
        DownloadState::delete(&file).await?;
        return Ok(());
    }

    let progress = state.progress();
    let incomplete_chunks = state.get_incomplete_chunks();

    if verbose {
        println!("\n{}", "Resume Information:".yellow());
        println!("  URL: {}", state.url);
        println!("  Total size: {} bytes", state.total_size);
        println!("  Progress: {:.2}%", progress);
        println!("  Remaining chunks: {}", incomplete_chunks.len());
        println!();
    }

    println!(
        "{} Resuming download ({:.1}% complete, {} chunks remaining)...",
        "⟳".cyan(),
        progress,
        incomplete_chunks.len()
    );

    let config = ChunkedDownloadConfig::new()
        .chunk_size(state.chunk_size)
        .progress_config(ProgressConfig::new().with_enabled(true));

    let downloader = ChunkedDownload::new(config)?;

    let start = Instant::now();

    let output_path = state.output_path.clone();
    match downloader
        .resume(&mut state, output_path.to_str().unwrap())
        .await
    {
        Ok(_) => {
            let elapsed = start.elapsed();
            println!("\n{}", "Resume completed successfully!".green().bold());
            println!("  Time: {:?}", elapsed);
            println!("  File: {}", state.output_path.display());

            DownloadState::delete(&file).await?;
        }
        Err(e) => {
            eprintln!("\n{} {}", "Resume failed:".red(), e);
            println!(
                "{}",
                "State file preserved for future resume attempts".yellow()
            );
        }
    }

    Ok(())
}
