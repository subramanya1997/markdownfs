#![cfg(feature = "fuser")]

use crate::auth::session::Session;
use crate::fs::inode::InodeKind;
use crate::fs::VirtualFs;
use crate::posix::{PosixFs, PosixSetAttr};
use fuser::{
    BsdFileFlags, Config, Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags,
    Generation, INodeNo, KernelConfig, MountOption, OpenAccMode, OpenFlags, RenameFlags,
    ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry, ReplyOpen,
    ReplyWrite, Request, TimeOrNow, WriteFlags,
};
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TTL: Duration = Duration::from_secs(1);

pub struct MarkdownFsFuse {
    fs: Arc<Mutex<VirtualFs>>,
}

impl MarkdownFsFuse {
    pub fn new(fs: Arc<Mutex<VirtualFs>>) -> Self {
        Self { fs }
    }

    pub fn mount_config(read_only: bool) -> Config {
        let mut mount_options = vec![
            MountOption::FSName("markdownfs".to_string()),
            MountOption::Subtype("markdownfs".to_string()),
            MountOption::DefaultPermissions,
        ];
        if read_only {
            mount_options.push(MountOption::RO);
        } else {
            mount_options.push(MountOption::RW);
        }
        let mut config = Config::default();
        config.mount_options = mount_options;
        config
    }

    fn session_for(req: &Request) -> Session {
        Session::new(req.uid(), req.gid(), vec![req.gid()], format!("uid-{}", req.uid()))
    }

    fn path_for_inode(fs: &VirtualFs, ino: INodeNo) -> Option<String> {
        if ino.0 == fs.root_id() {
            return Some("/".to_string());
        }
        fn walk(
            fs: &VirtualFs,
            current: u64,
            current_path: &str,
            target: INodeNo,
        ) -> Option<String> {
            let inode = fs.get_inode(current).ok()?;
            let InodeKind::Directory { entries } = &inode.kind else {
                return None;
            };

            for (name, child) in entries {
                let child_path = if current_path == "/" {
                    format!("/{name}")
                } else {
                    format!("{current_path}/{name}")
                };
                if *child == target.0 {
                    return Some(child_path);
                }
                if let Some(found) = walk(fs, *child, &child_path, target) {
                    return Some(found);
                }
            }
            None
        }

        walk(fs, fs.root_id(), "/", ino)
    }

    fn child_path(fs: &VirtualFs, parent: INodeNo, name: &OsStr) -> Option<String> {
        let parent_path = Self::path_for_inode(fs, parent)?;
        let name = name.to_str()?;
        Some(if parent_path == "/" {
            format!("/{name}")
        } else {
            format!("{parent_path}/{name}")
        })
    }

}

impl Filesystem for MarkdownFsFuse {
    fn init(&mut self, _req: &Request, _config: &mut KernelConfig) -> io::Result<()> {
        Ok(())
    }

    fn lookup(&self, req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let mut guard = self.fs.lock().expect("fuse mutex poisoned");
        let Some(path) = Self::child_path(&guard, parent, name) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let session = Self::session_for(req);
        let posix = PosixFs::new(&mut guard, &session);
        match posix.lookup(&path).map(|stat| stat_to_attr(&stat)) {
            Ok(attr) => reply.entry(&TTL, &attr, Generation(0)),
            Err(err) => reply.error(map_error(err)),
        }
    }

    fn getattr(&self, req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        let mut guard = self.fs.lock().expect("fuse mutex poisoned");
        let Some(path) = Self::path_for_inode(&guard, ino) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let session = Self::session_for(req);
        let posix = PosixFs::new(&mut guard, &session);
        match posix.getattr(&path).map(|stat| stat_to_attr(&stat)) {
            Ok(attr) => reply.attr(&TTL, &attr),
            Err(err) => reply.error(map_error(err)),
        }
    }

