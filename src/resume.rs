use {
    crate::error::Result,
    serde::{Deserialize, Serialize},
    std::path::{Path, PathBuf},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadState {
    pub url: String,
    pub output_path: PathBuf,
    pub total_size: u64,
    pub downloaded_chunks: Vec<ChunkState>,
    pub timestamp: u64,
    pub chunk_size: u64,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChunkState {
    pub index: usize,
    pub start: u64,
    pub end: u64,
    pub completed: bool,
    pub hash: Option<String>,
}

impl DownloadState {
    pub fn new(url: String, output_path: PathBuf, total_size: u64, chunk_size: u64) -> Self {
        let num_chunks = total_size.div_ceil(chunk_size);
        let mut chunks = Vec::with_capacity(num_chunks as usize);

        for i in 0..num_chunks {
            let start = i * chunk_size;
            let end = std::cmp::min(start + chunk_size - 1, total_size - 1);

            chunks.push(ChunkState {
                index: i as usize,
                start,
                end,
                completed: false,
                hash: None,
            });
        }

        Self {
            url,
            output_path,
            total_size,
            downloaded_chunks: chunks,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            chunk_size,
            etag: None,
            last_modified: None,
        }
    }

    pub fn state_file_path(output_path: &Path) -> PathBuf {
        let mut path = output_path.to_path_buf();
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("download");
        path.set_file_name(format!(".{}.rpdl_state", filename));
        path
    }

    pub async fn save(&self, output_path: &Path) -> Result<()> {
        let state_path = Self::state_file_path(output_path);
        let json = serde_json::to_string_pretty(self)?;
        tokio::fs::write(state_path, json).await?;
        Ok(())
    }

    pub async fn load(output_path: &Path) -> Result<Option<Self>> {
        let state_path = Self::state_file_path(output_path);
        if !state_path.exists() {
            return Ok(None);
        }

        let json = tokio::fs::read_to_string(state_path).await?;
        let state: DownloadState = serde_json::from_str(&json)?;
        Ok(Some(state))
    }

    pub async fn delete(output_path: &Path) -> Result<()> {
        let state_path = Self::state_file_path(output_path);
        if state_path.exists() {
            tokio::fs::remove_file(state_path).await?;
        }
        Ok(())
    }

    pub fn mark_chunk_completed(&mut self, index: usize) {
        if let Some(chunk) = self.downloaded_chunks.get_mut(index) {
            chunk.completed = true;
        }
    }

    pub fn get_incomplete_chunks(&self) -> Vec<ChunkState> {
        self.downloaded_chunks
            .iter()
            .filter(|c| !c.completed)
            .cloned()
            .collect()
    }

    pub fn is_complete(&self) -> bool {
        self.downloaded_chunks.iter().all(|c| c.completed)
    }

    pub fn progress(&self) -> f64 {
        let completed = self
            .downloaded_chunks
            .iter()
            .filter(|c| c.completed)
            .count();
        (completed as f64 / self.downloaded_chunks.len() as f64) * 100.0
    }

    pub fn validate_integrity(&self) -> bool {
        let mut sorted = self.downloaded_chunks.clone();
        sorted.sort_by_key(|c| c.start);

        for (i, chunk) in sorted.iter().enumerate() {
            if chunk.index != i {
                return false;
            }
        }

        if sorted.first().map(|c| c.start).unwrap_or(1) != 0 {
            return false;
        }

        if sorted.last().map(|c| c.end).unwrap_or(0) != self.total_size - 1 {
            return false;
        }

        true
    }
}