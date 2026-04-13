pub mod revert;
pub mod snapshot;

use crate::error::VfsError;
use crate::fs::inode::{InodeKind, InodeId};
use crate::fs::VirtualFs;
use crate::store::blob::BlobStore;
use crate::store::commit::CommitObject;
use crate::store::tree::{TreeEntry, TreeEntryKind, TreeObject};
use crate::store::{ObjectId, ObjectKind};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Vcs {
    pub store: BlobStore,
    pub head: Option<ObjectId>,
    pub commits: Vec<CommitObject>,
}

impl Vcs {
    pub fn new() -> Self {
        Vcs {
            store: BlobStore::new(),
            head: None,
            commits: Vec::new(),
        }
    }

    pub fn commit(
        &mut self,
        fs: &VirtualFs,
        message: &str,
        author: &str,
    ) -> Result<ObjectId, VfsError> {
        let root_tree_id = self.snapshot_dir(fs, fs.root_id())?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let commit = CommitObject {
            id: ObjectId::from_bytes(&[0; 32]), // placeholder
            tree: root_tree_id,
            parent: self.head,
            timestamp,
            message: message.to_string(),
            author: author.to_string(),
        };

        let commit_data = commit.serialize();
        let commit_id = self.store.put(&commit_data, ObjectKind::Commit);

        let final_commit = CommitObject {
            id: commit_id,
            ..commit
        };

        self.commits.push(final_commit);
        self.head = Some(commit_id);
        Ok(commit_id)
    }

    fn snapshot_dir(&mut self, fs: &VirtualFs, dir_id: InodeId) -> Result<ObjectId, VfsError> {
        let inode = fs.get_inode(dir_id)?;
        let entries = match &inode.kind {
            InodeKind::Directory { entries } => entries,
            _ => {
                return Err(VfsError::NotDirectory {
                    path: format!("<inode {dir_id}>"),
                })
            }
        };

        let mut tree_entries = Vec::with_capacity(entries.len());
        for (name, &child_id) in entries {
            let child = fs.get_inode(child_id)?;
            let (kind, id) = match &child.kind {
                InodeKind::File { content } => {
                    let blob_id = self.store.put(content, ObjectKind::Blob);
                    (TreeEntryKind::Blob, blob_id)
                }
                InodeKind::Directory { .. } => {
                    let tree_id = self.snapshot_dir(fs, child_id)?;
                    (TreeEntryKind::Tree, tree_id)
                }
                InodeKind::Symlink { target } => {
                    let blob_id = self.store.put(target.as_bytes(), ObjectKind::Blob);
                    (TreeEntryKind::Symlink, blob_id)
                }
            };

            tree_entries.push(TreeEntry {
                name: name.clone(),
                kind,
                id,
                mode: child.mode,
                uid: child.uid,
                gid: child.gid,
            });
        }

        let tree = TreeObject {
            entries: tree_entries,
        };
        let tree_data = tree.serialize();
        Ok(self.store.put(&tree_data, ObjectKind::Tree))
    }

    pub fn log(&self) -> Vec<&CommitObject> {
        self.commits.iter().rev().collect()
    }

    pub fn revert(&mut self, fs: &mut VirtualFs, hash_prefix: &str) -> Result<(), VfsError> {
        let commit = self.find_commit(hash_prefix)?;
        let tree_id = commit.tree;
        let commit_id = commit.id;

        // Reconstruct VFS from tree
        revert::restore_from_tree(fs, &self.store, tree_id)?;

        self.head = Some(commit_id);
        Ok(())
    }

    pub fn find_commit(&self, hash_prefix: &str) -> Result<&CommitObject, VfsError> {
        let matches: Vec<&CommitObject> = self
            .commits
            .iter()
            .filter(|c| c.id.to_hex().starts_with(hash_prefix))
            .collect();

        match matches.len() {
            0 => Err(VfsError::ObjectNotFound {
                id: hash_prefix.to_string(),
            }),
            1 => Ok(matches[0]),
            _ => Err(VfsError::InvalidArgs {
                message: format!("ambiguous commit prefix: {hash_prefix}"),
            }),
        }
    }

    pub fn status(&self, fs: &VirtualFs) -> Result<String, VfsError> {
        if self.head.is_none() {
            return Ok("No commits yet.\n".to_string());
        }

        let mut output = String::new();
        output.push_str(&format!(
            "On commit {}\n",
            self.head.unwrap().short_hex()
        ));
        output.push_str(&format!(
            "Objects in store: {}\n",
            self.store.object_count()
        ));

        let mut file_count = 0u64;
        let mut total_size = 0u64;
        for inode in fs.all_inodes().values() {
            if let InodeKind::File { content } = &inode.kind {
                file_count += 1;
                total_size += content.len() as u64;
            }
        }
        output.push_str(&format!("Files: {file_count}, Total size: {total_size} bytes\n"));
        Ok(output)
    }
}
