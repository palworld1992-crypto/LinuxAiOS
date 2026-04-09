//! Download manager – tải file từ HTTPS/IPFS, kiểm tra checksum

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tracing::{info, warn};

pub struct HostDownloadManager {
    download_dir: PathBuf,
}

impl Default for HostDownloadManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HostDownloadManager {
    pub fn new() -> Self {
        Self {
            download_dir: PathBuf::from("/var/lib/aios/downloads"),
        }
    }

    pub async fn download_file(&self, url: &str, expected_sha256: Option<&str>) -> Result<PathBuf> {
        info!("Downloading file from {}", url);
        let response = reqwest::get(url).await?;
        let bytes = response.bytes().await?;
        if let Some(expected) = expected_sha256 {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let actual = hex::encode(hasher.finalize());
            if actual != expected {
                return Err(anyhow!(
                    "SHA256 mismatch: expected {}, got {}",
                    expected,
                    actual
                ));
            }
        }
        let filename = match url.split('/').next_back() {
    Some(name) => name,
    None => {
        warn!(url = %url, "Could not extract filename from URL, using default");
        "download"
    }
};
        let path = self.download_dir.join(filename);
        std::fs::write(&path, bytes)?;
        Ok(path)
    }
}