    fn readlink(&self, req: &Request, ino: INodeNo, reply: ReplyData) {
        let mut guard = self.fs.lock().expect("fuse mutex poisoned");
        let Some(path) = Self::path_for_inode(&guard, ino) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let session = Self::session_for(req);
        let posix = PosixFs::new(&mut guard, &session);
        match posix.readlink(&path) {
            Ok(target) => reply.data(target.as_bytes()),
            Err(err) => reply.error(map_error(err)),
        }
    }

    fn mkdir(
        &self,
        req: &Request,
        parent: INodeNo,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        let mut guard = self.fs.lock().expect("fuse mutex poisoned");
        let Some(path) = Self::child_path(&guard, parent, name) else {
            reply.error(Errno::EINVAL);
            return;
        };
        let session = Self::session_for(req);
        let mut posix = PosixFs::new(&mut guard, &session);
        match posix
            .mkdir(&path, mode as u16)
            .and_then(|_| posix.getattr(&path))
            .map(|stat| stat_to_attr(&stat))
        {
            Ok(attr) => reply.entry(&TTL, &attr, Generation(0)),
            Err(err) => reply.error(map_error(err)),
        }
    }

    fn create(
        &self,
        req: &Request,
        parent: INodeNo,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        let mut guard = self.fs.lock().expect("fuse mutex poisoned");
        let Some(path) = Self::child_path(&guard, parent, name) else {
            reply.error(Errno::EINVAL);
            return;
        };
        let session = Self::session_for(req);
        let mut posix = PosixFs::new(&mut guard, &session);
        match posix.create(&path, mode as u16).and_then(|_| posix.getattr(&path)) {
            Ok(stat) => reply.created(
                &TTL,
                &stat_to_attr(&stat),
                Generation(0),
                FileHandle(0),
                FopenFlags::empty(),
            ),
            Err(err) => reply.error(map_error(err)),
        }
    }

    fn open(&self, req: &Request, ino: INodeNo, flags: OpenFlags, reply: ReplyOpen) {
        let mut guard = self.fs.lock().expect("fuse mutex poisoned");
        let Some(path) = Self::path_for_inode(&guard, ino) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let writable = matches!(flags.acc_mode(), OpenAccMode::O_WRONLY | OpenAccMode::O_RDWR);
        let session = Self::session_for(req);
        let mut posix = PosixFs::new(&mut guard, &session);
        match posix.open(&path, writable) {
            Ok(_fh) => reply.opened(FileHandle(0), FopenFlags::empty()),
            Err(err) => reply.error(map_error(err)),
        }
    }

    fn read(
        &self,
        req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<fuser::LockOwner>,
        reply: ReplyData,
    ) {
        let mut guard = self.fs.lock().expect("fuse mutex poisoned");
        let Some(path) = Self::path_for_inode(&guard, ino) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let session = Self::session_for(req);
        let mut posix = PosixFs::new(&mut guard, &session);
        match posix.read(&path, offset as usize, size as usize) {
            Ok(data) => reply.data(&data),
            Err(err) => reply.error(map_error(err)),
        }
    }

    fn write(
        &self,
        req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        data: &[u8],
        _write_flags: WriteFlags,
        _flags: OpenFlags,
        _lock_owner: Option<fuser::LockOwner>,
        reply: ReplyWrite,
    ) {
        let mut guard = self.fs.lock().expect("fuse mutex poisoned");
        let Some(path) = Self::path_for_inode(&guard, ino) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let session = Self::session_for(req);
        let mut posix = PosixFs::new(&mut guard, &session);
        match posix.write(&path, offset as usize, data) {
            Ok(written) => reply.written(written as u32),
            Err(err) => reply.error(map_error(err)),
        }
    }

    fn release(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _fh: FileHandle,
        _flags: OpenFlags,
        _lock_owner: Option<fuser::LockOwner>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        reply.ok();
    }

