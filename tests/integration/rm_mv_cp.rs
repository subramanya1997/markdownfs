use super::*;

#[test]
fn test_rm_file() {
    let mut fs = VirtualFs::new();
    exec("touch file.md", &mut fs);
    assert!(exec("ls", &mut fs).contains("file.md"));
    exec("rm file.md", &mut fs);
    assert!(!exec("ls", &mut fs).contains("file.md"));
}

#[test]
fn test_rm_nonexistent_file_fails() {
    let mut fs = VirtualFs::new();
    let err = exec_err("rm ghost.md", &mut fs);
    assert!(err.contains("no such file"));
}

#[test]
fn test_rm_directory_fails_without_r() {
    let mut fs = VirtualFs::new();
    exec("mkdir dir", &mut fs);
    let err = exec_err("rm dir", &mut fs);
    assert!(err.contains("is a directory"));
}

#[test]
fn test_rm_rf() {
    let mut fs = VirtualFs::new();
    exec("mkdir -p a/b", &mut fs);
    exec("cd a/b", &mut fs);
    exec("touch x.md", &mut fs);
    exec("cd /", &mut fs);
    exec("rm -r a", &mut fs);
    assert!(!exec("ls", &mut fs).contains("a/"));
}

#[test]
fn test_rm_rf_deep_tree() {
    let mut fs = VirtualFs::new();
    exec("mkdir -p a/b/c/d", &mut fs);
    exec("cd a/b/c/d", &mut fs);
    exec("touch deep.md", &mut fs);
    exec("cd /a/b/c", &mut fs);
    exec("touch mid.md", &mut fs);
    exec("cd /a/b", &mut fs);
    exec("touch upper.md", &mut fs);
    exec("cd /", &mut fs);
    exec("rm -r a", &mut fs);
    assert!(!exec("ls", &mut fs).contains("a"));
}

#[test]
fn test_mv() {
    let mut fs = VirtualFs::new();
    exec("touch old.md", &mut fs);
    exec("mv old.md new.md", &mut fs);
    let listing = exec("ls", &mut fs);
    assert!(!listing.contains("old.md"));
    assert!(listing.contains("new.md"));
}

#[test]
fn test_mv_preserves_content() {
    let mut fs = VirtualFs::new();
    exec("touch src.md", &mut fs);
    exec("write src.md important data", &mut fs);
    exec("mv src.md dst.md", &mut fs);
    assert_eq!(exec("cat dst.md", &mut fs), "important data");
}

#[test]
fn test_mv_into_directory() {
    let mut fs = VirtualFs::new();
    exec("touch file.md", &mut fs);
    exec("write file.md content", &mut fs);
    exec("mkdir dir", &mut fs);
    exec("mv file.md dir/file.md", &mut fs);
    assert!(!exec("ls", &mut fs).contains("file.md"));
    assert_eq!(exec("cat dir/file.md", &mut fs), "content");
}

#[test]
fn test_mv_directory() {
    let mut fs = VirtualFs::new();
    exec("mkdir src", &mut fs);
    exec("mkdir dst", &mut fs);
    exec("mv src dst", &mut fs);
    let ls = exec("ls dst", &mut fs);
    assert!(ls.contains("src/"));
}

#[test]
fn test_cp() {
    let mut fs = VirtualFs::new();
    exec("touch a.md", &mut fs);
    exec("write a.md content", &mut fs);
    exec("cp a.md b.md", &mut fs);
    let output = exec("cat b.md", &mut fs);
    assert_eq!(output, "content");
}

#[test]
fn test_cp_independent_copy() {
    let mut fs = VirtualFs::new();
    exec("touch a.md", &mut fs);
    exec("write a.md original", &mut fs);
    exec("cp a.md b.md", &mut fs);
    exec("write a.md modified", &mut fs);
    assert_eq!(exec("cat a.md", &mut fs), "modified");
    assert_eq!(exec("cat b.md", &mut fs), "original");
}

#[test]
fn test_cp_into_directory() {
    let mut fs = VirtualFs::new();
    exec("touch file.md", &mut fs);
    exec("write file.md content", &mut fs);
    exec("mkdir dir", &mut fs);
    exec("cp file.md dir/file.md", &mut fs);
    assert_eq!(exec("cat dir/file.md", &mut fs), "content");
    // Original still exists
    assert_eq!(exec("cat file.md", &mut fs), "content");
}

#[test]
fn test_cp_nonexistent_src_fails() {
    let mut fs = VirtualFs::new();
    let err = exec_err("cp ghost.md dest.md", &mut fs);
    assert!(err.contains("no such file"));
}
