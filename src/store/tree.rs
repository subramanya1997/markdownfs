use super::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    pub name: String,
    pub kind: TreeEntryKind,
    pub id: ObjectId,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TreeEntryKind {
    Blob,
    Tree,
    Symlink,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeObject {
    pub entries: Vec<TreeEntry>,
}

impl TreeObject {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).expect("tree serialization should not fail")
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(data)
    }
}

// Need serde impls for ObjectId
impl Serialize for ObjectId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for ObjectId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
        if bytes.len() != 32 {
            return Err(serde::de::Error::custom("expected 32 bytes for ObjectId"));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(ObjectId(arr))
    }
}
