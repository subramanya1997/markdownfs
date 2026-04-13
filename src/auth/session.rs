use super::{Gid, Uid, ROOT_UID};

#[derive(Debug, Clone)]
pub struct Session {
    pub uid: Uid,
    pub gid: Gid,
    pub groups: Vec<Gid>,
    pub username: String,
}

impl Session {
    pub fn new(uid: Uid, gid: Gid, groups: Vec<Gid>, username: String) -> Self {
        Session {
            uid,
            gid,
            groups,
            username,
        }
    }

    pub fn root() -> Self {
        Session {
            uid: ROOT_UID,
            gid: 0,
            groups: vec![0, 1],
            username: "root".to_string(),
        }
    }

    pub fn is_root(&self) -> bool {
        self.uid == ROOT_UID
    }
}