    fn unlink(&self, req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let mut guard = self.fs.lock().expect("fuse mutex poisoned");
        let Some(path) = Self::child_path(&guard, parent, name) else {
            reply.error(Errno::EINVAL);
            return;
        };
        let session = Self::session_for(req);
        let mut posix = PosixFs::new(&mut guard, &session);
        match posix.unlink(&path) {
            Ok(()) => reply.ok(),
            Err(err) => reply.error(map_error(err)),
        }
    }

    fn rmdir(&self, req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let mut guard = self.fs.lock().expect("fuse mutex poisoned");
        let Some(path) = Self::child_path(&guard, parent, name) else {
            reply.error(Errno::EINVAL);
            return;
        };
        let session = Self::session_for(req);
        let mut posix = PosixFs::new(&mut guard, &session);
        match posix.rmdir(&path) {
            Ok(()) => reply.ok(),
            Err(err) => reply.error(map_error(err)),
        }
    }

    fn rename(
        &self,
        req: &Request,
        parent: INodeNo,
        name: &OsStr,
        newparent: INodeNo,
        newname: &OsStr,
        _flags: RenameFlags,
        reply: ReplyEmpty,
    ) {
        let mut guard = self.fs.lock().expect("fuse mutex poisoned");
        let Some(src) = Self::child_path(&guard, parent, name) else {
            reply.error(Errno::EINVAL);
            return;
        };
        let Some(dst) = Self::child_path(&guard, newparent, newname) else {
            reply.error(Errno::EINVAL);
            return;
        };
        let session = Self::session_for(req);
        let mut posix = PosixFs::new(&mut guard, &session);
        match posix.rename(&src, &dst) {
            Ok(()) => reply.ok(),
            Err(err) => reply.error(map_error(err)),
        }
    }

    fn symlink(
        &self,
        req: &Request,
        parent: INodeNo,
        link_name: &OsStr,
        target: &Path,
        reply: ReplyEntry,
    ) {
        let mut guard = self.fs.lock().expect("fuse mutex poisoned");
        let Some(path) = Self::child_path(&guard, parent, link_name) else {
            reply.error(Errno::EINVAL);
            return;
        };
        let target = target.to_string_lossy().into_owned();
        let session = Self::session_for(req);
        let mut posix = PosixFs::new(&mut guard, &session);
        match posix
            .symlink(&target, &path)
            .and_then(|_| posix.getattr(&path))
            .map(|stat| stat_to_attr(&stat))
        {
            Ok(attr) => reply.entry(&TTL, &attr, Generation(0)),
            Err(err) => reply.error(map_error(err)),
        }
    }

    fn link(
        &self,
        req: &Request,
        ino: INodeNo,
        newparent: INodeNo,
        newname: &OsStr,
        reply: ReplyEntry,
    ) {
        let mut guard = self.fs.lock().expect("fuse mutex poisoned");
        let Some(target) = Self::path_for_inode(&guard, ino) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let Some(path) = Self::child_path(&guard, newparent, newname) else {
            reply.error(Errno::EINVAL);
            return;
        };
        let session = Self::session_for(req);
        let mut posix = PosixFs::new(&mut guard, &session);
        match posix
            .link(&target, &path)
            .and_then(|_| posix.getattr(&path))
            .map(|stat| stat_to_attr(&stat))
        {
            Ok(attr) => reply.entry(&TTL, &attr, Generation(0)),
            Err(err) => reply.error(map_error(err)),
        }
    }

    fn setattr(
        &self,
        req: &Request,
        ino: INodeNo,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<TimeOrNow>,
        _mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<FileHandle>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<BsdFileFlags>,
        reply: ReplyAttr,
    ) {
        let mut guard = self.fs.lock().expect("fuse mutex poisoned");
        let Some(path) = Self::path_for_inode(&guard, ino) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let session = Self::session_for(req);
        let mut posix = PosixFs::new(&mut guard, &session);
        match posix
            .setattr(
                &path,
                PosixSetAttr {
                    mode: mode.map(|mode| mode as u16),
                    uid,
                    gid,
                    size: size.map(|size| size as usize),
                },
            )
            .map(|stat| stat_to_attr(&stat))
        {
            Ok(attr) => reply.attr(&TTL, &attr),
            Err(err) => reply.error(map_error(err)),
        }
    }

