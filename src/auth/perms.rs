use super::{Gid, Uid, ROOT_UID};
use crate::fs::inode::Inode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
    Execute,
}

/// Check if user `uid` with `user_groups` can perform `access` on `inode`.
/// Root (uid=0) bypasses all checks.
pub fn check_permission(inode: &Inode, uid: Uid, user_groups: &[Gid], access: Access) -> bool {
    if uid == ROOT_UID {
        return true;
    }

    let bit = match access {
        Access::Read => 4,
        Access::Write => 2,
        Access::Execute => 1,
    };

    if uid == inode.uid {
        // Owner bits (bits 8-6)
        return (inode.mode >> 6) & bit != 0;
    }

    if user_groups.contains(&inode.gid) {
        // Group bits (bits 5-3)
        return (inode.mode >> 3) & bit != 0;
    }

    // Other bits (bits 2-0)
    inode.mode & bit != 0
}

/// Check if sticky bit is set (0o1000).
/// In a sticky directory, only the file owner, dir owner, or root can delete/rename.
pub fn has_sticky_bit(mode: u16) -> bool {
    mode & 0o1000 != 0
}

/// Check if setgid bit is set (0o2000).
/// New files/dirs in a setgid directory inherit the directory's group.
pub fn has_setgid(mode: u16) -> bool {
    mode & 0o2000 != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::inode::Inode;

    fn make_file(mode: u16, uid: u32, gid: u32) -> Inode {
        let mut inode = Inode::new_file(1, uid, gid);
        inode.mode = mode;
        inode
    }

    #[test]
    fn test_root_bypasses_all() {
        let inode = make_file(0o000, 99, 99); // no permissions
        assert!(check_permission(&inode, ROOT_UID, &[0], Access::Read));
        assert!(check_permission(&inode, ROOT_UID, &[0], Access::Write));
        assert!(check_permission(&inode, ROOT_UID, &[0], Access::Execute));
    }

    #[test]
    fn test_owner_permissions() {
        let inode = make_file(0o600, 1, 10); // rw- --- ---
        assert!(check_permission(&inode, 1, &[10], Access::Read));
        assert!(check_permission(&inode, 1, &[10], Access::Write));
        assert!(!check_permission(&inode, 1, &[10], Access::Execute));
    }

    #[test]
    fn test_group_permissions() {
        let inode = make_file(0o050, 1, 10); // --- r-x ---
        assert!(!check_permission(&inode, 2, &[10], Access::Write));
        assert!(check_permission(&inode, 2, &[10], Access::Read));
        assert!(check_permission(&inode, 2, &[10], Access::Execute));
    }

    #[test]
    fn test_other_permissions() {
        let inode = make_file(0o004, 1, 10); // --- --- r--
        assert!(check_permission(&inode, 99, &[99], Access::Read));
        assert!(!check_permission(&inode, 99, &[99], Access::Write));
    }

    #[test]
    fn test_no_permissions() {
        let inode = make_file(0o000, 1, 10);
        assert!(!check_permission(&inode, 2, &[20], Access::Read));
        assert!(!check_permission(&inode, 2, &[20], Access::Write));
        assert!(!check_permission(&inode, 2, &[20], Access::Execute));
    }

    #[test]
    fn test_sticky_and_setgid() {
        assert!(has_sticky_bit(0o1755));
        assert!(!has_sticky_bit(0o0755));
        assert!(has_setgid(0o2755));
        assert!(!has_setgid(0o0755));
    }
}
