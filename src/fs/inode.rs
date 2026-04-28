use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub type InodeId = u64;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Timestamp {
    pub secs: u64,
    pub nanos: u32,
}

impl Timestamp {
    pub fn now() -> Self {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Self {
            secs: duration.as_secs(),
            nanos: duration.subsec_nanos(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inode {
    pub id: InodeId,
    pub kind: InodeKind,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
    #[serde(default = "default_nlink")]
    pub nlink: u64,
    #[serde(default = "default_block_size")]
    pub block_size: u64,
    #[serde(default)]
    pub created_at: Timestamp,
    #[serde(default)]
    pub modified_at: Timestamp,
    #[serde(default)]
    pub accessed_at: Timestamp,
    #[serde(default)]
    pub changed_at: Timestamp,
    #[serde(default)]
    pub created: u64,
    #[serde(default)]
    pub modified: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InodeKind {
    File { content: Vec<u8> },
    Directory { entries: BTreeMap<String, InodeId> },
    Symlink { target: String },
}

impl Inode {
    pub fn new_dir(id: InodeId, uid: u32, gid: u32) -> Self {
        let now = Timestamp::now();
        Inode {
            id,
            kind: InodeKind::Directory {
                entries: BTreeMap::new(),
            },
            mode: 0o755,
            uid,
            gid,
            nlink: 2,
            block_size: 4096,
            created_at: now,
            modified_at: now,
            accessed_at: now,
            changed_at: now,
            created: now.secs,
            modified: now.secs,
        }
    }

    pub fn new_file(id: InodeId, uid: u32, gid: u32) -> Self {
        let now = Timestamp::now();
        Inode {
            id,
            kind: InodeKind::File {
                content: Vec::new(),
            },
            mode: 0o644,
            uid,
            gid,
            nlink: 1,
            block_size: 4096,
            created_at: now,
            modified_at: now,
            accessed_at: now,
            changed_at: now,
            created: now.secs,
            modified: now.secs,
        }
    }

    pub fn new_symlink(id: InodeId, target: String, uid: u32, gid: u32) -> Self {
        let now = Timestamp::now();
        Inode {
            id,
            kind: InodeKind::Symlink { target },
            mode: 0o777,
            uid,
            gid,
            nlink: 1,
            block_size: 4096,
            created_at: now,
            modified_at: now,
            accessed_at: now,
            changed_at: now,
            created: now.secs,
            modified: now.secs,
        }
    }

    pub fn is_dir(&self) -> bool {
        matches!(self.kind, InodeKind::Directory { .. })
    }

    pub fn is_file(&self) -> bool {
        matches!(self.kind, InodeKind::File { .. })
    }

    pub fn size(&self) -> u64 {
        match &self.kind {
            InodeKind::File { content } => content.len() as u64,
            InodeKind::Directory { entries } => entries.len() as u64,
            InodeKind::Symlink { target } => target.len() as u64,
        }
    }

    pub fn blocks(&self) -> u64 {
        let size = self.size();
        if size == 0 {
            0
        } else {
            size.div_ceil(512)
        }
    }

    pub fn touch_access(&mut self) {
        let now = Timestamp::now();
        self.accessed_at = now;
    }

    pub fn touch_modify(&mut self) {
        let now = Timestamp::now();
        self.modified_at = now;
        self.changed_at = now;
        self.modified = now.secs;
    }

    pub fn touch_change(&mut self) {
        let now = Timestamp::now();
        self.changed_at = now;
    }
}

fn default_nlink() -> u64 {
    1
}

fn default_block_size() -> u64 {
    4096
}
