use crate::auth::registry::UserRegistry;
use crate::error::VfsError;
use crate::fs::inode::{Inode, InodeId};
use crate::fs::VirtualFs;
use crate::store::blob::BlobStore;
use crate::store::commit::CommitObject;
use crate::store::ObjectId;
use crate::vcs::Vcs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const VFS_DIR: &str = ".vfs";
const STATE_FILE: &str = "state.bin";
const VERSION: u32 = 2;

/// Complete persisted state of the VFS + VCS.
#[derive(Serialize, Deserialize)]
struct PersistedState {
    version: u32,
    fs_state: FsState,
    vcs_state: VcsState,
}

#[derive(Serialize, Deserialize)]
struct FsState {
    inodes: HashMap<InodeId, Inode>,
    root: InodeId,
    cwd: InodeId,
    next_id: InodeId,
    cwd_path: Vec<(String, InodeId)>,
    registry: UserRegistry,
}

#[derive(Serialize, Deserialize)]
struct VcsState {
    /// All objects in the blob store: (ObjectId bytes, kind byte, data)
    objects: Vec<(Vec<u8>, u8, Vec<u8>)>,
    head: Option<Vec<u8>>,
    commits: Vec<CommitObject>,
}

// ─── V1 legacy types for migration ───

#[derive(Deserialize)]
struct PersistedStateV1 {
    version: u32,
    fs_state: FsStateV1,
    vcs_state: VcsState,
}

#[derive(Deserialize)]
struct FsStateV1 {
    inodes: HashMap<InodeId, Inode>,
    root: InodeId,
    cwd: InodeId,
    next_id: InodeId,
    cwd_path: Vec<(String, InodeId)>,
    // No registry in V1
}

pub struct PersistManager {
    base_dir: PathBuf,
}

impl PersistManager {
    pub fn new(base_dir: &Path) -> Self {
        PersistManager {
            base_dir: base_dir.join(VFS_DIR),
        }
    }

    pub fn state_exists(&self) -> bool {
        self.base_dir.join(STATE_FILE).exists()
    }

    pub fn save(&self, vfs: &VirtualFs, vcs: &Vcs) -> Result<(), VfsError> {
        fs::create_dir_all(&self.base_dir)?;

        let fs_state = FsState {
            inodes: vfs.all_inodes().clone(),
            root: vfs.root_id(),
            cwd: vfs.cwd_id(),
            next_id: vfs.next_inode_id(),
            cwd_path: vfs.cwd_path_clone(),
            registry: vfs.registry.clone(),
        };

        let vcs_state = VcsState {
            objects: vcs.store.export_all(),
            head: vcs.head.map(|id| id.as_bytes().to_vec()),
            commits: vcs.commits.clone(),
        };

        let state = PersistedState {
            version: VERSION,
            fs_state,
            vcs_state,
        };

        let data = bincode::serialize(&state).map_err(|e| VfsError::CorruptStore {
            message: format!("serialization failed: {e}"),
        })?;

        let tmp_path = self.base_dir.join("state.tmp");
        let final_path = self.base_dir.join(STATE_FILE);

        // Atomic write: write to tmp, then rename
        fs::write(&tmp_path, &data)?;
        fs::rename(&tmp_path, &final_path)?;

        Ok(())
    }

    pub fn load(&self) -> Result<(VirtualFs, Vcs), VfsError> {
        let path = self.base_dir.join(STATE_FILE);
        let data = fs::read(&path)?;

        // Try V2 first
        if let Ok(state) = bincode::deserialize::<PersistedState>(&data) {
            if state.version == VERSION {
                return Self::load_v2(state);
            }
        }

        // Try V1 migration
        if let Ok(state) = bincode::deserialize::<PersistedStateV1>(&data) {
            if state.version == 1 {
                return Self::load_v1(state);
            }
        }

        Err(VfsError::CorruptStore {
            message: "failed to deserialize state (unknown version or corrupt data)".to_string(),
        })
    }

    fn load_v2(state: PersistedState) -> Result<(VirtualFs, Vcs), VfsError> {
        let vfs = VirtualFs::from_persisted(
            state.fs_state.inodes,
            state.fs_state.root,
            state.fs_state.cwd,
            state.fs_state.next_id,
            state.fs_state.cwd_path,
            state.fs_state.registry,
        );

        let vcs = Self::load_vcs(state.vcs_state)?;
        Ok((vfs, vcs))
    }

    fn load_v1(state: PersistedStateV1) -> Result<(VirtualFs, Vcs), VfsError> {
        // V1 inodes already have uid/gid=0 defaults from serde (they were added with defaults)
        // Create a fresh registry (root user only)
        let registry = UserRegistry::new();

        let vfs = VirtualFs::from_persisted(
            state.fs_state.inodes,
            state.fs_state.root,
            state.fs_state.cwd,
            state.fs_state.next_id,
            state.fs_state.cwd_path,
            registry,
        );

        let vcs = Self::load_vcs(state.vcs_state)?;
        Ok((vfs, vcs))
    }

    fn load_vcs(vcs_state: VcsState) -> Result<Vcs, VfsError> {
        let mut store = BlobStore::new();
        store.import_all(vcs_state.objects)?;

        let head = match vcs_state.head {
            Some(bytes) => {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                Some(ObjectId::from_raw(arr))
            }
            None => None,
        };

        Ok(Vcs {
            store,
            head,
            commits: vcs_state.commits,
        })
    }

    pub fn data_dir(&self) -> &Path {
        &self.base_dir
    }
}
