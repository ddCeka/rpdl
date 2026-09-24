use {
    crate::{
        chunked::{ChunkStrategy, ChunkedDownload, ChunkedDownloadConfig},
        progress::ProgressConfig,
    },
    color_eyre::Result,
    colored::*,
    std::time::Instant,
};

pub async fn handle(url: &str, samples: usize, verbose: bool) -> Result<()> {
    println!("{}", "Testing connection and gathering metrics...".cyan());
    println!("  URL: {}", url);
    println!("  Samples: {}", samples);
    println!();

    let config = ChunkedDownloadConfig::new()
        .chunk_size(1024 * 1024)
        .max_concurrent_chunks(samples)
        .strategy(ChunkStrategy::Smart)
        .progress_config(ProgressConfig::new().with_enabled(!verbose));

    let downloader = ChunkedDownload::new(config)?;

    let temp_file = std::env::temp_dir().join("rpdl_test.tmp");

    let start = Instant::now();
    match downloader.download(url, temp_file.to_str().unwrap()).await {
        Ok(_) => {
            let elapsed = start.elapsed();
            let metrics = downloader.get_network_metrics().await;

            println!("\n{}", "Connection Test Results:".green().bold());
            println!("  Time: {:?}", elapsed);
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

            let quality = if metrics.bandwidth_bps > 50_000_000.0 {
                "Excellent"
            } else if metrics.bandwidth_bps > 10_000_000.0 {
                "Good"
            } else if metrics.bandwidth_bps > 1_000_000.0 {
                "Fair"
            } else {
                "Poor"
            };

            println!(
                "\n{} {}",
                "Connection Quality:".yellow(),
                quality.green().bold()
            );

            tokio::fs::remove_file(&temp_file).await.ok();
        }
        Err(e) => {
            eprintln!("\n{} {}", "Test failed:".red(), e);
        }
    }

    Ok(())
}