#![allow(clippy::too_many_arguments)]
#![allow(unused, reason = "temporary")]

mod chunked;
mod cli;
mod downloader;
mod error;
mod extract;
mod network_monitor;
mod progress;
mod resume;
mod scheduler;
mod specialized;
mod task;
mod torrent;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    crate::cli::run().await
}
