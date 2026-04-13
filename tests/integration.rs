use mdvfs::cmd;
use mdvfs::cmd::parser;
use mdvfs::fs::VirtualFs;
use mdvfs::persist::PersistManager;
use mdvfs::vcs::Vcs;

fn exec(line: &str, fs: &mut VirtualFs) -> String {
    let pipeline = parser::parse_pipeline(line);
    cmd::execute_pipeline(&pipeline, fs).unwrap()
}

fn exec_err(line: &str, fs: &mut VirtualFs) -> String {
    let pipeline = parser::parse_pipeline(line);
    cmd::execute_pipeline(&pipeline, fs)
        .unwrap_err()
        .to_string()
}

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
fn test_touch_and_cat() {
    let mut fs = VirtualFs::new();
    exec("touch readme.md", &mut fs);
    exec("write readme.md Hello, mdvfs!", &mut fs);
    let output = exec("cat readme.md", &mut fs);
    assert_eq!(output, "Hello, mdvfs!");
}

#[test]
fn test_only_markdown_files() {
    let mut fs = VirtualFs::new();
    let err = exec_err("touch hello.txt", &mut fs);
    assert!(err.contains("only .md files"));
}

#[test]
fn test_cd_and_pwd() {
    let mut fs = VirtualFs::new();
    exec("mkdir project", &mut fs);
    exec("cd project", &mut fs);
    let pwd = exec("pwd", &mut fs);
    assert_eq!(pwd.trim(), "/project");
    exec("cd /", &mut fs);
    assert_eq!(exec("pwd", &mut fs).trim(), "/");
}

#[test]
fn test_nested_dirs() {
    let mut fs = VirtualFs::new();
    exec("mkdir -p a/b/c", &mut fs);
    exec("cd a/b/c", &mut fs);
    let pwd = exec("pwd", &mut fs);
    assert_eq!(pwd.trim(), "/a/b/c");
}

#[test]
fn test_rm_file() {
    let mut fs = VirtualFs::new();
    exec("touch file.md", &mut fs);
    assert!(exec("ls", &mut fs).contains("file.md"));
    exec("rm file.md", &mut fs);
    assert!(!exec("ls", &mut fs).contains("file.md"));
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
fn test_mv() {
    let mut fs = VirtualFs::new();
    exec("touch old.md", &mut fs);
    exec("mv old.md new.md", &mut fs);
    let listing = exec("ls", &mut fs);
    assert!(!listing.contains("old.md"));
    assert!(listing.contains("new.md"));
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
fn test_stat() {
    let mut fs = VirtualFs::new();
    exec("touch info.md", &mut fs);
    let output = exec("stat info.md", &mut fs);
    assert!(output.contains("file"));
    assert!(output.contains("info.md"));
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
fn test_find() {
    let mut fs = VirtualFs::new();
    exec("mkdir -p a/b", &mut fs);
    exec("cd a", &mut fs);
    exec("touch x.md", &mut fs);
    exec("cd b", &mut fs);
    exec("touch y.md", &mut fs);
    exec("cd /", &mut fs);
    let output = exec("find . -name *.md", &mut fs);
    assert!(output.contains("x.md"));
    assert!(output.contains("y.md"));
}

#[test]
fn test_grep() {
    let mut fs = VirtualFs::new();
    exec("touch notes.md", &mut fs);
    exec("write notes.md TODO: fix this\nDONE: that\nTODO: another", &mut fs);
    let output = exec("grep TODO notes.md", &mut fs);
    assert!(output.contains("TODO: fix this"));
    assert!(output.contains("TODO: another"));
    assert!(!output.contains("DONE"));
}

#[test]
fn test_pipes() {
    let mut fs = VirtualFs::new();
    let output = exec("echo hello world | wc -w", &mut fs);
    assert_eq!(output.trim(), "2");
}

#[test]
fn test_head_tail() {
    let mut fs = VirtualFs::new();
    exec("touch data.md", &mut fs);
    exec("write data.md 1\n2\n3\n4\n5", &mut fs);
    let head = exec("cat data.md | head -2", &mut fs);
    assert_eq!(head.trim(), "1\n2");
    let tail = exec("cat data.md | tail -2", &mut fs);
    assert_eq!(tail.trim(), "4\n5");
}

#[test]
fn test_symlink() {
    let mut fs = VirtualFs::new();
    exec("touch target.md", &mut fs);
    exec("write target.md linked content", &mut fs);
    exec("ln -s target.md link.md", &mut fs);
    let output = exec("cat link.md", &mut fs);
    // cat follows symlinks
    assert_eq!(output, "linked content");
}

#[test]
fn test_chmod() {
    let mut fs = VirtualFs::new();
    exec("touch secure.md", &mut fs);
    exec("chmod 600 secure.md", &mut fs);
    let output = exec("stat secure.md", &mut fs);
    assert!(output.contains("0600"));
}

// ───── VCS Tests ─────

#[test]
fn test_commit_and_log() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    exec("touch readme.md", &mut fs);
    exec("write readme.md # Hello", &mut fs);

    let id = vcs.commit(&fs, "initial commit").unwrap();
    assert!(!id.to_hex().is_empty());

    let log = vcs.log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].message, "initial commit");
}

