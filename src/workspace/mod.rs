use async_trait::async_trait;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::VfsError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRecord {
    pub id: Uuid,
    pub name: String,
    pub root_path: String,
    pub head_commit: Option<String>,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceTokenRecord {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub agent_token: String,
    pub secret_hash: String,
}

#[derive(Debug, Clone)]
pub struct IssuedWorkspaceToken {
    pub token: WorkspaceTokenRecord,
    pub raw_secret: String,
}

#[derive(Debug, Clone)]
pub struct ValidWorkspaceToken {
    pub workspace: WorkspaceRecord,
    pub token: WorkspaceTokenRecord,
}

#[async_trait]
pub trait WorkspaceMetadataStore: Send + Sync {
    async fn list_workspaces(&self) -> Result<Vec<WorkspaceRecord>, VfsError>;
    async fn create_workspace(&self, name: &str, root_path: &str) -> Result<WorkspaceRecord, VfsError>;
    async fn get_workspace(&self, id: Uuid) -> Result<Option<WorkspaceRecord>, VfsError>;
    async fn update_head_commit(&self, id: Uuid, head_commit: Option<String>) -> Result<Option<WorkspaceRecord>, VfsError>;
    async fn issue_workspace_token(
        &self,
        workspace_id: Uuid,
        name: &str,
        agent_token: &str,
    ) -> Result<IssuedWorkspaceToken, VfsError>;
    async fn validate_workspace_token(
        &self,
        workspace_id: Uuid,
        raw_secret: &str,
    ) -> Result<Option<ValidWorkspaceToken>, VfsError>;
}

#[derive(Default)]
pub struct InMemoryWorkspaceMetadataStore {
    inner: RwLock<WorkspaceMetadataState>,
}

#[derive(Default)]
struct WorkspaceMetadataState {
    workspaces: HashMap<Uuid, WorkspaceRecord>,
    tokens: HashMap<Uuid, Vec<WorkspaceTokenRecord>>,
}

impl InMemoryWorkspaceMetadataStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn hash_secret(raw_secret: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(raw_secret.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn generate_secret() -> String {
        let mut bytes = [0u8; 24];
        rand::thread_rng().fill_bytes(&mut bytes);
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}

#[async_trait]
impl WorkspaceMetadataStore for InMemoryWorkspaceMetadataStore {
    async fn list_workspaces(&self) -> Result<Vec<WorkspaceRecord>, VfsError> {
        let guard = self.inner.read().await;
        let mut items: Vec<_> = guard.workspaces.values().cloned().collect();
        items.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(items)
    }

    async fn create_workspace(&self, name: &str, root_path: &str) -> Result<WorkspaceRecord, VfsError> {
        let mut guard = self.inner.write().await;
        let record = WorkspaceRecord {
            id: Uuid::new_v4(),
            name: name.to_string(),
            root_path: root_path.to_string(),
            head_commit: None,
            version: 0,
        };
        guard.workspaces.insert(record.id, record.clone());
        Ok(record)
    }

    async fn get_workspace(&self, id: Uuid) -> Result<Option<WorkspaceRecord>, VfsError> {
        let guard = self.inner.read().await;
        Ok(guard.workspaces.get(&id).cloned())
    }

    async fn update_head_commit(
        &self,
        id: Uuid,
        head_commit: Option<String>,
    ) -> Result<Option<WorkspaceRecord>, VfsError> {
        let mut guard = self.inner.write().await;
        let Some(workspace) = guard.workspaces.get_mut(&id) else {
            return Ok(None);
        };
        workspace.head_commit = head_commit;
        workspace.version += 1;
        Ok(Some(workspace.clone()))
    }

    async fn issue_workspace_token(
        &self,
        workspace_id: Uuid,
        name: &str,
        agent_token: &str,
    ) -> Result<IssuedWorkspaceToken, VfsError> {
        let mut guard = self.inner.write().await;
        if !guard.workspaces.contains_key(&workspace_id) {
            return Err(VfsError::AuthError {
                message: format!("unknown workspace: {workspace_id}"),
            });
        }

        let raw_secret = Self::generate_secret();
        let token = WorkspaceTokenRecord {
            id: Uuid::new_v4(),
            workspace_id,
            name: name.to_string(),
            agent_token: agent_token.to_string(),
            secret_hash: Self::hash_secret(&raw_secret),
        };
        guard
            .tokens
            .entry(workspace_id)
            .or_default()
            .push(token.clone());
        Ok(IssuedWorkspaceToken { token, raw_secret })
    }

    async fn validate_workspace_token(
        &self,
        workspace_id: Uuid,
        raw_secret: &str,
    ) -> Result<Option<ValidWorkspaceToken>, VfsError> {
        let guard = self.inner.read().await;
        let Some(workspace) = guard.workspaces.get(&workspace_id).cloned() else {
            return Ok(None);
        };
        let expected = Self::hash_secret(raw_secret);
        let Some(token) = guard
            .tokens
            .get(&workspace_id)
            .and_then(|tokens| tokens.iter().find(|token| token.secret_hash == expected))
            .cloned()
        else {
            return Ok(None);
        };
        Ok(Some(ValidWorkspaceToken { workspace, token }))
    }
}

pub type SharedWorkspaceMetadataStore = Arc<dyn WorkspaceMetadataStore>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn issues_and_validates_workspace_tokens() {
        let store = InMemoryWorkspaceMetadataStore::new();
        let workspace = store
            .create_workspace("demo", "/incidents/demo")
            .await
            .unwrap();
        let issued = store
            .issue_workspace_token(workspace.id, "demo-token", "agent-secret")
            .await
            .unwrap();

        let valid = store
            .validate_workspace_token(workspace.id, &issued.raw_secret)
            .await
            .unwrap()
            .expect("token should validate");

        assert_eq!(valid.workspace.id, workspace.id);
        assert_eq!(valid.token.agent_token, "agent-secret");
    }
}
