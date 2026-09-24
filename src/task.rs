use {
    bytes::Bytes,
    serde::{Deserialize, Serialize},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DownloadTask {
    pub url: String,
    pub id: Option<String>,
}

impl DownloadTask {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            id: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

#[derive(Debug)]
pub struct DownloadResult {
    pub url: String,
    pub id: Option<String>,
    pub data: Bytes,
    pub status: u16,
    pub content_length: Option<u64>,
}
