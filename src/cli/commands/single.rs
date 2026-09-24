use {
    super::super::args::StrategyArg,
    crate::{
        chunked::{ChunkedDownload, ChunkedDownloadConfig},
        progress::ProgressConfig,
    },
    color_eyre::Result,
    colored::*,
    std::{
        path::Path,
        time::{Duration, Instant},
    },
};

pub async fn handle(
    url: &str,
    output: &Path,
    max_chunks: usize,
    chunk_size: u64,
    strategy: StrategyArg,
    adaptive: bool,
    show_progress: bool,
    timeout: Duration,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("{}", "Starting single file download...".cyan());
        println!("  URL: {}", url);
        println!("  Output: {}", output.display());
        println!("  Strategy: {:?}", strategy);
        println!("  Max chunks: {}", max_chunks);
        println!("  Chunk size: {} MB", chunk_size / 1024 / 1024);
        println!("  Adaptive sizing: {}", adaptive);
        println!();
    }

    let config = ChunkedDownloadConfig::new()
        .chunk_size(chunk_size)
        .max_concurrent_chunks(max_chunks)
        .strategy(strategy.into())
        .adaptive_chunk_sizing(adaptive)
        .timeout(timeout)
        .progress_config(ProgressConfig::new().with_enabled(show_progress));

    let downloader = ChunkedDownload::new(config)?;

    let start = Instant::now();
    downloader.download(url, output.to_str().unwrap()).await?;
    let elapsed = start.elapsed();

    println!("\n{}", "Download completed successfully!".green().bold());
    println!("  Time: {:?}", elapsed);
    println!("  File: {}", output.display());

    if verbose {
        let metrics = downloader.get_network_metrics().await;
        println!("\n{}", "Network Metrics:".yellow());
        println!(
            "  Bandwidth: {:.2} Mbps",
            metrics.bandwidth_bps / 1_000_000.0
        );
        println!(
            "  Throughput: {:.2} Mbps",
            metrics.throughput_bps / 1_000_000.0
        );
        println!("  Avg chunk time: {:.0} ms", metrics.avg_chunk_time_ms);
        println!("  Packet loss: {:.2}%", metrics.packet_loss_rate * 100.0);
        println!("  Jitter: {:.2} ms", metrics.jitter_ms);
    }

    Ok(())
}
