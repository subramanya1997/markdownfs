use crate::auth::perms::Access;
use crate::auth::session::Session;
use crate::auth::{Gid, Uid};
use crate::error::VfsError;
use crate::fs::{HandleId, LsEntry, StatInfo, VirtualFs};

#[derive(Debug, Clone)]
pub struct PosixDirEntry {
    pub name: String,
    pub stat: StatInfo,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PosixSetAttr {
    pub mode: Option<u16>,
    pub uid: Option<Uid>,
    pub gid: Option<Gid>,
    pub size: Option<usize>,
}

pub struct PosixFs<'a> {
    fs: &'a mut VirtualFs,
    session: &'a Session,
}

impl<'a> PosixFs<'a> {
    pub fn new(fs: &'a mut VirtualFs, session: &'a Session) -> Self {
        Self { fs, session }
    }

    pub fn getattr(&self, path: &str) -> Result<StatInfo, VfsError> {
        let _ = self.fs.resolve_path_checked(path, self.session)?;
        self.fs.stat(path)
    }

    pub fn lookup(&self, path: &str) -> Result<StatInfo, VfsError> {
        self.getattr(path)
    }

    pub fn opendir(&mut self, path: &str) -> Result<HandleId, VfsError> {
        let dir_id = self.fs.resolve_path_checked(path, self.session)?;
        require_access(self.fs, dir_id, self.session, Access::Read, path)?;
        require_access(self.fs, dir_id, self.session, Access::Execute, path)?;
        self.fs.opendir(path)
    }

    pub fn readdir(&self, path: &str) -> Result<Vec<PosixDirEntry>, VfsError> {
        let dir_id = self.fs.resolve_path_checked(path, self.session)?;
        require_access(self.fs, dir_id, self.session, Access::Read, path)?;
        require_access(self.fs, dir_id, self.session, Access::Execute, path)?;

        let entries = self.fs.ls(Some(path))?;
        let mut results = Vec::new();
        for entry in entries {
            if !self.can_see(&entry) {
                continue;
            }
            let child_path = join_paths(path, &entry.name);
            results.push(PosixDirEntry {
                name: entry.name,
                stat: self.fs.stat(&child_path)?,
            });
        }
        Ok(results)
    }

    pub fn open(&mut self, path: &str, writable: bool) -> Result<HandleId, VfsError> {
        let inode_id = self.fs.resolve_path_checked(path, self.session)?;
        let access = if writable { Access::Write } else { Access::Read };
        require_access(self.fs, inode_id, self.session, access, path)?;
        self.fs.open(path, writable)
    }

    pub fn create(&mut self, path: &str, mode: u16) -> Result<HandleId, VfsError> {
        let (parent_id, _) = self.fs.resolve_parent_checked(path, self.session)?;
        require_access(self.fs, parent_id, self.session, Access::Write, path)?;
        require_access(self.fs, parent_id, self.session, Access::Execute, path)?;
        self.fs.create_file(
            path,
            self.session.effective_uid(),
            self.session.effective_gid(),
            Some(mode),
        )?;
        self.fs.open(path, true)
    }

    pub fn mkdir(&mut self, path: &str, mode: u16) -> Result<(), VfsError> {
        let (parent_id, _) = self.fs.resolve_parent_checked(path, self.session)?;
        require_access(self.fs, parent_id, self.session, Access::Write, path)?;
        require_access(self.fs, parent_id, self.session, Access::Execute, path)?;
        self.fs
            .mkdir(path, self.session.effective_uid(), self.session.effective_gid())?;
        self.fs.chmod(path, mode)
    }

    pub fn read(&mut self, path: &str, offset: usize, size: usize) -> Result<Vec<u8>, VfsError> {
        let inode_id = self.fs.resolve_path_checked(path, self.session)?;
        require_access(self.fs, inode_id, self.session, Access::Read, path)?;
        self.fs.read_file_at(path, offset, size)
    }

    pub fn write(&mut self, path: &str, offset: usize, data: &[u8]) -> Result<usize, VfsError> {
        let inode_id = self.fs.resolve_path_checked(path, self.session)?;
        require_access(self.fs, inode_id, self.session, Access::Write, path)?;
        self.fs.write_file_at(path, offset, data)
    }

    pub fn read_handle(&mut self, handle: HandleId, size: usize) -> Result<Vec<u8>, VfsError> {
        self.fs.read_handle(handle, size)
    }

    pub fn write_handle(&mut self, handle: HandleId, data: &[u8]) -> Result<usize, VfsError> {
        self.fs.write_handle(handle, data)
    }

    pub fn release(&mut self, handle: HandleId) -> Result<(), VfsError> {
        self.fs.release_handle(handle)
    }

    pub fn truncate(&mut self, path: &str, size: usize) -> Result<(), VfsError> {
        let inode_id = self.fs.resolve_path_checked(path, self.session)?;
        require_access(self.fs, inode_id, self.session, Access::Write, path)?;
        self.fs.truncate(path, size)
    }

