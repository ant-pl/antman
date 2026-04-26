use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use anyhow::{Context, Result};
use futures::{stream, StreamExt};
use reqwest::Client;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;

/// 多线程下载器配置
#[derive(Debug, Clone)]
pub struct DownloaderConfig {
    pub max_concurrent_downloads: usize,
    pub timeout: Duration,
    pub user_agent: Option<String>,
    pub retry_attempts: u32,
    pub retry_delay: Duration,
}

impl Default for DownloaderConfig {
    fn default() -> Self {
        Self {
            max_concurrent_downloads: 8,
            timeout: Duration::from_secs(30),
            user_agent: Some("Mozilla/5.0 (compatible; Downloader/1.0)".to_string()),
            retry_attempts: 5,
            retry_delay: Duration::from_secs(1),
        }
    }
}

/// 多线程下载器
#[derive(Debug)]
pub struct Downloader {
    client: Client,
    config: DownloaderConfig,
    semaphore: Arc<Semaphore>,
}

impl Downloader {
    /// 创建新的下载器实例
    pub fn new(config: DownloaderConfig) -> Result<Self> {
        let mut client_builder = Client::builder()
            .timeout(config.timeout)
            .pool_idle_timeout(Duration::from_secs(30))
            .tcp_keepalive(Duration::from_secs(60));
        
        if let Some(ua) = &config.user_agent {
            client_builder = client_builder.user_agent(ua);
        }
        
        let client = client_builder.build()?;
        
        Ok(Self {
            client,
            config: config.clone(),
            semaphore: Arc::new(Semaphore::new(config.max_concurrent_downloads)),
        })
    }
    
    /// 下载单个文件
    pub async fn download_file<P: AsRef<Path>>(
        &self,
        url: &str,
        destination: P,
    ) -> Result<PathBuf> {
        let destination = destination.as_ref().to_path_buf();
        
        // 确保目标目录存在
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        // 使用信号量限制并发数
        let _permit = self.semaphore.acquire().await?;
        
        // 带重试机制的下载
        let mut attempts = 0;
        loop {
            attempts += 1;
            
            match self.try_download(url, &destination).await {
                Ok(_) => return Ok(destination),
                Err(e) => {
                    if attempts >= self.config.retry_attempts {
                        return Err(e);
                    }
                    
                    tokio::time::sleep(self.config.retry_delay).await;
                }
            }
        }
    }
    
    /// 尝试下载文件（无重试）
    async fn try_download(&self, url: &str, destination: &Path) -> Result<()> {
        let response = self.client.get(url).send().await?;
        
        if !response.status().is_success() {
            anyhow::bail!("HTTP error: {}", response.status());
        }
        
        let mut file = File::create(destination).await?;
        let mut stream = response.bytes_stream();
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
        }
        
        file.flush().await?;
        Ok(())
    }
    
    /// 批量下载多个文件
    pub async fn download_multiple<P, I, S>(&self, downloads: I) -> Vec<Result<PathBuf>>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = (S, P)>,
        S: AsRef<str>,
    {
        let downloads: Vec<(String, PathBuf)> = downloads
            .into_iter()
            .map(|(url, path)| (url.as_ref().to_string(), path.as_ref().to_path_buf()))
            .collect();
        
        stream::iter(downloads)
            .map(|(url, path)| async move {
                self.download_file(&url, &path).await
            })
            .buffer_unordered(self.config.max_concurrent_downloads)
            .collect()
            .await
    }
    
    /// 获取文件大小（不下载整个文件）
    pub async fn get_content_length(&self, url: &str) -> Result<u64> {
        let response = self.client.head(url).send().await?;
        
        if !response.status().is_success() {
            anyhow::bail!("HTTP error: {}", response.status());
        }
        
        response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|s| s.parse().ok())
            .context("Could not determine content length")
    }
}