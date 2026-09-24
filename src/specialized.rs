use {
    crate::{
        cli::utils::sanitize_filename,
        error::{DownloadError, Result},
    },
    async_trait::async_trait,
    futures::TryFutureExt,
    reqwest::Client,
    serde::Deserialize,
    std::path::Path,
    tokio::{fs::File, io::AsyncWriteExt},
};

pub use octocrab::Octocrab;

#[async_trait]
pub trait SiteDownloader: Send + Sync {
    async fn get_download_url(&self, input: &str) -> Result<DownloadInfo>;
    fn site_id(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct DownloadInfo {
    pub url: String,
    pub filename: Option<String>,
    pub description: Option<String>,
    pub auth_token: Option<String>,
}

impl DownloadInfo {
    pub async fn download_simple(&self, output_path: &Path) -> Result<()> {
        let client_builder =
            Client::builder().user_agent("Mozilla/5.0 (compatible; RPDL/1.0)");

        let client = client_builder.build()?;

        let mut request = client.get(&self.url);

        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.send().await?.error_for_status()?;

        let bytes = response.bytes().await?;

        let mut file = File::create(output_path).await?;
        file.write_all(&bytes).await?;
        file.flush().await?;
        file.sync_all().await?;

        Ok(())
    }

    pub async fn supports_chunking(&self) -> bool {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (compatible; RPDL/1.0)")
            .build();

        let Ok(client) = client else {
            return false;
        };

        let Ok(response) = client.head(&self.url).send().await else {
            return false;
        };

        let has_content_length = response.headers().get("content-length").is_some();
        let accepts_ranges = response
            .headers()
            .get("accept-ranges")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.eq_ignore_ascii_case("bytes"))
            .unwrap_or(false);

        has_content_length && accepts_ranges
    }
}

pub struct SpecializedDownloaderManager {
    github_token: Option<String>,
}

impl SpecializedDownloaderManager {
    pub fn new() -> Self {
        Self {
            github_token: None,
        }
    }

    pub fn with_github_token(mut self, token: String) -> Self {
        self.github_token = Some(token);
        self
    }

    pub async fn download_github(
        &self,
        repo: &str,
        tag: Option<&str>,
        asset: Option<&str>,
    ) -> Result<DownloadInfo> {
        let downloader = GitHubDownloader::new(self.github_token.clone())?;
        downloader.get_download_url_v2(repo, tag, asset).await
    }
}

pub struct GitHubDownloader {
    client: Octocrab,
}

impl GitHubDownloader {
    pub fn new(token: Option<String>) -> Result<Self> {
        let mut builder = Octocrab::builder();

        if let Some(token) = token {
            builder = builder.personal_token(token);
        }

        let client = builder.build().map_err(|e| {
            DownloadError::InvalidUrl(format!("Failed to initialize GitHub client: {}", e))
        })?;

        Ok(Self { client })
    }

    pub async fn get_download_url_v2(
        &self,
        repo: &str,
        tag: Option<&str>,
        asset: Option<&str>,
    ) -> Result<DownloadInfo> {
        let (repo_path, commit) = if let Some(idx) = repo.find('#') {
            (&repo[..idx], Some(&repo[idx + 1..]))
        } else {
            (repo, None)
        };

        let parts: Vec<&str> = repo_path.split('/').collect();
        if parts.len() != 2 {
            return Err(DownloadError::InvalidUrl(
                "Format must be 'owner/repo' or 'owner/repo#commit'".into(),
            ));
        }

        let owner = parts[0];
        let repo_name = parts[1];

        if let Some(commit_hash) = commit {
            let url = format!(
                "https://github.com/{}/{}/archive/{}.zip",
                owner, repo_name, commit_hash
            );
            let filename = format!("{}-{}.zip", repo_name, commit_hash);
            let description = format!("{}/{} @ {}", owner, repo_name, commit_hash);

            return Ok(DownloadInfo {
                url,
                filename: Some(filename),
                description: Some(description),
                auth_token: None,
            });
        }

        if tag.is_none() {
            let url = format!(
                "https://github.com/{}/{}/archive/refs/heads/main.zip",
                owner, repo_name
            );
            let filename = format!("{}-main.zip", repo_name);
            let description = format!("{}/{} - main branch source", owner, repo_name);

            return Ok(DownloadInfo {
                url,
                filename: Some(filename),
                description: Some(description),
                auth_token: None,
            });
        }

        let tag_name = tag.unwrap();

        let release = self
            .client
            .repos(owner, repo_name)
            .releases()
            .get_by_tag(tag_name)
            .await
            .map_err(|e| {
                DownloadError::InvalidUrl(format!("Failed to fetch release '{}': {}", tag_name, e))
            })?;

        if let Some(asset_name) = asset {
            let found_asset = release
                .assets
                .iter()
                .find(|a| a.name.contains(asset_name))
                .ok_or_else(|| {
                    DownloadError::InvalidUrl(format!("No asset matching '{}' found", asset_name))
                })?;

            return Ok(DownloadInfo {
                url: found_asset.browser_download_url.clone().to_string(),
                filename: Some(found_asset.name.clone()),
                description: Some(format!(
                    "{}/{} - Release {} - {}",
                    owner, repo_name, release.tag_name, found_asset.name
                )),
                auth_token: None,
            });
        }

        if release.assets.is_empty() {
            return Err(DownloadError::InvalidUrl(format!(
                "No assets found in release '{}'",
                tag_name
            )));
        }

        let first_asset = &release.assets[0];
        Ok(DownloadInfo {
            url: first_asset.browser_download_url.clone().to_string(),
            filename: Some(first_asset.name.clone()),
            description: Some(format!(
                "{}/{} - Release {} ({} assets available)",
                owner,
                repo_name,
                release.tag_name,
                release.assets.len()
            )),
            auth_token: None,
        })
    }
}

#[async_trait]
impl SiteDownloader for GitHubDownloader {
    async fn get_download_url(&self, input: &str) -> Result<DownloadInfo> {
        let parts: Vec<&str> = input.split('/').collect();

        if parts.len() < 2 {
            return Err(DownloadError::InvalidUrl(
                "Format must be 'owner/repo[/asset_name]'".into(),
            ));
        }

        let owner = parts[0];
        let repo = parts[1];
        let asset_filter = parts.get(2).copied();

        let release = self
            .client
            .repos(owner, repo)
            .releases()
            .get_latest()
            .await
            .map_err(|e| DownloadError::InvalidUrl(format!("Failed to fetch release: {}", e)))?;

        let asset = if let Some(filter) = asset_filter {
            release
                .assets
                .iter()
                .find(|a| a.name.contains(filter))
                .ok_or_else(|| {
                    DownloadError::InvalidUrl(format!("No asset matching '{}' found", filter))
                })?
        } else {
            release
                .assets
                .first()
                .ok_or_else(|| DownloadError::InvalidUrl("No assets found in release".into()))?
        };

        Ok(DownloadInfo {
            url: asset.browser_download_url.clone().to_string(),
            filename: Some(asset.name.clone()),
            description: Some(format!(
                "{}/{} - Release {} ({})",
                owner,
                repo,
                release.tag_name,
                release.name.unwrap_or("Unknown".to_string())
            )),
            auth_token: None,
        })
    }
    fn site_id(&self) -> &str {
        "github"
    }
}

impl Default for SpecializedDownloaderManager {
    fn default() -> Self {
        Self::new()
    }
}