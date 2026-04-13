use super::*;
use markdownfs::vcs::Vcs;

#[test]
fn test_commit_and_log() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    exec("touch readme.md", &mut fs);
    exec("write readme.md # Hello", &mut fs);

    let id = vcs.commit(&fs, "initial commit", "root").unwrap();
    assert!(!id.to_hex().is_empty());

    let log = vcs.log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].message, "initial commit");
    assert_eq!(log[0].author, "root");
}

#[test]
fn test_commit_and_revert() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    exec("touch a.md", &mut fs);
    exec("write a.md version1", &mut fs);
    let id1 = vcs.commit(&fs, "v1", "root").unwrap();

    exec("write a.md version2", &mut fs);
    exec("touch b.md", &mut fs);
    exec("write b.md extra", &mut fs);
    let _id2 = vcs.commit(&fs, "v2", "root").unwrap();

    assert_eq!(exec("cat a.md", &mut fs), "version2");
    assert!(exec("ls", &mut fs).contains("b.md"));

    vcs.revert(&mut fs, &id1.short_hex()).unwrap();

    assert_eq!(exec("cat a.md", &mut fs), "version1");
    assert!(!exec("ls", &mut fs).contains("b.md"));
}

#[test]
fn test_multiple_commits_and_log() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    exec("touch a.md", &mut fs);
    vcs.commit(&fs, "first", "root").unwrap();

    exec("touch b.md", &mut fs);
    vcs.commit(&fs, "second", "root").unwrap();

    exec("touch c.md", &mut fs);
    vcs.commit(&fs, "third", "root").unwrap();

    let log = vcs.log();
    assert_eq!(log.len(), 3);
    assert_eq!(log[0].message, "third");
    assert_eq!(log[1].message, "second");
    assert_eq!(log[2].message, "first");
}

#[test]
fn test_deduplication() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    exec("touch a.md", &mut fs);
    exec("write a.md same content", &mut fs);
    exec("touch b.md", &mut fs);
    exec("write b.md same content", &mut fs);

    vcs.commit(&fs, "dedup test", "root").unwrap();

    let count = vcs.store.object_count();
    assert!(count > 0);
}

#[test]
fn test_revert_to_first_of_many_commits() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    exec("touch file.md", &mut fs);
    exec("write file.md v1", &mut fs);
    let id1 = vcs.commit(&fs, "c1", "root").unwrap();

    for i in 2..=10 {
        exec(&format!("write file.md v{i}"), &mut fs);
        vcs.commit(&fs, &format!("c{i}"), "root").unwrap();
    }

    assert_eq!(exec("cat file.md", &mut fs), "v10");
    vcs.revert(&mut fs, &id1.short_hex()).unwrap();
    assert_eq!(exec("cat file.md", &mut fs), "v1");
}

#[test]
fn test_commit_with_directories_and_revert() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    exec("mkdir -p src/lib", &mut fs);
    exec("cd src/lib", &mut fs);
    exec("touch module.md", &mut fs);
    exec("write module.md # Module", &mut fs);
    exec("cd /", &mut fs);
    exec("touch readme.md", &mut fs);
    exec("write readme.md # Project", &mut fs);

    let id1 = vcs.commit(&fs, "project structure", "root").unwrap();

    exec("rm -r src", &mut fs);
    exec("write readme.md # Changed", &mut fs);
    vcs.commit(&fs, "destructive change", "root").unwrap();

    vcs.revert(&mut fs, &id1.short_hex()).unwrap();

    assert_eq!(exec("cat readme.md", &mut fs), "# Project");
    assert_eq!(exec("cat src/lib/module.md", &mut fs), "# Module");
}

#[test]
fn test_commit_preserves_author() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    exec("touch file.md", &mut fs);
    vcs.commit(&fs, "by alice", "alice").unwrap();
    vcs.commit(&fs, "by bob", "bob").unwrap();

    let log = vcs.log();
    assert_eq!(log[0].author, "bob");
    assert_eq!(log[1].author, "alice");
}

#[test]
fn test_dedup_identical_files() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    let content = "# Same template\n\nIdentical everywhere.\n";
    for i in 0..100 {
        let path = format!("dup_{i:03}.md");
        fs.touch(&path, 0, 0).unwrap();
        fs.write_file(&path, content.as_bytes().to_vec()).unwrap();
    }

    vcs.commit(&fs, "dedup test", "root").unwrap();
    let count = vcs.store.object_count();
    // 1 blob + 1 tree + 1 commit = 3 objects (not 100+)
    assert!(count < 10, "expected dedup, got {count} objects for 100 identical files");
}

#[test]
fn test_dedup_after_modification() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    exec("touch a.md", &mut fs);
    exec("write a.md shared", &mut fs);
    exec("touch b.md", &mut fs);
    exec("write b.md shared", &mut fs);
    vcs.commit(&fs, "c1", "root").unwrap();

    exec("write a.md different", &mut fs);
    vcs.commit(&fs, "c2", "root").unwrap();

    // a.md has a new blob, b.md still references the old one
    let count = vcs.store.object_count();
    assert!(count > 3, "should have new blob for 'different'");
}
