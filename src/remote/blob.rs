use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_config::Region;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use std::path::{Path, PathBuf};

use crate::error::VfsError;

#[derive(Debug, Clone)]
pub struct R2BlobStoreConfig {
    pub bucket: String,
    pub endpoint: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub region: String,
    pub prefix: String,
}

impl R2BlobStoreConfig {
    pub fn from_env() -> Option<Self> {
        let bucket = std::env::var("MARKDOWNFS_R2_BUCKET").ok()?;
        let endpoint = std::env::var("MARKDOWNFS_R2_ENDPOINT").ok()?;
        let access_key_id = std::env::var("MARKDOWNFS_R2_ACCESS_KEY_ID").ok()?;
        let secret_access_key = std::env::var("MARKDOWNFS_R2_SECRET_ACCESS_KEY").ok()?;
        let region = std::env::var("MARKDOWNFS_R2_REGION").unwrap_or_else(|_| "auto".to_string());
        let prefix = std::env::var("MARKDOWNFS_R2_PREFIX").unwrap_or_else(|_| "markdownfs".to_string());
        Some(Self {
            bucket,
            endpoint,
            access_key_id,
            secret_access_key,
            region,
            prefix,
        })
    }
}

#[async_trait]
pub trait RemoteBlobStore: Send + Sync {
    async fn put_bytes(&self, key: &str, data: Vec<u8>) -> Result<(), VfsError>;
    async fn get_bytes(&self, key: &str) -> Result<Vec<u8>, VfsError>;

    async fn put_content_blob(&self, hash: &str, data: Vec<u8>) -> Result<(), VfsError> {
        self.put_bytes(&format!("blobs/{hash}"), data).await
    }

    async fn put_commit_object(&self, hash: &str, data: Vec<u8>) -> Result<(), VfsError> {
        self.put_bytes(&format!("commits/{hash}.bin"), data).await
    }

    async fn put_snapshot_bundle(&self, name: &str, data: Vec<u8>) -> Result<(), VfsError> {
        self.put_bytes(&format!("snapshots/{name}.bin"), data).await
    }
}

pub struct LocalBlobStore {
    base_dir: PathBuf,
}

impl LocalBlobStore {
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    fn key_path(&self, key: &str) -> PathBuf {
        self.base_dir.join(key)
    }
}

#[async_trait]
impl RemoteBlobStore for LocalBlobStore {
    async fn put_bytes(&self, key: &str, data: Vec<u8>) -> Result<(), VfsError> {
        let path = self.key_path(key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, data).await?;
        Ok(())
    }

    async fn get_bytes(&self, key: &str) -> Result<Vec<u8>, VfsError> {
        Ok(tokio::fs::read(self.key_path(key)).await?)
    }
}

pub struct R2BlobStore {
    client: Client,
    bucket: String,
    prefix: String,
}

impl R2BlobStore {
    pub async fn new(config: R2BlobStoreConfig) -> Result<Self, VfsError> {
        let credentials = Credentials::new(
            config.access_key_id,
            config.secret_access_key,
            None,
            None,
            "markdownfs-r2",
        );

        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .endpoint_url(config.endpoint)
            .region(Region::new(config.region))
            .credentials_provider(credentials)
            .load()
            .await;

        Ok(Self {
            client: Client::new(&shared_config),
            bucket: config.bucket,
            prefix: config.prefix.trim_matches('/').to_string(),
        })
    }

    fn object_key(&self, key: &str) -> String {
        if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}/{}", self.prefix, key)
        }
    }
}

#[async_trait]
impl RemoteBlobStore for R2BlobStore {
    async fn put_bytes(&self, key: &str, data: Vec<u8>) -> Result<(), VfsError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(self.object_key(key))
            .body(ByteStream::from(data))
            .send()
            .await
            .map_err(|e| VfsError::IoError(std::io::Error::other(e.to_string())))?;
        Ok(())
    }

    async fn get_bytes(&self, key: &str) -> Result<Vec<u8>, VfsError> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(self.object_key(key))
            .send()
            .await
            .map_err(|e| VfsError::IoError(std::io::Error::other(e.to_string())))?;

        let bytes = output
            .body
            .collect()
            .await
            .map_err(|e| VfsError::IoError(std::io::Error::other(e.to_string())))?;
        Ok(bytes.into_bytes().to_vec())
    }
}
