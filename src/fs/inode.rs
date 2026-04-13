use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub type InodeId = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inode {
    pub id: InodeId,
    pub kind: InodeKind,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
    pub created: u64,
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
        let now = now_epoch();
        Inode {
            id,
            kind: InodeKind::Directory {
                entries: BTreeMap::new(),
            },
            mode: 0o755,
            uid,
            gid,
            created: now,
            modified: now,
        }
    }

    pub fn new_file(id: InodeId, uid: u32, gid: u32) -> Self {
        let now = now_epoch();
        Inode {
            id,
            kind: InodeKind::File {
                content: Vec::new(),
            },
            mode: 0o644,
            uid,
            gid,
            created: now,
            modified: now,
        }
    }

    pub fn new_symlink(id: InodeId, target: String, uid: u32, gid: u32) -> Self {
        let now = now_epoch();
        Inode {
            id,
            kind: InodeKind::Symlink { target },
            mode: 0o777,
            uid,
            gid,
            created: now,
            modified: now,
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
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
