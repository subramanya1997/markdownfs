pub mod inode;

use crate::auth::perms::{has_setgid, Access};
use crate::auth::registry::UserRegistry;
use crate::auth::session::Session;
use crate::auth::{Gid, Uid, ROOT_GID, ROOT_UID};
use crate::error::VfsError;
use inode::{Inode, InodeId, InodeKind};
use std::collections::BTreeMap;
use std::collections::HashMap;

pub struct VirtualFs {
    inodes: HashMap<InodeId, Inode>,
    root: InodeId,
    cwd: InodeId,
    next_id: InodeId,
    cwd_path: Vec<(String, InodeId)>,
    pub registry: UserRegistry,
}

impl VirtualFs {
    pub fn new() -> Self {
        let root_id = 0;
        let root = Inode::new_dir(root_id, ROOT_UID, ROOT_GID);
        let mut inodes = HashMap::new();
        inodes.insert(root_id, root);
        VirtualFs {
            inodes,
            root: root_id,
            cwd: root_id,
            next_id: 1,
            cwd_path: Vec::new(),
            registry: UserRegistry::new(),
        }
    }

    fn alloc_id(&mut self) -> InodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn pwd(&self) -> String {
        if self.cwd_path.is_empty() {
            "/".to_string()
        } else {
            let mut path = String::new();
            for (name, _) in &self.cwd_path {
                path.push('/');
                path.push_str(name);
            }
            path
        }
    }

    pub fn get_inode(&self, id: InodeId) -> Result<&Inode, VfsError> {
        self.inodes.get(&id).ok_or_else(|| VfsError::NotFound {
            path: format!("<inode {id}>"),
        })
    }

    fn get_inode_mut(&mut self, id: InodeId) -> Result<&mut Inode, VfsError> {
        self.inodes.get_mut(&id).ok_or_else(|| VfsError::NotFound {
            path: format!("<inode {id}>"),
        })
    }

    fn dir_entries(&self, id: InodeId) -> Result<&BTreeMap<String, InodeId>, VfsError> {
        match &self.get_inode(id)?.kind {
            InodeKind::Directory { entries } => Ok(entries),
            _ => Err(VfsError::NotDirectory {
                path: format!("<inode {id}>"),
            }),
        }
    }

    fn dir_entries_mut(
        &mut self,
        id: InodeId,
    ) -> Result<&mut BTreeMap<String, InodeId>, VfsError> {
        match &mut self.get_inode_mut(id)?.kind {
            InodeKind::Directory { entries } => Ok(entries),
            _ => Err(VfsError::NotDirectory {
                path: format!("<inode {id}>"),
            }),
        }
    }

    /// Resolve a path without permission checks (for internal/VCS use).
    pub fn resolve_path(&self, path: &str) -> Result<InodeId, VfsError> {
        let (start, components) = self.parse_path(path);
        let mut current = start;
        for component in &components {
            match component.as_str() {
                "." => {}
                ".." => {
                    current = self.parent_of(current);
                }
                name => {
                    let entries = self.dir_entries(current)?;
                    current = *entries.get(name).ok_or_else(|| VfsError::NotFound {
                        path: path.to_string(),
                    })?;
                }
            }
        }
        Ok(current)
    }

    /// Resolve a path with Execute permission checks on every directory.
    pub fn resolve_path_checked(
        &self,
        path: &str,
        session: &Session,
    ) -> Result<InodeId, VfsError> {
        let (start, components) = self.parse_path(path);
        let mut current = start;

        // Check execute on starting directory
        let start_inode = self.get_inode(current)?;
        if !session.has_permission(start_inode, Access::Execute) {
            return Err(VfsError::PermissionDenied {
                path: path.to_string(),
            });
        }

        for component in &components {
            match component.as_str() {
                "." => {}
                ".." => {
                    current = self.parent_of(current);
                }
                name => {
                    let entries = self.dir_entries(current)?;
                    current = *entries.get(name).ok_or_else(|| VfsError::NotFound {
                        path: path.to_string(),
                    })?;
                    // Check execute on intermediate directories
                    let inode = self.get_inode(current)?;
                    if inode.is_dir() && !session.has_permission(inode, Access::Execute) {
                        return Err(VfsError::PermissionDenied {
                            path: path.to_string(),
                        });
                    }
                }
            }
        }
        Ok(current)
    }

