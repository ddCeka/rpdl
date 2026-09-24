use {
    crate::{
        chunked::{ChunkedDownload, ChunkedDownloadConfig},
        cli::utils::{humanize_bytes, sanitize_filename},
        error::{DownloadError, Result},
        specialized::SpecializedDownloaderManager,
    },
    colored::Colorize,
    std::{
        path::{Path, PathBuf},
        time::Instant,
    },
};

pub fn list() {
    println!("\n{}", "Supported Specialized Sites:".cyan().bold());
    println!();

    println!("{}", "GitHub".green().bold());
    println!("  Description: Download release assets or source from GitHub repositories");
    println!("  Usage:");
    println!("    rpdl sp gh <owner>/<repo> -o <output>           # Download source (main branch)");
    println!("    rpdl sp gh <owner>/<repo>#<commit> -o <output>  # Download source at commit");
    println!("    rpdl sp gh <owner>/<repo> <tag> -o <output>     # Download all release assets");
    println!("    rpdl sp gh <owner>/<repo> <tag> <asset> -o <out> # Download specific asset");
    println!("  Example: rpdl sp gh rust-lang/rust v1.70.0 rustc -o rustc.tar.gz");
    println!("  Auth: Optional GITHUB_TOKEN for private repos or higher rate limits");
    println!();

    println!("{}", "Configuration:".yellow());
    println!("  Set credentials via environment variables or command-line flags");
    println!("  See 'rpdl specialized <site> --help' for site-specific options");
}

pub async fn handle_github(
    repo: &str,
    tag: Option<&str>,
    asset: Option<&str>,
    output: &Path,
    token: Option<String>,
    show_info: bool,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("{}", "Fetching GitHub release...".cyan());
        println!("  Repository: {}", repo);
        if let Some(t) = tag {
            println!("  Tag: {}", t);
        }
        if let Some(a) = asset {
            println!("  Asset: {}", a);
        }
    }

    let manager = SpecializedDownloaderManager::new();
    let manager = if let Some(token) = token {
        manager.with_github_token(token)
    } else {
        manager
    };

    let info = manager.download_github(repo, tag, asset).await?;

    if show_info || verbose {
        println!("\n{}", "Release Information:".yellow());
        if let Some(ref desc) = info.description {
            println!("  {}", desc);
        }
        if let Some(ref filename) = info.filename {
            println!("  Filename: {}", filename);
        }
        println!("  Download URL: {}", info.url);
        println!();
    }

    println!("{}", "Downloading...".cyan());

    let config = ChunkedDownloadConfig::default();
    let downloader = ChunkedDownload::new(config)?;

    let start = Instant::now();
    downloader
        .download(&info.url, output.to_str().unwrap())
        .await?;
    let elapsed = start.elapsed();

    println!("\n{}", "Download completed!".green().bold());
    println!("  Time: {:?}", elapsed);
    println!("  File: {}", output.display());

    Ok(())
}
