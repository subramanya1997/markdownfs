pub mod blob;
pub mod commit;
pub mod index;
pub mod tree;

use crate::error::VfsError;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId([u8; 32]);

impl ObjectId {
    pub fn from_bytes(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut id = [0u8; 32];
        id.copy_from_slice(&result);
        ObjectId(id)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    pub fn short_hex(&self) -> String {
        self.to_hex()[..8].to_string()
    }

    pub fn from_raw(bytes: [u8; 32]) -> Self {
        ObjectId(bytes)
    }

    pub fn from_hex(hex: &str) -> Result<Self, VfsError> {
        if hex.len() != 64 {
            return Err(VfsError::InvalidArgs {
                message: format!("invalid object ID: {hex}"),
            });
        }
        let mut bytes = [0u8; 32];
        for i in 0..32 {
            bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|_| {
                VfsError::InvalidArgs {
                    message: format!("invalid hex: {hex}"),
                }
            })?;
        }
        Ok(ObjectId(bytes))
    }
}

impl std::fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ObjectId({})", self.short_hex())
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ObjectKind {
    Blob,
    Tree,
    Commit,
}
