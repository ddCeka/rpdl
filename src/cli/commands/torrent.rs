use {
    crate::{cli::utils::humanize_bytes, error::Result, torrent::TorrentDownloader},
    colored::Colorize,
    std::path::PathBuf,
};

pub async fn handle(
    source: &str,
    output_dir: Option<PathBuf>,
    select: Option<String>,
    list_only: bool,
    show_progress: bool,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("{}", "Starting torrent download...".cyan());
        println!("  Source: {}", source);
        if let Some(ref dir) = output_dir {
            println!("  Output: {}", dir.display());
        }
        println!();
    }

    let download_dir = output_dir.unwrap_or_else(|| PathBuf::from("."));
    tokio::fs::create_dir_all(&download_dir).await?;

    let downloader = crate::torrent::TorrentDownloader::new(download_dir.clone()).await?;

    downloader
        .download_from_url(source, Some(download_dir), list_only, select, show_progress)
        .await?;

    Ok(())
}

pub async fn handle_info(source: &str, verbose: bool) -> Result<()> {
    let downloader = TorrentDownloader::new(PathBuf::from(".")).await?;
    let info = downloader.get_info(source).await?;

    println!("\n{}", "Torrent Information:".cyan().bold());
    println!("  Name: {}", info.name);
    println!("  Total size: {}", humanize_bytes(info.total_size));
    println!("  Files: {}", info.files.len());

    if verbose {
        println!("\n{}", "Files:".yellow());
        for (idx, file) in info.files.iter().enumerate() {
            println!("  [{}] {} ({})", idx, file.name, humanize_bytes(file.size));
        }
    }

    Ok(())
}
