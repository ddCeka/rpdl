use {
    crate::error::{DownloadError, Result},
    colored::Colorize,
    librqbit::{AddTorrent, AddTorrentOptions, ManagedTorrent, Session},
    std::{path::PathBuf, sync::Arc, time::Duration},
    tokio::time::interval,
};

pub struct TorrentDownloader {
    session: Arc<Session>,
}

impl TorrentDownloader {
    pub async fn new(download_dir: PathBuf) -> Result<Self> {
        if !download_dir.exists() {
            tokio::fs::create_dir_all(&download_dir)
                .await
                .map_err(|e| {
                    DownloadError::InvalidUrl(format!("Failed to create download directory: {}", e))
                })?;
        }

        let session = Session::new(download_dir)
            .await
            .map_err(|e| DownloadError::InvalidUrl(format!("Failed to create session: {}", e)))?;

        Ok(Self { session })
    }

    pub async fn download_from_url(
        &self,
        url: &str,
        output_dir: Option<PathBuf>,
        list_only: bool,
        file_pattern: Option<String>,
        show_progress: bool,
    ) -> Result<()> {
        if url.is_empty() {
            return Err(DownloadError::InvalidUrl("URL cannot be empty".into()));
        }

        println!("{}", "Parsing torrent/magnet link...".cyan());

        let add_torrent = if url.starts_with("magnet:")
            || url.starts_with("http://")
            || url.starts_with("https://")
        {
            AddTorrent::from_url(url)
        } else {
            if !tokio::fs::try_exists(url).await.unwrap_or(false) {
                return Err(DownloadError::InvalidUrl(format!(
                    "Torrent file not found: {}",
                    url
                )));
            }

            let content = tokio::fs::read(url).await.map_err(|e| {
                DownloadError::InvalidUrl(format!("Failed to read torrent file: {}", e))
            })?;

            if content.is_empty() {
                return Err(DownloadError::InvalidUrl("Torrent file is empty".into()));
            }

            AddTorrent::from_bytes(content)
        };

        if let Some(ref out_dir) = output_dir
            && !out_dir.exists()
        {
            tokio::fs::create_dir_all(out_dir).await.map_err(|e| {
                DownloadError::InvalidUrl(format!("Failed to create output directory: {}", e))
            })?;
        }

        let opts = AddTorrentOptions {
            overwrite: false,
            only_files_regex: file_pattern,
            output_folder: output_dir.map(|p| p.to_string_lossy().to_string()),
            list_only,
            ..Default::default()
        };

        if list_only {
            let response = self
                .session
                .add_torrent(add_torrent, Some(opts))
                .await
                .map_err(|e| DownloadError::InvalidUrl(format!("Failed to list files: {}", e)))?;

            if let Some(handle) = response.into_handle() {
                if let Some(metadata) = handle.metadata.load().as_ref() {
                    println!("\n{}", "Torrent files:".cyan().bold());

                    if let Ok(files) = metadata.info.iter_file_details() {
                        for (idx, file) in files.enumerate() {
                            let filename = file
                                .filename
                                .to_string()
                                .unwrap_or_else(|_| format!("<invalid UTF-8 name #{}>", idx));
                            let size = humanize_bytes(file.len);
                            println!("  [{}] {} ({})", idx, filename, size);
                        }
                    } else {
                        println!("  {}", "No files found in torrent".yellow());
                    }

                    println!(
                        "\n{} {}",
                        "Total size:".yellow(),
                        humanize_bytes(metadata.lengths.total_length())
                    );
                } else {
                    return Err(DownloadError::InvalidUrl("Metadata not available".into()));
                }
            }

            return Ok(());
        }

        let response = self
            .session
            .add_torrent(add_torrent, Some(opts))
            .await
            .map_err(|e| DownloadError::InvalidUrl(format!("Failed to add torrent: {}", e)))?;

        let handle = response
            .into_handle()
            .ok_or_else(|| DownloadError::InvalidUrl("Torrent handle not available".into()))?;

        let metadata = handle
            .metadata
            .load()
            .as_ref()
            .ok_or_else(|| DownloadError::InvalidUrl("Metadata not available".into()))?
            .clone();

        let name = metadata
            .name
            .as_ref()
            .map(|n| n.to_string())
            .unwrap_or_else(|| "<unknown>".to_string());

        println!(
            "{} {} ({})",
            "Downloading:".green().bold(),
            name,
            humanize_bytes(metadata.lengths.total_length())
        );

        if show_progress {
            self.monitor_progress(handle.clone()).await;
        }

        println!("{}", "Waiting for download to complete...".yellow());
        handle
            .wait_until_completed()
            .await
            .map_err(|e| DownloadError::InvalidUrl(format!("Download failed: {}", e)))?;

        println!("\n{}", "Torrent download completed!".green().bold());
        Ok(())
    }

