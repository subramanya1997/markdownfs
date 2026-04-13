use super::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitObject {
    pub id: ObjectId,
    pub tree: ObjectId,
    pub parent: Option<ObjectId>,
    pub timestamp: u64,
    pub message: String,
    pub author: String,
}

impl CommitObject {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).expect("commit serialization should not fail")
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(data)
    }
}