    fn readdir(
        &self,
        req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        mut reply: ReplyDirectory,
    ) {
        let mut guard = self.fs.lock().expect("fuse mutex poisoned");
        let Some(path) = Self::path_for_inode(&guard, ino) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let session = Self::session_for(req);
        let posix = PosixFs::new(&mut guard, &session);
        match posix.readdir(&path) {
            Ok(entries) => {
                if offset == 0 {
                    let _ = reply.add(ino, 1, FileType::Directory, ".");
                    let parent_ino = if path == "/" {
                        ino
                    } else {
                        let parent_path = Path::new(&path)
                            .parent()
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "/".to_string());
                        let parent_path = if parent_path.is_empty() {
                            "/".to_string()
                        } else {
                            parent_path
                        };
                        guard
                            .stat(&parent_path)
                            .map(|stat| INodeNo(stat.inode_id))
                            .unwrap_or(ino)
                    };
                    let _ = reply.add(parent_ino, 2, FileType::Directory, "..");
                }
                let start = offset.saturating_sub(2) as usize;
                for (entry_index, entry) in entries.into_iter().enumerate().skip(start) {
                    let full = (entry_index + 3) as u64;
                    let full_type = file_type(entry.stat.kind);
                    if reply.add(INodeNo(entry.stat.inode_id), full, full_type, entry.name) {
                        break;
                    }
                }
                reply.ok();
            }
            Err(err) => reply.error(map_error(err)),
        }
    }
}

fn stat_to_attr(stat: &crate::fs::StatInfo) -> FileAttr {
    let atime = UNIX_EPOCH + Duration::new(stat.accessed, stat.accessed_nanos);
    let mtime = UNIX_EPOCH + Duration::new(stat.modified, stat.modified_nanos);
    let ctime = UNIX_EPOCH + Duration::new(stat.changed, stat.changed_nanos);
    let crtime = UNIX_EPOCH + Duration::new(stat.created, stat.created_nanos);

    FileAttr {
        ino: INodeNo(stat.inode_id),
        size: stat.size,
        blocks: stat.blocks,
        atime,
        mtime,
        ctime,
        crtime,
        kind: file_type(stat.kind),
        perm: stat.mode,
        nlink: stat.nlink as u32,
        uid: stat.uid,
        gid: stat.gid,
        rdev: 0,
        blksize: stat.block_size as u32,
        flags: 0,
    }
}

fn file_type(kind: &str) -> FileType {
    match kind {
        "directory" => FileType::Directory,
        "symlink" => FileType::Symlink,
        _ => FileType::RegularFile,
    }
}

fn map_error(err: crate::error::VfsError) -> Errno {
    match err {
        crate::error::VfsError::NotFound { .. } => Errno::ENOENT,
        crate::error::VfsError::PermissionDenied { .. } => Errno::EACCES,
        crate::error::VfsError::AlreadyExists { .. } => Errno::EEXIST,
        crate::error::VfsError::NotDirectory { .. } => Errno::ENOTDIR,
        crate::error::VfsError::IsDirectory { .. } => Errno::EISDIR,
        crate::error::VfsError::NotEmpty { .. } => Errno::ENOTEMPTY,
        _ => Errno::EINVAL,
    }
}

pub fn mount(fs: Arc<Mutex<VirtualFs>>, mountpoint: PathBuf, read_only: bool) -> Result<(), std::io::Error> {
    let config = MarkdownFsFuse::mount_config(read_only);
    fuser::mount2(MarkdownFsFuse::new(fs), mountpoint, &config)
}
