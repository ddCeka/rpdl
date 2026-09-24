use {
    std::{
        collections::VecDeque,
        sync::Arc,
        time::{Duration, Instant},
    },
    tokio::sync::RwLock,
};

#[derive(Debug, Clone, Copy)]
pub struct NetworkMetrics {
    pub bandwidth_bps: f64,
    pub avg_chunk_time_ms: f64,
    pub packet_loss_rate: f64,
    pub jitter_ms: f64,
    pub throughput_bps: f64,
}

#[derive(Debug, Clone)]
struct ChunkDownloadSample {
    bytes: u64,
    duration: Duration,
    timestamp: Instant,
    success: bool,
}

pub struct NetworkMonitor {
    samples: Arc<RwLock<VecDeque<ChunkDownloadSample>>>,
    max_samples: usize,
    window_size: Duration,
}

impl NetworkMonitor {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(RwLock::new(VecDeque::new())),
            max_samples: 50,
            window_size: Duration::from_secs(30),
        }
    }

    pub async fn record_download(&self, bytes: u64, duration: Duration, success: bool) {
        let mut samples = self.samples.write().await;

        samples.push_back(ChunkDownloadSample {
            bytes,
            duration,
            timestamp: Instant::now(),
            success,
        });

        while samples.len() > self.max_samples {
            samples.pop_front();
        }

        let cutoff = Instant::now() - self.window_size;
        while samples.front().is_some_and(|s| s.timestamp < cutoff) {
            samples.pop_front();
        }
    }

    pub async fn get_metrics(&self) -> NetworkMetrics {
        let samples = self.samples.read().await;

        if samples.is_empty() {
            return NetworkMetrics {
                bandwidth_bps: 10_000_000.0,
                avg_chunk_time_ms: 1000.0,
                packet_loss_rate: 0.0,
                jitter_ms: 0.0,
                throughput_bps: 10_000_000.0,
            };
        }

        let total_bytes: u64 = samples.iter().map(|s| s.bytes).sum();
        let total_duration: Duration = samples.iter().map(|s| s.duration).sum();
        let failed_count = samples.iter().filter(|s| !s.success).count();

        let throughput_bps = if total_duration.as_secs_f64() > 0.0 {
            (total_bytes as f64 * 8.0) / total_duration.as_secs_f64()
        } else {
            10_000_000.0
        };

        let avg_chunk_time_ms = if !samples.is_empty() {
            samples
                .iter()
                .map(|s| s.duration.as_millis() as f64)
                .sum::<f64>()
                / samples.len() as f64
        } else {
            10_000_000.0
        };

        let durations: Vec<f64> = samples
            .iter()
            .map(|s| s.duration.as_millis() as f64)
            .collect();

        let jitter_ms = if durations.len() > 1 {
            let mean = durations.iter().sum::<f64>() / durations.len() as f64;
            let variance =
                durations.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / durations.len() as f64;

            variance.sqrt()
        } else {
            0.0
        };

        let packet_loss_rate = failed_count as f64 / samples.len() as f64;
        let bandwidth_bps = self.estimate_bandwidth(throughput_bps, packet_loss_rate, jitter_ms);

        NetworkMetrics {
            bandwidth_bps,
            avg_chunk_time_ms,
            packet_loss_rate,
            jitter_ms,
            throughput_bps,
        }
    }

    fn estimate_bandwidth(&self, throughput: f64, loss_rate: f64, jitter: f64) -> f64 {
        let loss_penalty = 1.0 - (loss_rate * 0.5);
        let jitter_penalty = 1.0 - (jitter / 10000.0).min(0.3);

        throughput * loss_penalty * jitter_penalty
    }

    pub async fn predict_optimal_chunk_size(&self) -> u64 {
        let metrics = self.get_metrics().await;
        let base_chunk_size = 2 * 1024 * 1024;
        let bandwidth_mbps = metrics.bandwidth_bps / 1_000_000.0;

        let optimal_size = if bandwidth_mbps > 50.0 {
            10 * 1024 * 1024
        } else if bandwidth_mbps > 20.0 {
            5 * 1024 * 1024
        } else if bandwidth_mbps > 5.0 {
            base_chunk_size
        } else {
            1024 * 1024
        };

        if metrics.packet_loss_rate > 0.05 {
            optimal_size / 2
        } else if metrics.jitter_ms > 500.0 {
            (optimal_size as f64 * 0.7) as u64
        } else {
            optimal_size
        }
    }

    pub async fn get_recent_trend(&self) -> f64 {
        let samples = self.samples.read().await;

        if samples.len() < 3 {
            return 0.0;
        }

        let recent_samples: Vec<_> = samples.iter().rev().take(5).collect();

        if recent_samples.len() < 2 {
            return 0.0;
        }

        let recent_throughput: Vec<f64> = recent_samples
            .iter()
            .map(|s| (s.bytes as f64 * 8.0) / s.duration.as_secs_f64())
            .collect();

        let first_half = recent_throughput[..recent_throughput.len() / 2]
            .iter()
            .sum::<f64>()
            / (recent_throughput.len() / 2) as f64;
        let second_half = recent_throughput[recent_throughput.len() / 2..]
            .iter()
            .sum::<f64>()
            / (recent_throughput.len() - recent_throughput.len() / 2) as f64;

        (second_half - first_half) / first_half
    }
}

impl Default for NetworkMonitor {
    fn default() -> Self {
        Self::new()
    }
}