    pub fn setattr(&mut self, path: &str, attr: PosixSetAttr) -> Result<StatInfo, VfsError> {
        let inode_id = self.fs.resolve_path_checked(path, self.session)?;
        let (current_uid, current_gid) = {
            let inode = self.fs.get_inode(inode_id)?;
            (inode.uid, inode.gid)
        };

        if let Some(mode) = attr.mode {
            if !self.session.is_effective_owner(current_uid) {
                return Err(VfsError::PermissionDenied {
                    path: path.to_string(),
                });
            }
            self.fs.chmod(path, mode)?;
        }

        if let Some(uid) = attr.uid {
            if !self.session.is_effectively_root() {
                return Err(VfsError::PermissionDenied {
                    path: path.to_string(),
                });
            }
            let gid = attr.gid.unwrap_or(current_gid);
            self.fs.chown(path, uid, gid)?;
        } else if let Some(gid) = attr.gid {
            if !self.session.is_effective_owner(current_uid) && !self.session.is_effectively_root() {
                return Err(VfsError::PermissionDenied {
                    path: path.to_string(),
                });
            }
            self.fs.chown(path, current_uid, gid)?;
        }

        if let Some(size) = attr.size {
            require_access(self.fs, inode_id, self.session, Access::Write, path)?;
            self.fs.truncate(path, size)?;
        }

        self.fs.stat(path)
    }

    pub fn unlink(&mut self, path: &str) -> Result<(), VfsError> {
        let (parent_id, _) = self.fs.resolve_parent_checked(path, self.session)?;
        require_access(self.fs, parent_id, self.session, Access::Write, path)?;
        require_access(self.fs, parent_id, self.session, Access::Execute, path)?;
        self.fs.rm(path)
    }

    pub fn rmdir(&mut self, path: &str) -> Result<(), VfsError> {
        let (parent_id, _) = self.fs.resolve_parent_checked(path, self.session)?;
        require_access(self.fs, parent_id, self.session, Access::Write, path)?;
        require_access(self.fs, parent_id, self.session, Access::Execute, path)?;
        self.fs.rmdir(path)
    }

    pub fn rename(&mut self, src: &str, dst: &str) -> Result<(), VfsError> {
        let (src_parent, _) = self.fs.resolve_parent_checked(src, self.session)?;
        require_access(self.fs, src_parent, self.session, Access::Write, src)?;
        require_access(self.fs, src_parent, self.session, Access::Execute, src)?;

        if let Ok((dst_parent, _)) = self.fs.resolve_parent_checked(dst, self.session) {
            require_access(self.fs, dst_parent, self.session, Access::Write, dst)?;
            require_access(self.fs, dst_parent, self.session, Access::Execute, dst)?;
        }

        self.fs.rename(src, dst)
    }

    pub fn symlink(&mut self, target: &str, link_path: &str) -> Result<(), VfsError> {
        let (parent_id, _) = self.fs.resolve_parent_checked(link_path, self.session)?;
        require_access(self.fs, parent_id, self.session, Access::Write, link_path)?;
        require_access(self.fs, parent_id, self.session, Access::Execute, link_path)?;
        self.fs.ln_s(
            target,
            link_path,
            self.session.effective_uid(),
            self.session.effective_gid(),
        )
    }

    pub fn link(&mut self, target: &str, link_path: &str) -> Result<(), VfsError> {
        let target_id = self.fs.resolve_path_checked(target, self.session)?;
        require_access(self.fs, target_id, self.session, Access::Read, target)?;
        let (parent_id, _) = self.fs.resolve_parent_checked(link_path, self.session)?;
        require_access(self.fs, parent_id, self.session, Access::Write, link_path)?;
        require_access(self.fs, parent_id, self.session, Access::Execute, link_path)?;
        self.fs.link(target, link_path)
    }

    pub fn readlink(&self, path: &str) -> Result<String, VfsError> {
        let inode_id = self.fs.resolve_path_checked(path, self.session)?;
        require_access(self.fs, inode_id, self.session, Access::Read, path)?;
        self.fs.readlink(path)
    }

    fn can_see(&self, entry: &LsEntry) -> bool {
        self.session
            .has_permission_bits(entry.mode, entry.uid, entry.gid, Access::Read)
    }
}

fn require_access(
    fs: &VirtualFs,
    inode_id: u64,
    session: &Session,
    access: Access,
    path: &str,
) -> Result<(), VfsError> {
    let inode = fs.get_inode(inode_id)?;
    if !session.has_permission(inode, access) {
        return Err(VfsError::PermissionDenied {
            path: path.to_string(),
        });
    }
    Ok(())
}

fn join_paths(parent: &str, child: &str) -> String {
    if parent == "/" {
        format!("/{child}")
    } else if parent == "." {
        format!("./{child}")
    } else {
        format!("{}/{}", parent.trim_end_matches('/'), child)
    }
}
