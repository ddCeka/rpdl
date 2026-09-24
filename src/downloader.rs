use {
    crate::{
        error::{DownloadError, Result},
        progress::{ProgressConfig, ProgressTracker},
        task::{DownloadResult, DownloadTask},
    },
    futures::StreamExt,
    macror::BuilderLite,
    reqwest::Client,
    smart_default::SmartDefault,
    std::{sync::Arc, time::Duration},
    tokio::sync::Semaphore,
};

pub struct Downloader {
    client: Client,
    max_concurrent: usize,
    timeout: Duration,
    progress_config: ProgressConfig,
}

impl Downloader {
    pub fn builder() -> DownloadBuilder {
        DownloadBuilder::default()
    }

    pub async fn download(&self, tasks: Vec<DownloadTask>) -> Vec<Result<DownloadResult>> {
        let progress = Arc::new(ProgressTracker::new(
            tasks.len() as u64,
            self.progress_config.clone(),
        ));

        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));

        let results = futures::stream::iter(tasks)
            .map(|task| {
                let client = self.client.clone();
                let semaphore = semaphore.clone();
                let timeout = self.timeout;
                let progress = progress.clone();

                tokio::spawn(async move {
                    let _permit = semaphore
                        .acquire()
                        .await
                        .expect("Failed to acquire permit from semaphore");

                    let task_id = task.id.clone().unwrap_or_else(|| task.url.clone());
                    let task_pb = progress.create_task_progress(&task_id);

                    let result = Self::download_single(client, task, timeout).await;

                    let success = result.is_ok();
                    let msg = if let Ok(ref download) = result {
                        format!("{} ({} bytes)", download.url, download.data.len())
                    } else {
                        task_id.clone()
                    };

                    progress.finish_task(task_pb, success, &msg);
                    result
                })
            })
            .buffered(self.max_concurrent)
            .collect::<Vec<_>>()
            .await;

        progress.finish();

        results
            .into_iter()
            .map(|join_result| join_result.map_err(DownloadError::from).and_then(|r| r))
            .collect()
    }

    pub async fn download_unordered(
        &self,
        tasks: Vec<DownloadTask>,
    ) -> Vec<Result<DownloadResult>> {
        let progress = Arc::new(ProgressTracker::new(
            tasks.len() as u64,
            self.progress_config.clone(),
        ));

        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));

        let results = futures::stream::iter(tasks)
            .map(|task| {
                let client = self.client.clone();
                let semaphore = semaphore.clone();
                let timeout = self.timeout;
                let progress = progress.clone();

                tokio::spawn(async move {
                    let _permit = semaphore
                        .acquire()
                        .await
                        .expect("Failed to acquire permit from semaphore");

                    let task_id = task.id.clone().unwrap_or_else(|| task.url.clone());
                    let task_pb = progress.create_task_progress(&task_id);

                    let result = Self::download_single(client, task, timeout).await;

                    let success = result.is_ok();
                    let msg = if let Ok(ref download) = result {
                        format!("{} ({} bytes)", download.url, download.data.len())
                    } else {
                        task_id.clone()
                    };

                    progress.finish_task(task_pb, success, &msg);
                    result
                })
            })
            .buffer_unordered(self.max_concurrent)
            .collect::<Vec<_>>()
            .await;

        progress.finish();

        results
            .into_iter()
            .map(|join_result| join_result.map_err(DownloadError::from).and_then(|r| r))
            .collect()
    }

    pub async fn download_single(
        client: Client,
        task: DownloadTask,
        timeout: Duration,
    ) -> Result<DownloadResult> {
        let response = client.get(&task.url).timeout(timeout).send().await?;

        let status = response.status().as_u16();
        let content_length = response.content_length();
        let data = response.bytes().await?;

        Ok(DownloadResult {
            url: task.url,
            id: task.id,
            data,
            status,
            content_length,
        })
    }
}

#[derive(SmartDefault, BuilderLite)]
pub struct DownloadBuilder {
    #[default(10)]
    pub max_concurrent: usize,

    #[default(Duration::from_secs(30))]
    pub timeout: Duration,

    #[default(None)]
    pub user_agent: Option<String>,

    #[default(Duration::from_secs(10))]
    pub connection_timeout: Duration,

    #[default(ProgressConfig::default())]
    pub progress_config: ProgressConfig,
}

impl DownloadBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(self) -> Result<Downloader> {
        let mut client_builder = Client::builder()
            .connect_timeout(self.connection_timeout)
            .pool_max_idle_per_host(self.max_concurrent);

        if let Some(user_agent) = self.user_agent {
            client_builder = client_builder.user_agent(user_agent);
        }

        let client = client_builder.build()?;

        Ok(Downloader {
            client,
            max_concurrent: self.max_concurrent,
            timeout: self.timeout,
            progress_config: self.progress_config,
        })
    }
}