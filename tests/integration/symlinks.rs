use super::*;

#[test]
fn test_symlink() {
    let mut fs = VirtualFs::new();
    exec("touch target.md", &mut fs);
    exec("write target.md linked content", &mut fs);
    exec("ln -s target.md link.md", &mut fs);
    let output = exec("cat link.md", &mut fs);
    assert_eq!(output, "linked content");
}

#[test]
fn test_symlink_stat() {
    let mut fs = VirtualFs::new();
    exec("touch target.md", &mut fs);
    exec("ln -s target.md link.md", &mut fs);
    let stat = exec("stat link.md", &mut fs);
    assert!(stat.contains("symlink"));
}

#[test]
fn test_symlink_write_through() {
    let mut fs = VirtualFs::new();
    exec("touch target.md", &mut fs);
    exec("write target.md original", &mut fs);
    exec("ln -s target.md link.md", &mut fs);
    exec("write link.md updated", &mut fs);
    // Reading via original path should reflect the update
    assert_eq!(exec("cat target.md", &mut fs), "updated");
}

#[test]
fn test_symlink_duplicate_fails() {
    let mut fs = VirtualFs::new();
    exec("touch target.md", &mut fs);
    exec("ln -s target.md link.md", &mut fs);
    let err = exec_err("ln -s target.md link.md", &mut fs);
    assert!(err.contains("already exists"));
}
