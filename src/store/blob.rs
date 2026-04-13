use super::{ObjectId, ObjectKind};
use crate::error::VfsError;
use std::collections::HashMap;

/// In-memory content-addressable blob store.
/// Will be upgraded to mmap-backed pack file in Phase 3.
pub struct BlobStore {
    objects: HashMap<ObjectId, (ObjectKind, Vec<u8>)>,
}

impl BlobStore {
    pub fn new() -> Self {
        BlobStore {
            objects: HashMap::new(),
        }
    }

    pub fn put(&mut self, data: &[u8], kind: ObjectKind) -> ObjectId {
        let id = ObjectId::from_bytes(data);
        // Dedup: if already stored, skip
        self.objects.entry(id).or_insert_with(|| (kind, data.to_vec()));
        id
    }

    pub fn get(&self, id: &ObjectId) -> Result<&[u8], VfsError> {
        self.objects
            .get(id)
            .map(|(_, data)| data.as_slice())
            .ok_or_else(|| VfsError::ObjectNotFound {
                id: id.short_hex(),
            })
    }

    pub fn contains(&self, id: &ObjectId) -> bool {
        self.objects.contains_key(id)
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// Export all objects for persistence: (ObjectId bytes, kind as u8, data).
    pub fn export_all(&self) -> Vec<(Vec<u8>, u8, Vec<u8>)> {
        self.objects
            .iter()
            .map(|(id, (kind, data))| {
                let kind_byte = match kind {
                    ObjectKind::Blob => 0u8,
                    ObjectKind::Tree => 1u8,
                    ObjectKind::Commit => 2u8,
                };
                (id.as_bytes().to_vec(), kind_byte, data.clone())
            })
            .collect()
    }

    /// Import objects from persisted state.
    pub fn import_all(&mut self, objects: Vec<(Vec<u8>, u8, Vec<u8>)>) -> Result<(), crate::error::VfsError> {
        for (id_bytes, kind_byte, data) in objects {
            let mut arr = [0u8; 32];
            if id_bytes.len() != 32 {
                return Err(crate::error::VfsError::CorruptStore {
                    message: "invalid object ID length".to_string(),
                });
            }
            arr.copy_from_slice(&id_bytes);
            let id = ObjectId::from_raw(arr);
            let kind = match kind_byte {
                0 => ObjectKind::Blob,
                1 => ObjectKind::Tree,
                2 => ObjectKind::Commit,
                _ => {
                    return Err(crate::error::VfsError::CorruptStore {
                        message: format!("unknown object kind: {kind_byte}"),
                    })
                }
            };
            self.objects.insert(id, (kind, data));
        }
        Ok(())
    }
}