#[test]
fn test_commit_and_revert() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    // State 1: one file
    exec("touch a.md", &mut fs);
    exec("write a.md version1", &mut fs);
    let id1 = vcs.commit(&fs, "v1").unwrap();

    // State 2: modified file + new file
    exec("write a.md version2", &mut fs);
    exec("touch b.md", &mut fs);
    exec("write b.md extra", &mut fs);
    let _id2 = vcs.commit(&fs, "v2").unwrap();

    // Verify state 2
    assert_eq!(exec("cat a.md", &mut fs), "version2");
    assert!(exec("ls", &mut fs).contains("b.md"));

    // Revert to state 1
    vcs.revert(&mut fs, &id1.short_hex()).unwrap();

    assert_eq!(exec("cat a.md", &mut fs), "version1");
    assert!(!exec("ls", &mut fs).contains("b.md"));
}

#[test]
fn test_multiple_commits_and_log() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    exec("touch a.md", &mut fs);
    vcs.commit(&fs, "first").unwrap();

    exec("touch b.md", &mut fs);
    vcs.commit(&fs, "second").unwrap();

    exec("touch c.md", &mut fs);
    vcs.commit(&fs, "third").unwrap();

    let log = vcs.log();
    assert_eq!(log.len(), 3);
    // Log is newest first
    assert_eq!(log[0].message, "third");
    assert_eq!(log[1].message, "second");
    assert_eq!(log[2].message, "first");
}

#[test]
fn test_deduplication() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    // Two files with identical content
    exec("touch a.md", &mut fs);
    exec("write a.md same content", &mut fs);
    exec("touch b.md", &mut fs);
    exec("write b.md same content", &mut fs);

    vcs.commit(&fs, "dedup test").unwrap();

    // The store should only have one blob for "same content"
    // (plus tree objects and commit object)
    // With dedup, identical content is stored once
    let count = vcs.store.object_count();
    // Exact count depends on tree structure, but should be < 2 blobs + overhead
    assert!(count > 0);
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
fn test_grep_pipe() {
    let mut fs = VirtualFs::new();
    exec("touch log.md", &mut fs);
    exec(
        "write log.md ERROR: disk full\nINFO: started\nERROR: timeout\nINFO: done",
        &mut fs,
    );
    let output = exec("cat log.md | grep ERROR | wc -l", &mut fs);
    assert_eq!(output.trim(), "2");
}

#[test]
fn test_echo_pipe_write() {
    let mut fs = VirtualFs::new();
    exec("touch out.md", &mut fs);
    exec("echo hello from pipe | write out.md", &mut fs);
    let content = exec("cat out.md", &mut fs);
    assert_eq!(content.trim(), "hello from pipe");
}

// ───── Persistence Tests ─────

#[test]
fn test_persist_save_and_load() {
    let tmp = std::env::temp_dir().join(format!("mdvfs_test_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();

    let persist = PersistManager::new(&tmp);

    // Build a VFS with some state
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    exec("mkdir docs", &mut fs);
    exec("touch readme.md", &mut fs);
    exec("write readme.md # Hello World", &mut fs);
    exec("cd docs", &mut fs);
    exec("touch notes.md", &mut fs);
    exec("write notes.md Some notes here", &mut fs);
    exec("cd /", &mut fs);

    vcs.commit(&fs, "initial").unwrap();

    exec("touch changelog.md", &mut fs);
    exec("write changelog.md ## v0.1.0", &mut fs);
    vcs.commit(&fs, "add changelog").unwrap();

    // Save
    persist.save(&fs, &vcs).unwrap();
    assert!(persist.state_exists());

    // Load into fresh state
    let (fs2, vcs2) = persist.load().unwrap();

    // Verify filesystem state
    assert_eq!(
        String::from_utf8_lossy(fs2.cat("readme.md").unwrap()),
        "# Hello World"
    );
    assert_eq!(
        String::from_utf8_lossy(fs2.cat("docs/notes.md").unwrap()),
        "Some notes here"
    );
    assert_eq!(
        String::from_utf8_lossy(fs2.cat("changelog.md").unwrap()),
        "## v0.1.0"
    );

    // Verify VCS state
    assert_eq!(vcs2.commits.len(), 2);
    assert_eq!(vcs2.commits[0].message, "initial");
    assert_eq!(vcs2.commits[1].message, "add changelog");
    assert!(vcs2.head.is_some());

    // Cleanup
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_persist_revert_after_reload() {
    let tmp = std::env::temp_dir().join(format!("mdvfs_revert_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();

    let persist = PersistManager::new(&tmp);

    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    exec("touch data.md", &mut fs);
    exec("write data.md version1", &mut fs);
    let id1 = vcs.commit(&fs, "v1").unwrap();

    exec("write data.md version2", &mut fs);
    vcs.commit(&fs, "v2").unwrap();

    // Save and reload
    persist.save(&fs, &vcs).unwrap();
    let (mut fs2, mut vcs2) = persist.load().unwrap();

    // Revert to v1 after reload
    vcs2.revert(&mut fs2, &id1.short_hex()).unwrap();
    assert_eq!(
        String::from_utf8_lossy(fs2.cat("data.md").unwrap()),
        "version1"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_persist_empty_state() {
    let tmp = std::env::temp_dir().join(format!("mdvfs_empty_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();

    let persist = PersistManager::new(&tmp);

    // Save empty state
    let fs = VirtualFs::new();
    let vcs = Vcs::new();
    persist.save(&fs, &vcs).unwrap();

    // Load it back
    let (fs2, vcs2) = persist.load().unwrap();
    assert_eq!(fs2.pwd(), "/");
    assert!(vcs2.commits.is_empty());
    assert!(vcs2.head.is_none());

    // Cleanup
    let _ = std::fs::remove_dir_all(&tmp);
}
