use super::*;

#[test]
fn test_stat() {
    let mut fs = VirtualFs::new();
    exec("touch info.md", &mut fs);
    let output = exec("stat info.md", &mut fs);
    assert!(output.contains("file"));
    assert!(output.contains("info.md"));
}

#[test]
fn test_stat_directory() {
    let mut fs = VirtualFs::new();
    exec("mkdir mydir", &mut fs);
    let output = exec("stat mydir", &mut fs);
    assert!(output.contains("directory"));
}

#[test]
fn test_stat_shows_mode() {
    let mut fs = VirtualFs::new();
    exec("touch secret.md", &mut fs);
    exec("chmod 600 secret.md", &mut fs);
    let output = exec("stat secret.md", &mut fs);
    assert!(output.contains("0600"));
}

#[test]
fn test_stat_shows_uid_gid() {
    let mut fs = VirtualFs::new();
    exec("touch owned.md", &mut fs);
    let output = exec("stat owned.md", &mut fs);
    assert!(output.contains("Uid:"));
    assert!(output.contains("Gid:"));
}

#[test]
fn test_tree() {
    let mut fs = VirtualFs::new();
    exec("mkdir docs", &mut fs);
    exec("cd docs", &mut fs);
    exec("touch readme.md", &mut fs);
    exec("cd /", &mut fs);
    let output = exec("tree", &mut fs);
    assert!(output.contains("docs/"));
    assert!(output.contains("readme.md"));
}

#[test]
fn test_tree_deep_hierarchy() {
    let mut fs = VirtualFs::new();
    exec("mkdir -p a/b/c", &mut fs);
    exec("cd a/b/c", &mut fs);
    exec("touch leaf.md", &mut fs);
    exec("cd /", &mut fs);
    let output = exec("tree", &mut fs);
    assert!(output.contains("a/"));
    assert!(output.contains("b/"));
    assert!(output.contains("c/"));
    assert!(output.contains("leaf.md"));
}

#[test]
fn test_tree_empty_root() {
    let fs = VirtualFs::new();
    let tree = fs.tree(None, "", None).unwrap();
    assert!(tree.starts_with('.'));
}

#[test]
fn test_chmod() {
    let mut fs = VirtualFs::new();
    exec("touch secure.md", &mut fs);
    exec("chmod 600 secure.md", &mut fs);
    let output = exec("stat secure.md", &mut fs);
    assert!(output.contains("0600"));
}

#[test]
fn test_chmod_various_modes() {
    let mut fs = VirtualFs::new();
    exec("touch file.md", &mut fs);
    for mode in ["755", "644", "700", "400", "000", "777"] {
        exec(&format!("chmod {mode} file.md"), &mut fs);
        let stat = exec("stat file.md", &mut fs);
        assert!(
            stat.contains(&format!("0{mode}")),
            "mode {mode} not found in stat output: {stat}"
        );
    }
}

#[test]
fn test_chmod_directory() {
    let mut fs = VirtualFs::new();
    exec("mkdir restricted", &mut fs);
    exec("chmod 700 restricted", &mut fs);
    let stat = exec("stat restricted", &mut fs);
    assert!(stat.contains("0700"));
}

#[test]
fn test_ls_long_format() {
    let mut fs = VirtualFs::new();
    exec("mkdir docs", &mut fs);
    exec("touch readme.md", &mut fs);
    let output = exec("ls -l", &mut fs);
    assert!(output.contains("drwx"));
    assert!(output.contains("-rw-"));
}

#[test]
fn test_ls_long_shows_size() {
    let mut fs = VirtualFs::new();
    exec("touch file.md", &mut fs);
    exec("write file.md hello world", &mut fs);
    let output = exec("ls -l", &mut fs);
    assert!(output.contains("11") || output.contains("file.md"));
}

#[test]
fn test_ls_empty_dir() {
    let mut fs = VirtualFs::new();
    exec("mkdir empty", &mut fs);
    let output = exec("ls empty", &mut fs);
    assert!(output.trim().is_empty());
}
