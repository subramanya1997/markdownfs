use super::*;

#[test]
fn test_mkdir_and_ls() {
    let mut fs = VirtualFs::new();
    exec("mkdir docs", &mut fs);
    exec("mkdir src", &mut fs);
    let output = exec("ls", &mut fs);
    assert!(output.contains("docs/"));
    assert!(output.contains("src/"));
}

#[test]
fn test_mkdir_duplicate() {
    let mut fs = VirtualFs::new();
    exec("mkdir docs", &mut fs);
    let err = exec_err("mkdir docs", &mut fs);
    assert!(err.contains("already exists"));
}

#[test]
fn test_mkdir_p_creates_intermediate() {
    let mut fs = VirtualFs::new();
    exec("mkdir -p a/b/c/d/e", &mut fs);
    exec("cd a/b/c/d/e", &mut fs);
    assert_eq!(exec("pwd", &mut fs).trim(), "/a/b/c/d/e");
}

#[test]
fn test_mkdir_p_idempotent() {
    let mut fs = VirtualFs::new();
    exec("mkdir -p a/b/c", &mut fs);
    exec("mkdir -p a/b/c", &mut fs); // should not error
    exec("cd a/b/c", &mut fs);
    assert_eq!(exec("pwd", &mut fs).trim(), "/a/b/c");
}

#[test]
fn test_rmdir_empty() {
    let mut fs = VirtualFs::new();
    exec("mkdir empty", &mut fs);
    exec("rmdir empty", &mut fs);
    let ls = exec("ls", &mut fs);
    assert!(!ls.contains("empty"));
}

#[test]
fn test_rmdir_non_empty_fails() {
    let mut fs = VirtualFs::new();
    exec("mkdir dir", &mut fs);
    exec("cd dir", &mut fs);
    exec("touch file.md", &mut fs);
    exec("cd /", &mut fs);
    let err = exec_err("rmdir dir", &mut fs);
    assert!(err.contains("not empty"));
}

#[test]
fn test_rmdir_on_file_fails() {
    let mut fs = VirtualFs::new();
    exec("touch file.md", &mut fs);
    let err = exec_err("rmdir file.md", &mut fs);
    assert!(err.contains("not a directory"));
}
