use super::perms::{check_permission, Access};
use super::{Gid, Uid, ROOT_UID, WHEEL_GID};
use crate::fs::inode::Inode;

/// Context for the user an agent is acting on behalf of.
#[derive(Debug, Clone)]
pub struct DelegateContext {
    pub uid: Uid,
    pub gid: Gid,
    pub groups: Vec<Gid>,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub uid: Uid,
    pub gid: Gid,
    pub groups: Vec<Gid>,
    pub username: String,
    /// When set, the session acts on behalf of this user.
    /// All permission checks require BOTH the principal AND
    /// the delegate to have access (intersection / least-privilege).
    pub delegate: Option<DelegateContext>,
}

impl Session {
    pub fn new(uid: Uid, gid: Gid, groups: Vec<Gid>, username: String) -> Self {
        Session {
            uid,
            gid,
            groups,
            username,
            delegate: None,
        }
    }

    pub fn root() -> Self {
        Session {
            uid: ROOT_UID,
            gid: 0,
            groups: vec![0, 1],
            username: "root".to_string(),
            delegate: None,
        }
    }

    pub fn is_root(&self) -> bool {
        self.uid == ROOT_UID
    }

    /// Check permission with delegation intersection.
    /// Returns true only if both the principal AND the delegate (if any) have access.
    pub fn has_permission(&self, inode: &Inode, access: Access) -> bool {
        if !check_permission(inode, self.uid, &self.groups, access) {
            return false;
        }
        if let Some(ref delegate) = self.delegate {
            if !check_permission(inode, delegate.uid, &delegate.groups, access) {
                return false;
            }
        }
        true
    }

    /// Check permission using raw mode/uid/gid bits (for LsEntry filtering).
    /// Respects delegation intersection.
    pub fn has_permission_bits(
        &self,
        mode: u16,
        file_uid: Uid,
        file_gid: Gid,
        access: Access,
    ) -> bool {
        if !check_bits(self.uid, &self.groups, mode, file_uid, file_gid, access) {
            return false;
        }
        if let Some(ref delegate) = self.delegate {
            if !check_bits(
                delegate.uid,
                &delegate.groups,
                mode,
                file_uid,
                file_gid,
                access,
            ) {
                return false;
            }
        }
        true
    }

    /// The effective uid for file ownership — if delegating, use the delegate's uid.
    pub fn effective_uid(&self) -> Uid {
        match &self.delegate {
            Some(d) => d.uid,
            None => self.uid,
        }
    }

    /// The effective gid for file ownership — if delegating, use the delegate's gid.
    pub fn effective_gid(&self) -> Gid {
        match &self.delegate {
            Some(d) => d.gid,
            None => self.gid,
        }
    }

    /// Whether the principal is a wheel-group member (admin-equivalent).
    pub fn is_wheel(&self) -> bool {
        self.groups.contains(&WHEEL_GID)
    }

    /// Whether the session may bypass per-file Read checks (i.e., see every
    /// directory entry and read every file). True for literal uid=0 or any
    /// wheel-group member. Privileged write ops (chmod-across-owner,
    /// chown-uid) still go through the stricter `is_effectively_root` check.
    /// When delegating, the delegate must also be admin-equivalent.
    pub fn can_read_anywhere(&self) -> bool {
        let principal_admin = self.uid == ROOT_UID || self.is_wheel();
        if !principal_admin {
            return false;
        }
        match &self.delegate {
            Some(d) => d.uid == ROOT_UID || d.groups.contains(&WHEEL_GID),
            None => true,
        }
    }

    /// Whether either the principal or (if delegating) the delegate is root.
    /// Strict: only literal uid=0 qualifies. Used by privileged write ops.
    pub fn is_effectively_root(&self) -> bool {
        if self.uid != ROOT_UID {
            return false;
        }
        match &self.delegate {
            Some(d) => d.uid == ROOT_UID,
            None => true,
        }
    }

    /// Whether the principal (ignoring delegation) is the owner of a file.
    /// When delegating, checks if the delegate is the owner.
    pub fn is_effective_owner(&self, file_uid: Uid) -> bool {
        match &self.delegate {
            Some(d) => {
                // Both must be owner or root
                let principal_ok = self.uid == ROOT_UID || self.uid == file_uid;
                let delegate_ok = d.uid == ROOT_UID || d.uid == file_uid;
                principal_ok && delegate_ok
            }
            None => self.uid == ROOT_UID || self.uid == file_uid,
        }
    }
}

/// Raw bit-level permission check for a single principal.
fn check_bits(
    uid: Uid,
    groups: &[Gid],
    mode: u16,
    file_uid: Uid,
    file_gid: Gid,
    access: Access,
) -> bool {
    if uid == ROOT_UID {
        return true;
    }
    let bit = match access {
        Access::Read => 4,
        Access::Write => 2,
        Access::Execute => 1,
    };
    if uid == file_uid {
        return (mode >> 6) & bit != 0;
    }
    if groups.contains(&file_gid) {
        return (mode >> 3) & bit != 0;
    }
    mode & bit != 0
}
