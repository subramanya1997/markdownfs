pub mod perms;
pub mod registry;
pub mod session;

use serde::{Deserialize, Serialize};

pub type Uid = u32;
pub type Gid = u32;

pub const ROOT_UID: Uid = 0;
pub const ROOT_GID: Gid = 0;
pub const WHEEL_GID: Gid = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub uid: Uid,
    pub name: String,
    pub groups: Vec<Gid>,
    pub api_token: Option<String>,
    pub is_agent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub gid: Gid,
    pub name: String,
    pub members: Vec<Uid>,
}
