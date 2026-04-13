use crate::error::VfsError;
use crate::fs::VirtualFs;
use crate::store::blob::BlobStore;
use crate::store::tree::{TreeEntryKind, TreeObject};
use crate::store::ObjectId;

/// Reconstruct a VirtualFs from a tree object in the store.
pub fn restore_from_tree(
    fs: &mut VirtualFs,
    store: &BlobStore,
    root_tree_id: ObjectId,
) -> Result<(), VfsError> {
    // Create a fresh VFS
    *fs = VirtualFs::new();

    // Recursively restore from root tree
    restore_dir(fs, store, "/", root_tree_id)?;
    Ok(())
}

fn restore_dir(
    fs: &mut VirtualFs,
    store: &BlobStore,
    dir_path: &str,
    tree_id: ObjectId,
) -> Result<(), VfsError> {
    let tree_data = store.get(&tree_id)?;
    let tree = TreeObject::deserialize(tree_data).map_err(|e| VfsError::CorruptStore {
        message: format!("failed to deserialize tree: {e}"),
    })?;

    for entry in &tree.entries {
        let child_path = if dir_path == "/" {
            format!("/{}", entry.name)
        } else {
            format!("{}/{}", dir_path, entry.name)
        };

        match entry.kind {
            TreeEntryKind::Blob => {
                let content = store.get(&entry.id)?;
                fs.touch(&child_path, 0, 0)?;
                fs.write_file(&child_path, content.to_vec())?;
                fs.chmod(&child_path, entry.mode)?;
            }
            TreeEntryKind::Tree => {
                fs.mkdir(&child_path, 0, 0)?;
                fs.chmod(&child_path, entry.mode)?;
                restore_dir(fs, store, &child_path, entry.id)?;
            }
            TreeEntryKind::Symlink => {
                let target = String::from_utf8_lossy(store.get(&entry.id)?).to_string();
                fs.ln_s(&target, &child_path, 0, 0)?;
            }
        }
    }

    Ok(())
}
