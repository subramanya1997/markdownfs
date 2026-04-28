use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use reqwest::Client;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::VfsError;

#[derive(Clone, Debug)]
pub enum ClientAuth {
    Root,
    User(String),
    Bearer(String),
    WorkspaceBearer {
        workspace_id: Uuid,
        secret: String,
    },
}

#[derive(Clone)]
pub struct MarkdownFsClient {
    base_url: String,
    client: Client,
    auth: ClientAuth,
}

#[derive(Debug, Deserialize)]
pub struct ClientLsResponse {
    pub path: String,
    pub entries: Vec<ClientLsEntry>,
}

#[derive(Debug, Deserialize)]
pub struct ClientLsEntry {
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientGrepResult {
    pub file: String,
    pub line_num: usize,
    pub line: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientGrepResponse {
    pub results: Vec<ClientGrepResult>,
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct ClientFindResponse {
    pub results: Vec<String>,
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct ClientCommit {
    pub hash: String,
    pub message: String,
    pub author: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientLogResponse {
    pub commits: Vec<ClientCommitLog>,
}

#[derive(Debug, Deserialize)]
pub struct ClientCommitLog {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: u64,
}

impl MarkdownFsClient {
    pub fn new(base_url: impl Into<String>, auth: ClientAuth) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: Client::new(),
            auth,
        }
    }

    fn headers(&self) -> Result<HeaderMap, VfsError> {
        let mut headers = HeaderMap::new();
        match &self.auth {
            ClientAuth::Root => {}
            ClientAuth::User(username) => {
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!("User {username}")).map_err(|e| {
                        VfsError::InvalidArgs {
                            message: format!("invalid username header: {e}"),
                        }
                    })?,
                );
            }
            ClientAuth::Bearer(token) => {
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!("Bearer {token}")).map_err(|e| {
                        VfsError::InvalidArgs {
                            message: format!("invalid bearer header: {e}"),
                        }
                    })?,
                );
            }
            ClientAuth::WorkspaceBearer {
                workspace_id,
                secret,
            } => {
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!("Bearer {secret}")).map_err(|e| {
                        VfsError::InvalidArgs {
                            message: format!("invalid workspace bearer header: {e}"),
                        }
                    })?,
                );
                headers.insert(
                    "x-markdownfs-workspace",
                    HeaderValue::from_str(&workspace_id.to_string()).map_err(|e| {
                        VfsError::InvalidArgs {
                            message: format!("invalid workspace header: {e}"),
                        }
                    })?,
                );
            }
        }
        Ok(headers)
    }

    pub async fn health(&self) -> Result<serde_json::Value, VfsError> {
        self.json(self.client.get(format!("{}/health", self.base_url)))
            .await
    }

    pub async fn list_directory(&self, path: &str) -> Result<ClientLsResponse, VfsError> {
        let path = path.trim_start_matches('/');
        let url = if path.is_empty() {
            format!("{}/fs", self.base_url)
        } else {
            format!("{}/fs/{}", self.base_url, path)
        };
        self.json(self.client.get(url)).await
    }

    pub async fn read_file(&self, path: &str) -> Result<String, VfsError> {
        let url = format!("{}/fs/{}", self.base_url, path.trim_start_matches('/'));
        let response = self
            .client
            .get(url)
            .headers(self.headers()?)
            .send()
            .await
            .map_err(|e| VfsError::IoError(std::io::Error::other(e.to_string())))?;
        Self::ensure_success(response.status(), response.text().await.unwrap_or_default())
    }

    pub async fn write_file(&self, path: &str, content: String) -> Result<serde_json::Value, VfsError> {
        let url = format!("{}/fs/{}", self.base_url, path.trim_start_matches('/'));
        self.json(
            self.client
                .put(url)
                .headers(self.headers()?)
                .body(content),
        )
        .await
    }

    pub async fn grep(&self, pattern: &str, path: Option<&str>) -> Result<ClientGrepResponse, VfsError> {
        let mut url = format!("{}/search/grep?pattern={}&recursive=true", self.base_url, urlencoding::encode(pattern));
        if let Some(path) = path {
            url.push_str("&path=");
            url.push_str(&urlencoding::encode(path));
        }
        self.json(self.client.get(url)).await
    }

    pub async fn find(&self, pattern: &str, path: Option<&str>) -> Result<ClientFindResponse, VfsError> {
        let mut url = format!("{}/search/find?name={}", self.base_url, urlencoding::encode(pattern));
        if let Some(path) = path {
            url.push_str("&path=");
            url.push_str(&urlencoding::encode(path));
        }
        self.json(self.client.get(url)).await
    }

    pub async fn tree(&self, path: Option<&str>) -> Result<String, VfsError> {
        let url = match path {
            Some(path) if !path.trim_matches('/').is_empty() => {
                format!("{}/tree/{}", self.base_url, path.trim_start_matches('/'))
            }
            _ => format!("{}/tree", self.base_url),
        };
        let response = self
            .client
            .get(url)
            .headers(self.headers()?)
            .send()
            .await
            .map_err(|e| VfsError::IoError(std::io::Error::other(e.to_string())))?;
        Self::ensure_success(response.status(), response.text().await.unwrap_or_default())
    }

    pub async fn commit(&self, message: &str) -> Result<ClientCommit, VfsError> {
        self.json(
            self.client
                .post(format!("{}/vcs/commit", self.base_url))
                .headers(self.headers()?)
                .json(&serde_json::json!({ "message": message })),
        )
        .await
    }

    pub async fn log(&self) -> Result<ClientLogResponse, VfsError> {
        self.json(self.client.get(format!("{}/vcs/log", self.base_url))).await
    }

    pub async fn revert(&self, hash: &str) -> Result<serde_json::Value, VfsError> {
        self.json(
            self.client
                .post(format!("{}/vcs/revert", self.base_url))
                .headers(self.headers()?)
                .json(&serde_json::json!({ "hash": hash })),
        )
        .await
    }

    pub async fn status(&self) -> Result<String, VfsError> {
        let response = self
            .client
            .get(format!("{}/vcs/status", self.base_url))
            .headers(self.headers()?)
            .send()
            .await
            .map_err(|e| VfsError::IoError(std::io::Error::other(e.to_string())))?;
        Self::ensure_success(response.status(), response.text().await.unwrap_or_default())
    }

    pub async fn list_workspaces(&self) -> Result<serde_json::Value, VfsError> {
        self.json(self.client.get(format!("{}/workspaces", self.base_url)))
            .await
    }

    pub async fn create_workspace(
        &self,
        name: &str,
        root_path: &str,
    ) -> Result<serde_json::Value, VfsError> {
        self.json(
            self.client
                .post(format!("{}/workspaces", self.base_url))
                .headers(self.headers()?)
                .json(&serde_json::json!({ "name": name, "root_path": root_path })),
        )
        .await
    }

    pub async fn issue_workspace_token(
        &self,
        workspace_id: Uuid,
        name: &str,
        agent_token: &str,
    ) -> Result<serde_json::Value, VfsError> {
        self.json(
            self.client
                .post(format!("{}/workspaces/{workspace_id}/tokens", self.base_url))
                .headers(self.headers()?)
                .json(&serde_json::json!({ "name": name, "agent_token": agent_token })),
        )
        .await
    }

    async fn json<T>(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> Result<T, VfsError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let response = builder
            .headers(self.headers()?)
            .send()
            .await
            .map_err(|e| VfsError::IoError(std::io::Error::other(e.to_string())))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| VfsError::IoError(std::io::Error::other(e.to_string())))?;
        if !status.is_success() {
            return Err(VfsError::InvalidArgs { message: body });
        }
        serde_json::from_str(&body).map_err(|e| VfsError::CorruptStore {
            message: format!("invalid server response: {e}"),
        })
    }

    fn ensure_success(status: reqwest::StatusCode, body: String) -> Result<String, VfsError> {
        if status.is_success() {
            Ok(body)
        } else {
            Err(VfsError::InvalidArgs { message: body })
        }
    }
}