    /// Resolve parent with permission checks.
    pub fn resolve_parent_checked(
        &self,
        path: &str,
        session: &Session,
    ) -> Result<(InodeId, String), VfsError> {
        let (start, components) = self.parse_path(path);
        if components.is_empty() {
            return Err(VfsError::InvalidPath {
                path: path.to_string(),
            });
        }
        let name = components.last().unwrap().clone();
        let parent_path = if components.len() == 1 {
            if path.starts_with('/') {
                "/".to_string()
            } else {
                ".".to_string()
            }
        } else {
            let parent_components = &components[..components.len() - 1];
            if path.starts_with('/') {
                format!("/{}", parent_components.join("/"))
            } else {
                parent_components.join("/")
            }
        };
        let _ = start; // used via parse_path in resolve_path_checked
        let parent_id = self.resolve_path_checked(&parent_path, session)?;
        Ok((parent_id, name))
    }

    fn resolve_parent(&self, path: &str) -> Result<(InodeId, String), VfsError> {
        let (start, components) = self.parse_path(path);
        if components.is_empty() {
            return Err(VfsError::InvalidPath {
                path: path.to_string(),
            });
        }
        let name = components.last().unwrap().clone();
        let mut current = start;
        for component in &components[..components.len() - 1] {
            match component.as_str() {
                "." => {}
                ".." => {
                    current = self.parent_of(current);
                }
                n => {
                    let entries = self.dir_entries(current)?;
                    current = *entries.get(n).ok_or_else(|| VfsError::NotFound {
                        path: path.to_string(),
                    })?;
                }
            }
        }
        Ok((current, name))
    }

    fn parse_path(&self, path: &str) -> (InodeId, Vec<String>) {
        let (start, path_str) = if path.starts_with('/') {
            (self.root, &path[1..])
        } else {
            (self.cwd, path)
        };
        let components: Vec<String> = path_str
            .split('/')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        (start, components)
    }

    fn parent_of(&self, id: InodeId) -> InodeId {
        if id == self.root {
            return self.root;
        }
        for i in (0..self.cwd_path.len()).rev() {
            if self.cwd_path[i].1 == id && i > 0 {
                return self.cwd_path[i - 1].1;
            }
        }
        for inode in self.inodes.values() {
            if let InodeKind::Directory { entries } = &inode.kind {
                if entries.values().any(|&child| child == id) {
                    return inode.id;
                }
            }
        }
        self.root
    }

    /// Determine the gid for a new child in this directory.
    /// If the parent has setgid, inherit parent's gid; otherwise use caller's gid.
    fn effective_gid(&self, parent_id: InodeId, caller_gid: Gid) -> Gid {
        if let Ok(parent) = self.get_inode(parent_id) {
            if has_setgid(parent.mode) {
                return parent.gid;
            }
        }
        caller_gid
    }

    // ───── Commands ─────

