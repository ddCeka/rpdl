use {
    crate::{
        chunked::{ChunkedDownload, ChunkedDownloadConfig},
        error::{DownloadError, Result},
    },
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
    std::{collections::HashMap, path::PathBuf, sync::Arc},
    tokio::{
        fs,
        sync::RwLock,
        time::{Duration, interval},
    },
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScheduledDownload {
    pub id: String,
    pub url: String,
    pub output_path: PathBuf,
    pub scheduled_time: DateTime<Utc>,
    pub repeat: Option<RepeatPattern>,
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    #[serde(default)]
    pub download_config: Option<ScheduledDownloadConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScheduledDownloadConfig {
    pub chunk_size: Option<u64>,
    pub max_concurrent_chunks: Option<usize>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RepeatPattern {
    Daily,
    Weekly,
    Monthly,
    Hourly,
    #[serde(rename = "minutes")]
    EveryMinutes(u64),
    #[serde(rename = "hours")]
    EveryHours(u64),
    #[serde(rename = "days")]
    EveryDays(u64),
    Cron(String),
}

impl RepeatPattern {
    fn next_run(&self, from: DateTime<Utc>) -> DateTime<Utc> {
        use chrono::Duration as ChronoDuration;

        match self {
            RepeatPattern::Hourly => from + ChronoDuration::hours(1),
            RepeatPattern::Daily => from + ChronoDuration::days(1),
            RepeatPattern::Weekly => from + ChronoDuration::weeks(1),
            RepeatPattern::Monthly => from + ChronoDuration::days(30),
            RepeatPattern::EveryMinutes(m) => from + ChronoDuration::minutes(*m as i64),
            RepeatPattern::EveryHours(h) => from + ChronoDuration::hours(*h as i64),
            RepeatPattern::EveryDays(d) => from + ChronoDuration::days(*d as i64),
            RepeatPattern::Cron(_expr) => from + ChronoDuration::hours(1),
        }
    }
}

pub struct Scheduler {
    scheduled: Arc<RwLock<HashMap<String, ScheduledDownload>>>,
    storage_path: PathBuf,
    running: Arc<RwLock<bool>>,
}

impl Scheduler {
    pub fn new(storage_path: PathBuf) -> Self {
        Self {
            scheduled: Arc::new(RwLock::new(HashMap::new())),
            storage_path,
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn load_from_disk(&self) -> Result<()> {
        if !self.storage_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.storage_path).await?;
        let downloads: Vec<ScheduledDownload> = serde_json::from_str(&content)
            .map_err(|e| DownloadError::InvalidUrl(format!("Failed to parse schedule: {}", e)))?;

        let mut scheduled = self.scheduled.write().await;
        for download in downloads {
            scheduled.insert(download.id.clone(), download);
        }

        Ok(())
    }

    pub async fn save_to_disk(&self) -> Result<()> {
        let scheduled = self.scheduled.read().await;
        let downloads: Vec<ScheduledDownload> = scheduled.values().cloned().collect();

        let json = serde_json::to_string_pretty(&downloads)
            .map_err(|e| DownloadError::InvalidUrl(format!("Failed to serialize: {}", e)))?;

        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&self.storage_path, json).await?;
        Ok(())
    }

    pub async fn add(&self, mut download: ScheduledDownload) -> Result<()> {
        download.next_run = Some(download.scheduled_time);

        let mut scheduled = self.scheduled.write().await;
        scheduled.insert(download.id.clone(), download);
        drop(scheduled);

        self.save_to_disk().await?;
        Ok(())
    }

    pub async fn remove(&self, id: &str) -> Result<bool> {
        let mut scheduled = self.scheduled.write().await;
        let removed = scheduled.remove(id).is_some();
        drop(scheduled);

        if removed {
            self.save_to_disk().await?;
        }

        Ok(removed)
    }

    pub async fn enable(&self, id: &str, enabled: bool) -> Result<bool> {
        let mut scheduled = self.scheduled.write().await;

        if let Some(download) = scheduled.get_mut(id) {
            download.enabled = enabled;
            drop(scheduled);
            self.save_to_disk().await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn list(&self) -> Vec<ScheduledDownload> {
        let scheduled = self.scheduled.read().await;
        scheduled.values().cloned().collect()
    }

    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(DownloadError::InvalidUrl(
                "Scheduler already running".into(),
            ));
        }
        *running = true;
        drop(running);

        let scheduled = self.scheduled.clone();
        let storage_path = self.storage_path.clone();
        let running_flag = self.running.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(60));

            while *running_flag.read().await {
                ticker.tick().await;

                let now = Utc::now();
                let mut to_run = Vec::new();

                {
                    let scheduled_read = scheduled.read().await;
                    for (id, download) in scheduled_read.iter() {
                        if !download.enabled {
                            continue;
                        }

                        if let Some(next_run) = download.next_run
                            && next_run <= now
                        {
                            to_run.push((id.clone(), download.clone()));
                        }
                    }
                }

                for (id, mut download) in to_run {
                    println!("⏰ Running scheduled download: {}", id);

                    match Self::execute_download(&download).await {
                        Ok(_) => {
                            println!("✓ Completed: {}", id);
                            download.last_run = Some(now);

                            if let Some(repeat) = &download.repeat {
                                download.next_run = Some(repeat.next_run(now));
                            } else {
                                download.enabled = false;
                            }
                        }
                        Err(e) => {
                            eprintln!("✗ Failed {}: {}", id, e);
                        }
                    }

                    let mut scheduled_write = scheduled.write().await;
                    scheduled_write.insert(id.clone(), download);
                    drop(scheduled_write);

                    let scheduled_clone = scheduled.clone();
                    let storage_clone = storage_path.clone();
                    tokio::spawn(async move {
                        let s = Self {
                            scheduled: scheduled_clone,
                            storage_path: storage_clone,
                            running: Arc::new(RwLock::new(false)),
                        };
                        let _ = s.save_to_disk().await;
                    });
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }

    async fn execute_download(download: &ScheduledDownload) -> Result<()> {
        let mut config = ChunkedDownloadConfig::default();

        if let Some(dl_config) = &download.download_config {
            if let Some(chunk_size) = dl_config.chunk_size {
                config = config.chunk_size(chunk_size);
            }
            if let Some(max_chunks) = dl_config.max_concurrent_chunks {
                config = config.max_concurrent_chunks(max_chunks);
            }
            if let Some(timeout_secs) = dl_config.timeout_secs {
                config = config.timeout(Duration::from_secs(timeout_secs));
            }
        }

        let downloader = ChunkedDownload::new(config)?;
        let output = download
            .output_path
            .to_str()
            .ok_or_else(|| DownloadError::InvalidUrl("Invalid output path".into()))?;

        downloader.download(&download.url, output).await?;
        Ok(())
    }
}
