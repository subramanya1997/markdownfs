use crate::auth::registry::UserRegistry;
use crate::config::CompatibilityTarget;
use crate::error::VfsError;
use crate::fs::inode::{Inode, InodeId};
use crate::fs::{FsOptions, VirtualFs};
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
const VERSION: u32 = 3;

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
    compatibility_target: CompatibilityTarget,
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

#[derive(Deserialize)]
struct PersistedStateV2 {
    version: u32,
    fs_state: FsStateV2,
    vcs_state: VcsState,
}

#[derive(Deserialize)]
struct FsStateV2 {
    inodes: HashMap<InodeId, Inode>,
    root: InodeId,
    cwd: InodeId,
    next_id: InodeId,
    cwd_path: Vec<(String, InodeId)>,
    registry: UserRegistry,
}

pub struct PersistManager {
    base_dir: PathBuf,
}

pub struct LoadedState {
    pub fs: VirtualFs,
    pub vcs: Vcs,
}

#[derive(Debug, Clone)]
pub struct PersistenceInfo {
    pub backend: &'static str,
    pub location: Option<PathBuf>,
}

pub trait PersistenceBackend: Send + Sync {
    fn load(&self) -> Result<Option<LoadedState>, VfsError>;
    fn save(&self, vfs: &VirtualFs, vcs: &Vcs) -> Result<(), VfsError>;
    fn info(&self) -> PersistenceInfo;
}

pub struct LocalStateBackend {
    manager: PersistManager,
}

impl LocalStateBackend {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            manager: PersistManager::new(base_dir),
        }
    }
}

pub struct MemoryPersistenceBackend;

impl PersistenceBackend for LocalStateBackend {
    fn load(&self) -> Result<Option<LoadedState>, VfsError> {
        if !self.manager.state_exists() {
            return Ok(None);
        }
        let (fs, vcs) = self.manager.load()?;
        Ok(Some(LoadedState { fs, vcs }))
    }

    fn save(&self, vfs: &VirtualFs, vcs: &Vcs) -> Result<(), VfsError> {
        self.manager.save(vfs, vcs)
    }

    fn info(&self) -> PersistenceInfo {
        PersistenceInfo {
            backend: "local-state-file",
            location: Some(self.manager.data_dir().to_path_buf()),
        }
    }
}

impl PersistenceBackend for MemoryPersistenceBackend {
    fn load(&self) -> Result<Option<LoadedState>, VfsError> {
        Ok(None)
    }

    fn save(&self, _vfs: &VirtualFs, _vcs: &Vcs) -> Result<(), VfsError> {
        Ok(())
    }

    fn info(&self) -> PersistenceInfo {
        PersistenceInfo {
            backend: "memory",
            location: None,
        }
    }
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
            compatibility_target: vfs.compatibility_target(),
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

        // Try V3 first
        if let Ok(state) = bincode::deserialize::<PersistedState>(&data) {
            if state.version == VERSION {
                return Self::load_v3(state);
            }
        }

        // Try V2 migration
        if let Ok(state) = bincode::deserialize::<PersistedStateV2>(&data) {
            if state.version == 2 {
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

    fn load_v3(state: PersistedState) -> Result<(VirtualFs, Vcs), VfsError> {
        let vfs = VirtualFs::from_persisted(
            state.fs_state.inodes,
            state.fs_state.root,
            state.fs_state.cwd,
            state.fs_state.next_id,
            state.fs_state.cwd_path,
            FsOptions {
                compatibility_target: state.fs_state.compatibility_target,
            },
            state.fs_state.registry,
        );

        let vcs = Self::load_vcs(state.vcs_state)?;
        Ok((vfs, vcs))
    }

    fn load_v2(state: PersistedStateV2) -> Result<(VirtualFs, Vcs), VfsError> {
        let vfs = VirtualFs::from_persisted(
            state.fs_state.inodes,
            state.fs_state.root,
            state.fs_state.cwd,
            state.fs_state.next_id,
            state.fs_state.cwd_path,
            FsOptions::default(),
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
            FsOptions::default(),
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