    pub fn ls(&self, path: Option<&str>) -> Result<Vec<LsEntry>, VfsError> {
        let id = match path {
            Some(p) => self.resolve_path(p)?,
            None => self.cwd,
        };
        let inode = self.get_inode(id)?;
        match &inode.kind {
            InodeKind::Directory { entries } => {
                let mut result = Vec::with_capacity(entries.len());
                for (name, &child_id) in entries {
                    let child = self.get_inode(child_id)?;
                    result.push(LsEntry {
                        name: name.clone(),
                        is_dir: child.is_dir(),
                        is_symlink: matches!(child.kind, InodeKind::Symlink { .. }),
                        size: child.size(),
                        mode: child.mode,
                        uid: child.uid,
                        gid: child.gid,
                        modified: child.modified,
                    });
                }
                Ok(result)
            }
            InodeKind::File { .. } => Ok(vec![LsEntry {
                name: path.unwrap_or("").to_string(),
                is_dir: false,
                is_symlink: false,
                size: inode.size(),
                mode: inode.mode,
                uid: inode.uid,
                gid: inode.gid,
                modified: inode.modified,
            }]),
            InodeKind::Symlink { .. } => Ok(vec![LsEntry {
                name: path.unwrap_or("").to_string(),
                is_dir: false,
                is_symlink: true,
                size: inode.size(),
                mode: inode.mode,
                uid: inode.uid,
                gid: inode.gid,
                modified: inode.modified,
            }]),
        }
    }

    pub fn cd(&mut self, path: &str) -> Result<(), VfsError> {
        if path == "/" {
            self.cwd = self.root;
            self.cwd_path.clear();
            return Ok(());
        }

        let target = self.resolve_path(path)?;
        let inode = self.get_inode(target)?;
        if !inode.is_dir() {
            return Err(VfsError::NotDirectory {
                path: path.to_string(),
            });
        }

        if path.starts_with('/') {
            self.cwd_path.clear();
        }

        for component in path.split('/').filter(|s| !s.is_empty()) {
            match component {
                "." => {}
                ".." => {
                    self.cwd_path.pop();
                }
                name => {
                    let current_dir = self
                        .cwd_path
                        .last()
                        .map(|(_, id)| *id)
                        .unwrap_or(self.root);
                    let entries = self.dir_entries(current_dir)?;
                    if let Some(&child_id) = entries.get(name) {
                        self.cwd_path.push((name.to_string(), child_id));
                    }
                }
            }
        }

        self.cwd = target;
        Ok(())
    }

    pub fn mkdir(&mut self, path: &str, uid: Uid, gid: Gid) -> Result<(), VfsError> {
        let (parent_id, name) = self.resolve_parent(path)?;

        let entries = self.dir_entries(parent_id)?;
        if entries.contains_key(&name) {
            return Err(VfsError::AlreadyExists {
                path: path.to_string(),
            });
        }

        let effective_gid = self.effective_gid(parent_id, gid);
        let new_id = self.alloc_id();
        let mut new_dir = Inode::new_dir(new_id, uid, effective_gid);

        // Inherit setgid bit from parent
        if let Ok(parent) = self.get_inode(parent_id) {
            if has_setgid(parent.mode) {
                new_dir.mode |= 0o2000;
            }
        }

        self.inodes.insert(new_id, new_dir);
        self.dir_entries_mut(parent_id)?.insert(name, new_id);
        Ok(())
    }

