use {
    crate::{
        error::{DownloadError, Result},
        network_monitor::{NetworkMetrics, NetworkMonitor},
        progress::{ProgressConfig, ProgressTracker},
        resume::DownloadState,
    },
    bytes::Bytes,
    futures::stream::{self, StreamExt},
    reqwest::{Client, StatusCode, header},
    std::{
        cmp::Ordering,
        collections::BinaryHeap,
        sync::Arc,
        time::{Duration, Instant},
    },
    tokio::{
        fs::File,
        io::AsyncWriteExt,
        sync::{RwLock, Semaphore},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkStrategy {
    Simple,
    Smart,
}

#[derive(Debug, Clone)]
pub struct ChunkedDownloadConfig {
    pub chunk_size: u64,
    pub max_concurrent_chunks: usize,
    pub timeout: Duration,
    pub strategy: ChunkStrategy,
    pub progress_config: ProgressConfig,
    pub adaptive_chunk_sizing: bool,
}

impl Default for ChunkedDownloadConfig {
    fn default() -> Self {
        Self {
            chunk_size: 5 * 1024 * 1024,
            max_concurrent_chunks: 8,
            timeout: Duration::from_secs(30),
            strategy: ChunkStrategy::Simple,
            progress_config: ProgressConfig::default(),
            adaptive_chunk_sizing: true,
        }
    }
}

impl ChunkedDownloadConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn chunk_size(mut self, size: u64) -> Self {
        self.chunk_size = size;
        self
    }

    pub fn max_concurrent_chunks(mut self, max: usize) -> Self {
        self.max_concurrent_chunks = max;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn strategy(mut self, strategy: ChunkStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn progress_config(mut self, config: ProgressConfig) -> Self {
        self.progress_config = config;
        self
    }

    pub fn adaptive_chunk_sizing(mut self, enabled: bool) -> Self {
        self.adaptive_chunk_sizing = enabled;
        self
    }
}

#[derive(Debug, Clone)]
pub struct ChunkInfo {
    pub index: usize,
    pub start: u64,
    pub end: u64,
    pub priority: ChunkPriority,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChunkPriority {
    Critical = 4,
    High = 3,
    Normal = 2,
    Low = 1,
    Deferred = 0,
}

impl ChunkPriority {
    fn value(&self) -> u8 {
        *self as u8
    }

    fn from_metrics(
        index: usize,
        total_chunks: usize,
        metrics: &NetworkMetrics,
        trend: f64,
    ) -> Self {
        let position_ratio = index as f64 / total_chunks as f64;

        let base_priority = if position_ratio < 0.05 {
            ChunkPriority::Critical
        } else if position_ratio < 0.20 {
            ChunkPriority::High
        } else if position_ratio > 0.90 {
            ChunkPriority::Low
        } else {
            ChunkPriority::Normal
        };

        let bandwidth_mbps = metrics.bandwidth_bps / 1_000_000.0;

        if bandwidth_mbps < 1.0 && position_ratio > 0.5 {
            return ChunkPriority::Deferred;
        }

        if trend < -0.2 && position_ratio > 0.3 {
            match base_priority {
                ChunkPriority::Critical => ChunkPriority::High,
                ChunkPriority::High => ChunkPriority::Normal,
                ChunkPriority::Normal => ChunkPriority::Low,
                _ => ChunkPriority::Low,
            }
        } else if trend > 0.2 && metrics.packet_loss_rate < 0.01 {
            match base_priority {
                ChunkPriority::Low => ChunkPriority::Normal,
                ChunkPriority::Normal => ChunkPriority::High,
                _ => base_priority,
            }
        } else {
            base_priority
        }
    }
}

#[derive(Debug, Clone)]
struct PrioritizedChunk {
    chunk: ChunkInfo,
    computed_priority: f64,
}

impl PartialEq for PrioritizedChunk {
    fn eq(&self, other: &Self) -> bool {
        self.computed_priority == other.computed_priority
    }
}

impl Eq for PrioritizedChunk {}

impl PartialOrd for PrioritizedChunk {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedChunk {
    fn cmp(&self, other: &Self) -> Ordering {
        self.computed_priority
            .partial_cmp(&other.computed_priority)
            .unwrap_or(Ordering::Equal)
            .then_with(|| other.chunk.index.cmp(&self.chunk.index))
    }
}

#[derive(Debug)]
pub struct ChunkResult {
    pub index: usize,
    pub data: Bytes,
}

pub struct ChunkedDownload {
    client: Client,
    config: ChunkedDownloadConfig,
    network_monitor: Arc<NetworkMonitor>,
}

impl ChunkedDownload {
    pub fn new(config: ChunkedDownloadConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .pool_idle_timeout(Some(Duration::from_secs(90)))
            .pool_max_idle_per_host(config.max_concurrent_chunks)
            .http2_keep_alive_interval(Some(Duration::from_secs(10)))
            .http2_keep_alive_timeout(Duration::from_secs(20))
            .build()?;

        Ok(Self {
            client,
            config,
            network_monitor: Arc::new(NetworkMonitor::new()),
        })
    }

    pub async fn download(&self, url: &str, output_path: &str) -> Result<()> {
        if url.is_empty() {
            return Err(DownloadError::InvalidUrl("URL cannot be empty".to_string()));
        }

        let content_length = self.get_content_length(url).await?;
        self.check_range_support(url).await?;

        let initial_chunk_size = if self.config.adaptive_chunk_sizing {
            self.network_monitor.predict_optimal_chunk_size().await
        } else {
            self.config.chunk_size
        };

        let chunks = self.create_chunks(content_length, initial_chunk_size);

        let progress = Arc::new(ProgressTracker::new(
            chunks.len() as u64,
            self.config.progress_config.clone(),
        ));

        let chunk_results = match self.config.strategy {
            ChunkStrategy::Simple => self.download_simple(url, chunks, progress.clone()).await?,
            ChunkStrategy::Smart => self.download_smart(url, chunks, progress.clone()).await?,
        };

        progress.finish();

        self.write_chunks_to_file(chunk_results, output_path)
            .await?;

        Ok(())
    }

    async fn get_content_length(&self, url: &str) -> Result<u64> {
        let response = self.client.head(url).send().await?.error_for_status()?;

        response
            .headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .ok_or(DownloadError::ContentLengthUnknown)
    }

    async fn check_range_support(&self, url: &str) -> Result<()> {
        let response = self.client.head(url).send().await?;

        let supports_ranges = response
            .headers()
            .get(header::ACCEPT_RANGES)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.eq_ignore_ascii_case("bytes"))
            .unwrap_or(false);

        if supports_ranges {
            Ok(())
        } else {
            Err(DownloadError::RangeNotSupported)
        }
    }

    fn create_chunks(&self, content_length: u64, chunk_size: u64) -> Vec<ChunkInfo> {
        let mut chunks = Vec::new();
        let mut start = 0u64;
        let mut index = 0;

        while start < content_length {
            let end = std::cmp::min(start + chunk_size - 1, content_length - 1);
            let size = end - start + 1;

            chunks.push(ChunkInfo {
                index,
                start,
                end,
                priority: ChunkPriority::Normal,
                size,
            });

            start = end + 1;
            index += 1;
        }

        chunks
    }

    async fn prioritize_chunks_intelligently(&self, chunks: Vec<ChunkInfo>) -> Vec<ChunkInfo> {
        let total_chunks = chunks.len();
        let metrics = self.network_monitor.get_metrics().await;
        let trend = self.network_monitor.get_recent_trend().await;

        chunks
            .into_iter()
            .map(|mut chunk| {
                chunk.priority =
                    ChunkPriority::from_metrics(chunk.index, total_chunks, &metrics, trend);
                chunk
            })
            .collect()
    }

    async fn download_simple(
        &self,
        url: &str,
        chunks: Vec<ChunkInfo>,
        progress: Arc<ProgressTracker>,
    ) -> Result<Vec<ChunkResult>> {
        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_chunks));
        let total_chunks = chunks.len();

        let results = stream::iter(chunks.into_iter().enumerate())
            .map(|(idx, chunk)| {
                let client = self.client.clone();
                let url = url.to_string();
                let semaphore = semaphore.clone();
                let progress = progress.clone();
                let monitor = self.network_monitor.clone();

                async move {
                    let _permit = semaphore
                        .acquire()
                        .await
                        .map_err(|_| DownloadError::Cancelled)?;

                    let task_pb = progress.create_task_progress(&format!(
                        "Chunk {}/{}",
                        idx + 1,
                        total_chunks
                    ));

                    let start_time = Instant::now();
                    let result = Self::download_chunk_data(&client, &url, &chunk).await;
                    let duration = start_time.elapsed();

                    let success = result.is_ok();
                    monitor.record_download(chunk.size, duration, success).await;

                    let message = format!(
                        "Chunk {}/{} ({}-{}) {:.2}MB/s",
                        idx + 1,
                        total_chunks,
                        chunk.start,
                        chunk.end,
                        (chunk.size as f64 / duration.as_secs_f64()) / 1_000_000.0
                    );
                    progress.finish_task(task_pb, success, &message);

                    result
                }
            })
            .buffer_unordered(self.config.max_concurrent_chunks)
            .collect::<Vec<_>>()
            .await;

        results.into_iter().collect()
    }

    async fn download_smart(
        &self,
        url: &str,
        chunks: Vec<ChunkInfo>,
        progress: Arc<ProgressTracker>,
    ) -> Result<Vec<ChunkResult>> {
        let prioritized_chunks = self.prioritize_chunks_intelligently(chunks).await;

        let priority_queue = Arc::new(RwLock::new({
            let mut pq = BinaryHeap::new();
            for chunk in prioritized_chunks {
                let computed_priority =
                    chunk.priority.value() as f64 + (1.0 / (chunk.index + 1) as f64) * 0.5;

                pq.push(PrioritizedChunk {
                    chunk,
                    computed_priority,
                });
            }
            pq
        }));

        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_chunks));
        let mut tasks = Vec::with_capacity(self.config.max_concurrent_chunks);

        for worker_id in 0..self.config.max_concurrent_chunks {
            let client = self.client.clone();
            let url = url.to_string();
            let semaphore = semaphore.clone();
            let queue = priority_queue.clone();
            let progress = progress.clone();
            let monitor = self.network_monitor.clone();
            let adaptive = self.config.adaptive_chunk_sizing;

            let task = tokio::spawn(async move {
                let mut results = Vec::new();
                let mut consecutive_downloads = 0;

                loop {
                    let chunk = {
                        let mut q = queue.write().await;
                        q.pop().map(|pc| pc.chunk)
                    };

                    let Some(chunk_info) = chunk else {
                        break;
                    };

                    if chunk_info.priority == ChunkPriority::Deferred {
                        let metrics = monitor.get_metrics().await;
                        if metrics.bandwidth_bps < 500_000.0 {
                            tokio::time::sleep(Duration::from_millis(500)).await;
                        }
                    }

                    let _permit = semaphore
                        .acquire()
                        .await
                        .map_err(|_| DownloadError::Cancelled)?;

                    let task_pb = progress.create_task_progress(&format!(
                        "W{} Chunk {} (P:{:?})",
                        worker_id, chunk_info.index, chunk_info.priority
                    ));

                    let start_time = Instant::now();
                    let result = Self::download_chunk_data(&client, &url, &chunk_info).await;
                    let duration = start_time.elapsed();

                    let success = result.is_ok();
                    monitor
                        .record_download(chunk_info.size, duration, success)
                        .await;

                    let speed_mbps =
                        (chunk_info.size as f64 / duration.as_secs_f64()) / 1_000_000.0;
                    let message = format!(
                        "Chunk {} ({}-{}) P:{:?} {:.2}MB/s",
                        chunk_info.index,
                        chunk_info.start,
                        chunk_info.end,
                        chunk_info.priority,
                        speed_mbps
                    );
                    progress.finish_task(task_pb, success, &message);

                    match result {
                        Ok(chunk_result) => results.push(chunk_result),
                        Err(e) => return Err(e),
                    }

                    consecutive_downloads += 1;

                    if adaptive && consecutive_downloads % 5 == 0 {
                        let metrics = monitor.get_metrics().await;
                        if metrics.packet_loss_rate > 0.1 {
                            tokio::time::sleep(Duration::from_millis(200)).await;
                        }
                    }
                }

                Ok(results)
            });

            tasks.push(task);
        }

        let mut all_results = Vec::new();
        for task in tasks {
            let mut worker_results = task.await.map_err(DownloadError::JoinError)??;
            all_results.append(&mut worker_results);
        }

        all_results.sort_by_key(|r| r.index);
        Ok(all_results)
    }

    async fn download_chunk_data(
        client: &Client,
        url: &str,
        chunk: &ChunkInfo,
    ) -> Result<ChunkResult> {
        let range_header = format!("bytes={}-{}", chunk.start, chunk.end);

        let response = client
            .get(url)
            .header(header::RANGE, range_header)
            .send()
            .await?;

        let status = response.status();

        if !matches!(status, StatusCode::OK | StatusCode::PARTIAL_CONTENT) {
            return Err(DownloadError::RequestFailed(
                response.error_for_status().unwrap_err(),
            ));
        }

        let data = response.bytes().await?;

        if data.len() as u64 != chunk.size {
            return Err(DownloadError::InvalidChunkRange(format!(
                "Expected {} bytes, got {} bytes for chunk {}",
                chunk.size,
                data.len(),
                chunk.index
            )));
        }

        Ok(ChunkResult {
            index: chunk.index,
            data,
        })
    }

    async fn append_chunks_to_file(
        &self,
        mut chunks: Vec<ChunkResult>,
        output_path: &str,
        state: &mut DownloadState,
    ) -> Result<()> {
        use tokio::io::{AsyncSeekExt, AsyncWriteExt};

        chunks.sort_by_key(|c| c.index);

        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_path)
            .await?;

        for chunk in chunks {
            let chunk_state = &state.downloaded_chunks[chunk.index];
            file.seek(std::io::SeekFrom::Start(chunk_state.start))
                .await?;
            file.write_all(&chunk.data).await?;

            state.mark_chunk_completed(chunk.index);
        }

        file.flush().await?;
        file.sync_all().await?;

        state.save(std::path::Path::new(output_path)).await?;

        Ok(())
    }

    pub async fn resume(&self, state: &mut DownloadState, output_path: &str) -> Result<()> {
        let incomplete_chunks: Vec<ChunkInfo> = state
            .get_incomplete_chunks()
            .into_iter()
            .map(|cs| ChunkInfo {
                index: cs.index,
                start: cs.start,
                end: cs.end,
                priority: ChunkPriority::Normal,
                size: cs.end - cs.start + 1,
            })
            .collect();

        if incomplete_chunks.is_empty() {
            return Ok(());
        }

        let progress = Arc::new(ProgressTracker::new(
            incomplete_chunks.len() as u64,
            self.config.progress_config.clone(),
        ));

        let chunk_results = match self.config.strategy {
            ChunkStrategy::Simple => {
                self.download_simple(&state.url, incomplete_chunks, progress.clone())
                    .await?
            }
            ChunkStrategy::Smart => {
                self.download_smart(&state.url, incomplete_chunks, progress.clone())
                    .await?
            }
        };

        progress.finish();

        self.append_chunks_to_file(chunk_results, output_path, state)
            .await?;

        Ok(())
    }

    async fn write_chunks_to_file(
        &self,
        mut chunks: Vec<ChunkResult>,
        output_path: &str,
    ) -> Result<()> {
        chunks.sort_by_key(|c| c.index);

        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.index != i {
                return Err(DownloadError::InvalidChunkRange(format!(
                    "Missing chunk at index {}",
                    i
                )));
            }
        }

        let mut file = File::create(output_path).await?;

        for chunk in chunks {
            file.write_all(&chunk.data).await?;
        }

        file.flush().await?;
        file.sync_all().await?;

        Ok(())
    }

    pub async fn get_network_metrics(&self) -> NetworkMetrics {
        self.network_monitor.get_metrics().await
    }
}
