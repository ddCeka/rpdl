use {
    crate::{downloader::Downloader, progress::ProgressConfig, task::DownloadTask},
    color_eyre::Result,
    colored::*,
    std::{
        path::PathBuf,
        time::{Duration, Instant},
    },
};

pub async fn handle(
    urls: Vec<String>,
    output_dir: Option<PathBuf>,
    max_concurrent: usize,
    unordered: bool,
    show_progress: bool,
    timeout: Duration,
    user_agent: Option<String>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("{}", "Starting multi-file download...".cyan());
        println!("  Files: {}", urls.len());
        println!("  Max concurrent: {}", max_concurrent);
        println!(
            "  Mode: {}",
            if unordered { "unordered" } else { "ordered" }
        );
        println!();
    }

    let output_dir = output_dir.unwrap_or_else(|| PathBuf::from("."));
    tokio::fs::create_dir_all(&output_dir).await?;

    let mut builder = Downloader::builder()
        .with_timeout(timeout)
        .with_progress_config(ProgressConfig::new().with_enabled(show_progress));

    if let Some(ua) = user_agent {
        builder = builder.with_user_agent(Some(ua));
    }

    let downloader = builder.build()?;

    let tasks: Vec<DownloadTask> = urls
        .into_iter()
        .enumerate()
        .map(|(idx, url)| {
            let filename = url
                .split('/')
                .next_back()
                .unwrap_or(&format!("file_{}", idx))
                .to_string();
            DownloadTask::new(url).with_id(filename)
        })
        .collect();

    let start = Instant::now();
    let results = if unordered {
        downloader.download_unordered(tasks).await
    } else {
        downloader.download(tasks).await
    };
    let elapsed = start.elapsed();

    let successful = results.iter().filter(|r| r.is_ok()).count();
    let failed = results.len() - successful;

    for (idx, result) in results.iter().enumerate() {
        match result {
            Ok(download) => {
                let filename = download.id.as_ref().unwrap();
                let output_path = output_dir.join(filename);
                tokio::fs::write(&output_path, &download.data).await?;

                if verbose {
                    println!(
                        "  {} {} ({} bytes)",
                        "✓".green(),
                        filename,
                        download.data.len()
                    );
                }
            }
            Err(e) => {
                eprintln!("  {} Download {} failed: {}", "✗".red(), idx, e);
            }
        }
    }

    println!("\n{}", "Downloads completed!".green().bold());
    println!("  Successful: {}", successful.to_string().green());
    if failed > 0 {
        println!("  Failed: {}", failed.to_string().red());
    }
    println!("  Time: {:?}", elapsed);

    Ok(())
}