    pub fn mkdir_p(&mut self, path: &str, uid: Uid, gid: Gid) -> Result<(), VfsError> {
        let (start, components) = self.parse_path(path);
        let mut current = start;
        for component in &components {
            match component.as_str() {
                "." => {}
                ".." => {
                    current = self.parent_of(current);
                }
                name => {
                    let entries = self.dir_entries(current)?;
                    if let Some(&existing) = entries.get(name) {
                        if !self.get_inode(existing)?.is_dir() {
                            return Err(VfsError::NotDirectory {
                                path: name.to_string(),
                            });
                        }
                        current = existing;
                    } else {
                        let effective_gid = self.effective_gid(current, gid);
                        let new_id = self.alloc_id();
                        let new_dir = Inode::new_dir(new_id, uid, effective_gid);
                        self.inodes.insert(new_id, new_dir);
                        self.dir_entries_mut(current)?
                            .insert(name.to_string(), new_id);
                        current = new_id;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn touch(&mut self, path: &str, uid: Uid, gid: Gid) -> Result<(), VfsError> {
        validate_markdown_filename(path)?;

        if let Ok(id) = self.resolve_path(path) {
            let inode = self.get_inode_mut(id)?;
            inode.modified = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            return Ok(());
        }

        let (parent_id, name) = self.resolve_parent(path)?;
        let effective_gid = self.effective_gid(parent_id, gid);
        let new_id = self.alloc_id();
        let new_file = Inode::new_file(new_id, uid, effective_gid);
        self.inodes.insert(new_id, new_file);
        self.dir_entries_mut(parent_id)?.insert(name, new_id);
        Ok(())
    }

    pub fn cat(&self, path: &str) -> Result<&[u8], VfsError> {
        let id = self.resolve_path(path)?;
        let inode = self.get_inode(id)?;
        match &inode.kind {
            InodeKind::File { content } => Ok(content),
            InodeKind::Directory { .. } => Err(VfsError::IsDirectory {
                path: path.to_string(),
            }),
            InodeKind::Symlink { target } => self.cat(target),
        }
    }

    pub fn cat_owned(&self, path: &str) -> Result<Vec<u8>, VfsError> {
        self.cat(path).map(|b| b.to_vec())
    }

    pub fn write_file(&mut self, path: &str, content: Vec<u8>) -> Result<(), VfsError> {
        validate_markdown_filename(path)?;
        let id = self.resolve_path(path)?;
        let inode = self.get_inode_mut(id)?;
        match &mut inode.kind {
            InodeKind::File { content: c, .. } => {
                *c = content;
                inode.modified = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                Ok(())
            }
            InodeKind::Directory { .. } => Err(VfsError::IsDirectory {
                path: path.to_string(),
            }),
            InodeKind::Symlink { target } => {
                let target = target.clone();
                self.write_file(&target, content)
            }
        }
    }

    pub fn rm(&mut self, path: &str) -> Result<(), VfsError> {
        let id = self.resolve_path(path)?;
        let inode = self.get_inode(id)?;
        if inode.is_dir() {
            return Err(VfsError::IsDirectory {
                path: path.to_string(),
            });
        }

        let (parent_id, name) = self.resolve_parent(path)?;
        self.dir_entries_mut(parent_id)?.remove(&name);
        self.inodes.remove(&id);
        Ok(())
    }

    pub fn rmdir(&mut self, path: &str) -> Result<(), VfsError> {
        let id = self.resolve_path(path)?;
        let inode = self.get_inode(id)?;
        match &inode.kind {
            InodeKind::Directory { entries } => {
                if !entries.is_empty() {
                    return Err(VfsError::NotEmpty {
                        path: path.to_string(),
                    });
                }
            }
            _ => {
                return Err(VfsError::NotDirectory {
                    path: path.to_string(),
                })
            }
        }

        if id == self.root {
            return Err(VfsError::InvalidPath {
                path: "cannot remove root".to_string(),
            });
        }

        let (parent_id, name) = self.resolve_parent(path)?;
        self.dir_entries_mut(parent_id)?.remove(&name);
        self.inodes.remove(&id);
        Ok(())
    }

    pub fn rm_rf(&mut self, path: &str) -> Result<(), VfsError> {
        let id = self.resolve_path(path)?;
        if id == self.root {
            return Err(VfsError::InvalidPath {
                path: "cannot remove root".to_string(),
            });
        }

        let ids_to_remove = self.collect_subtree(id);
        let (parent_id, name) = self.resolve_parent(path)?;
        self.dir_entries_mut(parent_id)?.remove(&name);
        for rid in ids_to_remove {
            self.inodes.remove(&rid);
        }
        Ok(())
    }

    fn collect_subtree(&self, id: InodeId) -> Vec<InodeId> {
        let mut result = vec![id];
        if let Ok(inode) = self.get_inode(id) {
            if let InodeKind::Directory { entries } = &inode.kind {
                for &child_id in entries.values() {
                    result.extend(self.collect_subtree(child_id));
                }
            }
        }
        result
    }

    pub fn mv(&mut self, src: &str, dst: &str) -> Result<(), VfsError> {
        let src_id = self.resolve_path(src)?;

        if self.get_inode(src_id)?.is_file() {
            let dst_name = dst.rsplit('/').next().unwrap_or(dst);
            validate_markdown_filename(dst_name)?;
        }

        let (src_parent, src_name) = self.resolve_parent(src)?;

        if let Ok(dst_id) = self.resolve_path(dst) {
            if self.get_inode(dst_id)?.is_dir() {
                self.dir_entries_mut(src_parent)?.remove(&src_name);
                self.dir_entries_mut(dst_id)?.insert(src_name, src_id);
                return Ok(());
            }
        }

        let (dst_parent, dst_name) = self.resolve_parent(dst)?;
        self.dir_entries_mut(src_parent)?.remove(&src_name);
        self.dir_entries_mut(dst_parent)?.insert(dst_name, src_id);
        Ok(())
    }

    pub fn cp(&mut self, src: &str, dst: &str, uid: Uid, gid: Gid) -> Result<(), VfsError> {
        let src_id = self.resolve_path(src)?;
        let src_inode = self.get_inode(src_id)?.clone();

        if src_inode.is_dir() {
            return Err(VfsError::IsDirectory {
                path: src.to_string(),
            });
        }

        let dst_name = dst.rsplit('/').next().unwrap_or(dst);
        if src_inode.is_file() {
            validate_markdown_filename(dst_name)?;
        }

        let (parent_id, name) = if let Ok(dst_id) = self.resolve_path(dst) {
            if self.get_inode(dst_id)?.is_dir() {
                let src_name = src.rsplit('/').next().unwrap_or(src);
                (dst_id, src_name.to_string())
            } else {
                self.resolve_parent(dst)?
            }
        } else {
            self.resolve_parent(dst)?
        };

        let new_id = self.alloc_id();
        let mut new_inode = src_inode;
        new_inode.id = new_id;
        new_inode.uid = uid; // Copy is owned by the caller
        new_inode.gid = self.effective_gid(parent_id, gid);
        self.inodes.insert(new_id, new_inode);
        self.dir_entries_mut(parent_id)?.insert(name, new_id);
        Ok(())
    }

    pub fn stat(&self, path: &str) -> Result<StatInfo, VfsError> {
        let id = self.resolve_path(path)?;
        let inode = self.get_inode(id)?;
        Ok(StatInfo {
            inode_id: id,
            kind: match &inode.kind {
                InodeKind::File { .. } => "file",
                InodeKind::Directory { .. } => "directory",
                InodeKind::Symlink { .. } => "symlink",
            },
            size: inode.size(),
            mode: inode.mode,
            uid: inode.uid,
            gid: inode.gid,
            created: inode.created,
            modified: inode.modified,
        })
    }

    pub fn chmod(&mut self, path: &str, mode: u16) -> Result<(), VfsError> {
        let id = self.resolve_path(path)?;
        let inode = self.get_inode_mut(id)?;
        inode.mode = mode;
        Ok(())
    }

    pub fn chown(&mut self, path: &str, uid: Uid, gid: Gid) -> Result<(), VfsError> {
        let id = self.resolve_path(path)?;
        let inode = self.get_inode_mut(id)?;
        inode.uid = uid;
        inode.gid = gid;
        Ok(())
    }

    pub fn ln_s(&mut self, target: &str, link_path: &str, uid: Uid, gid: Gid) -> Result<(), VfsError> {
        let (parent_id, name) = self.resolve_parent(link_path)?;
        let entries = self.dir_entries(parent_id)?;
        if entries.contains_key(&name) {
            return Err(VfsError::AlreadyExists {
                path: link_path.to_string(),
            });
        }

        let effective_gid = self.effective_gid(parent_id, gid);
        let new_id = self.alloc_id();
        let symlink = Inode::new_symlink(new_id, target.to_string(), uid, effective_gid);
        self.inodes.insert(new_id, symlink);
        self.dir_entries_mut(parent_id)?.insert(name, new_id);
        Ok(())
    }

    pub fn tree(
        &self,
        path: Option<&str>,
        prefix: &str,
        session: Option<&Session>,
    ) -> Result<String, VfsError> {
        let id = match path {
            Some(p) => self.resolve_path(p)?,
            None => self.cwd,
        };
        let mut output = String::new();
        if prefix.is_empty() {
            output.push_str(".\n");
        }
        self.tree_recursive(id, prefix, &mut output, session)?;
        Ok(output)
    }

    fn tree_recursive(
        &self,
        id: InodeId,
        prefix: &str,
        output: &mut String,
        session: Option<&Session>,
    ) -> Result<(), VfsError> {
        let entries = self.dir_entries(id)?;
        // Filter entries by read permission
        let visible: Vec<_> = entries
            .iter()
            .filter(|&(_, child_id)| self.is_visible(*child_id, session))
            .collect();
        let count = visible.len();
        for (i, (name, child_id)) in visible.iter().enumerate() {
            let is_last = i == count - 1;
            let connector = if is_last {
                "\u{2514}\u{2500}\u{2500} "
            } else {
                "\u{251c}\u{2500}\u{2500} "
            };
            let child = self.get_inode(**child_id)?;

            output.push_str(prefix);
            output.push_str(connector);
            output.push_str(name);
            if child.is_dir() {
                output.push('/');
            }
            output.push('\n');

            if child.is_dir() {
                let new_prefix = if is_last {
                    format!("{prefix}    ")
                } else {
                    format!("{prefix}\u{2502}   ")
                };
                self.tree_recursive(**child_id, &new_prefix, output, session)?;
            }
        }
        Ok(())
    }

    pub fn find(
        &self,
        path: Option<&str>,
        pattern: Option<&str>,
        session: Option<&Session>,
    ) -> Result<Vec<String>, VfsError> {
        let id = match path {
            Some(p) => self.resolve_path(p)?,
            None => self.cwd,
        };
        let base = match path {
            Some(p) => p.to_string(),
            None => ".".to_string(),
        };
        let mut results = Vec::new();
        self.find_recursive(id, &base, pattern, &mut results, session)?;
        Ok(results)
    }

    fn find_recursive(
        &self,
        id: InodeId,
        current_path: &str,
        pattern: Option<&str>,
        results: &mut Vec<String>,
        session: Option<&Session>,
    ) -> Result<(), VfsError> {
        let entries = self.dir_entries(id)?;
        for (name, &child_id) in entries {
            if !self.is_visible(child_id, session) {
                continue;
            }

            let child_path = if current_path == "." {
                format!("./{name}")
            } else {
                format!("{current_path}/{name}")
            };
            let child = self.get_inode(child_id)?;

            let matches = match pattern {
                Some(pat) => glob_match(pat, name),
                None => true,
            };

            if matches {
                results.push(child_path.clone());
            }

            if child.is_dir() {
                self.find_recursive(child_id, &child_path, pattern, results, session)?;
            }
        }
        Ok(())
    }

    pub fn grep(
        &self,
        pattern: &str,
        path: Option<&str>,
        recursive: bool,
        session: Option<&Session>,
    ) -> Result<Vec<GrepResult>, VfsError> {
        let re = regex::Regex::new(pattern).map_err(|e| VfsError::InvalidArgs {
            message: format!("invalid regex: {e}"),
        })?;

        let id = match path {
            Some(p) => self.resolve_path(p)?,
            None => self.cwd,
        };
        let base = path.unwrap_or(".");
        let mut results = Vec::new();
        let inode = self.get_inode(id)?;

        match &inode.kind {
            InodeKind::File { content } => {
                let text = String::from_utf8_lossy(content);
                for (line_num, line) in text.lines().enumerate() {
                    if re.is_match(line) {
                        results.push(GrepResult {
                            file: base.to_string(),
                            line_num: line_num + 1,
                            line: line.to_string(),
                        });
                    }
                }
            }
            InodeKind::Directory { .. } if recursive => {
                self.grep_recursive(id, base, &re, &mut results, session)?;
            }
            InodeKind::Directory { .. } => {
                return Err(VfsError::IsDirectory {
                    path: base.to_string(),
                });
            }
            _ => {}
        }
        Ok(results)
    }

    fn grep_recursive(
        &self,
        id: InodeId,
        base: &str,
        re: &regex::Regex,
        results: &mut Vec<GrepResult>,
        session: Option<&Session>,
    ) -> Result<(), VfsError> {
        let entries = self.dir_entries(id)?;
        for (name, &child_id) in entries {
            if !self.is_visible(child_id, session) {
                continue;
            }

            let child_path = if base == "." {
                format!("./{name}")
            } else {
                format!("{base}/{name}")
            };
            let child = self.get_inode(child_id)?;
            match &child.kind {
                InodeKind::File { content } => {
                    let text = String::from_utf8_lossy(content);
                    for (line_num, line) in text.lines().enumerate() {
                        if re.is_match(line) {
                            results.push(GrepResult {
                                file: child_path.clone(),
                                line_num: line_num + 1,
                                line: line.to_string(),
                            });
                        }
                    }
                }
                InodeKind::Directory { .. } => {
                    self.grep_recursive(child_id, &child_path, re, results, session)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Check if a user can see this inode (has read permission).
    /// If session is None, everything is visible (internal/root use).
    /// Respects delegation: both principal and delegate must have read access.
    fn is_visible(&self, id: InodeId, session: Option<&Session>) -> bool {
        let session = match session {
            Some(s) => s,
            None => return true,
        };
        let inode = match self.get_inode(id) {
            Ok(i) => i,
            Err(_) => return false,
        };
        session.has_permission(inode, Access::Read)
    }

    pub fn all_inodes(&self) -> &HashMap<InodeId, Inode> {
        &self.inodes
    }

    pub fn root_id(&self) -> InodeId {
        self.root
    }

    pub fn cwd_id(&self) -> InodeId {
        self.cwd
    }

    pub fn next_inode_id(&self) -> InodeId {
        self.next_id
    }

    pub fn cwd_path_clone(&self) -> Vec<(String, InodeId)> {
        self.cwd_path.clone()
    }

    pub fn from_persisted(
        inodes: HashMap<InodeId, Inode>,
        root: InodeId,
        cwd: InodeId,
        next_id: InodeId,
        cwd_path: Vec<(String, InodeId)>,
        registry: UserRegistry,
    ) -> Self {
        VirtualFs {
            inodes,
            root,
            cwd,
            next_id,
            cwd_path,
            registry,
        }
    }
}

fn glob_match(pattern: &str, name: &str) -> bool {
    let pat = pattern
        .replace('.', "\\.")
        .replace('*', ".*")
        .replace('?', ".");
    regex::Regex::new(&format!("^{pat}$"))
        .map(|re| re.is_match(name))
        .unwrap_or(false)
}

fn validate_markdown_filename(path: &str) -> Result<(), VfsError> {
    let filename = path.rsplit('/').next().unwrap_or(path);
    if !filename.ends_with(".md") {
        return Err(VfsError::InvalidExtension {
            name: filename.to_string(),
        });
    }
    Ok(())
}

#[derive(Debug)]
pub struct LsEntry {
    pub name: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
    pub modified: u64,
}

#[derive(Debug)]
pub struct StatInfo {
    pub inode_id: InodeId,
    pub kind: &'static str,
    pub size: u64,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
    pub created: u64,
    pub modified: u64,
}

#[derive(Debug, Clone)]
pub struct GrepResult {
    pub file: String,
    pub line_num: usize,
    pub line: String,
}