    async fn monitor_progress(&self, handle: Arc<ManagedTorrent>) {
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(1));

            loop {
                ticker.tick().await;

                let stats = handle.stats();

                if stats.finished {
                    println!("\r{} 100% - Download complete!             ", "✓".green());
                    break;
                }

                let total = stats.total_bytes.max(1);
                let progress_percent = (stats.progress_bytes as f64 / total as f64 * 100.0) as u64;

                print!(
                    "\r{} {}% - Downloaded: {} / {}   ",
                    "↓".cyan(),
                    progress_percent,
                    humanize_bytes(stats.progress_bytes),
                    humanize_bytes(stats.total_bytes)
                );
                use std::io::Write;
                let _ = std::io::stdout().flush();
            }
        });
    }

    pub async fn get_info(&self, url: &str) -> Result<TorrentInfo> {
        if url.is_empty() {
            return Err(DownloadError::InvalidUrl("URL cannot be empty".into()));
        }

        let add_torrent = if url.starts_with("magnet:")
            || url.starts_with("http://")
            || url.starts_with("https://")
        {
            AddTorrent::from_url(url)
        } else {
            if !tokio::fs::try_exists(url).await.unwrap_or(false) {
                return Err(DownloadError::InvalidUrl(format!(
                    "Torrent file not found: {}",
                    url
                )));
            }

            let content = tokio::fs::read(url).await.map_err(|e| {
                DownloadError::InvalidUrl(format!("Failed to read torrent file: {}", e))
            })?;

            if content.is_empty() {
                return Err(DownloadError::InvalidUrl("Torrent file is empty".into()));
            }

            AddTorrent::from_bytes(content)
        };

        let opts = AddTorrentOptions {
            list_only: true,
            ..Default::default()
        };

        let response = self
            .session
            .add_torrent(add_torrent, Some(opts))
            .await
            .map_err(|e| DownloadError::InvalidUrl(format!("Failed to get info: {}", e)))?;

        let handle = response
            .into_handle()
            .ok_or_else(|| DownloadError::InvalidUrl("No handle available".into()))?;

        let metadata = handle
            .metadata
            .load()
            .as_ref()
            .ok_or_else(|| DownloadError::InvalidUrl("Metadata not available".into()))?
            .clone();

        let mut files = Vec::new();
        if let Ok(file_iter) = metadata.info.iter_file_details() {
            for (idx, file) in file_iter.enumerate() {
                let name = file
                    .filename
                    .to_string()
                    .unwrap_or_else(|_| format!("<invalid UTF-8 name #{}>", idx));
                files.push(FileInfo {
                    name,
                    size: file.len,
                });
            }
        }

        let name = metadata
            .info
            .name
            .as_ref()
            .map(|n| n.to_string())
            .unwrap_or_else(|| "<unknown>".to_string());

        let total_size = metadata.info.length.unwrap_or(0);

        Ok(TorrentInfo {
            name,
            total_size,
            files,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TorrentInfo {
    pub name: String,
    pub total_size: u64,
    pub files: Vec<FileInfo>,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
}

fn humanize_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_idx])
}
