use super::*;
use markdownfs::vcs::Vcs;

#[test]
fn perf_commit_10k_files() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();
    let file_count = 10_000;

    let content = "# Title\n\nContent here.\n";
    for i in 0..file_count {
        let path = format!("f_{i:05}.md");
        fs.touch(&path, 0, 0).unwrap();
        fs.write_file(&path, content.as_bytes().to_vec()).unwrap();
    }

    let start = Instant::now();
    let id = vcs.commit(&fs, "big commit", "root").unwrap();
    let elapsed = start.elapsed();

    print_result(
        &format!("commit ({file_count} files)"),
        1,
        elapsed,
    );
    println!("    commit hash: {}", id.short_hex());
    println!("    objects in store: {}", vcs.store.object_count());
    assert!(elapsed.as_secs() < debug_limit(10), "too slow: {elapsed:?}");
}

#[test]
fn perf_sequential_commits_100() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();
    let commit_count = 100;

    for i in 0..100 {
        let path = format!("f_{i:03}.md");
        fs.touch(&path, 0, 0).unwrap();
        fs.write_file(&path, format!("# File {i}\n").into_bytes())
            .unwrap();
    }

    let start = Instant::now();
    for c in 0..commit_count {
        let path = format!("f_{:03}.md", c % 100);
        fs.write_file(&path, format!("# File updated at commit {c}\n").into_bytes())
            .unwrap();
        vcs.commit(&fs, &format!("commit {c}"), "root").unwrap();
    }
    let elapsed = start.elapsed();

    print_result(
        &format!("{commit_count} sequential commits (100 files)"),
        commit_count,
        elapsed,
    );
    assert_eq!(vcs.log().len(), commit_count);
    assert!(elapsed.as_secs() < debug_limit(10), "too slow: {elapsed:?}");
}

#[test]
fn perf_commit_500_sequential() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    fs.touch("evolving.md", 0, 0).unwrap();
    let count = 500;

    let start = Instant::now();
    for c in 0..count {
        fs.write_file("evolving.md", format!("Content at commit {c}\n").into_bytes()).unwrap();
        vcs.commit(&fs, &format!("c{c}"), "root").unwrap();
    }
    let elapsed = start.elapsed();

    print_result(
        &format!("{count} sequential commits (single file)"),
        count,
        elapsed,
    );
    assert_eq!(vcs.log().len(), count);
    assert!(elapsed.as_secs() < debug_limit(20), "too slow: {elapsed:?}");
}

#[test]
fn perf_revert_large_state() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();
    let file_count = 5_000;

    let content = "# Original\n\nContent.\n";
    for i in 0..file_count {
        let path = format!("f_{i:05}.md");
        fs.touch(&path, 0, 0).unwrap();
        fs.write_file(&path, content.as_bytes().to_vec()).unwrap();
    }
    let id1 = vcs.commit(&fs, "v1", "root").unwrap();

    for i in 0..file_count {
        let path = format!("f_{i:05}.md");
        fs.write_file(&path, b"# Modified\n".to_vec()).unwrap();
    }
    vcs.commit(&fs, "v2", "root").unwrap();

    let start = Instant::now();
    vcs.revert(&mut fs, &id1.short_hex()).unwrap();
    let elapsed = start.elapsed();

    print_result(
        &format!("revert ({file_count} files)"),
        1,
        elapsed,
    );

    let data = fs.cat("f_00000.md").unwrap();
    assert_eq!(String::from_utf8_lossy(data), content);
    assert!(elapsed.as_secs() < debug_limit(10), "too slow: {elapsed:?}");
}

#[test]
fn perf_revert_chain() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    fs.touch("file.md", 0, 0).unwrap();
    let mut commit_ids = Vec::new();

    for c in 0..20 {
        fs.write_file("file.md", format!("v{c}\n").into_bytes()).unwrap();
        let id = vcs.commit(&fs, &format!("c{c}"), "root").unwrap();
        commit_ids.push(id);
    }

    let start = Instant::now();
    for (i, id) in commit_ids.iter().rev().enumerate() {
        vcs.revert(&mut fs, &id.short_hex()).unwrap();
        let expected = format!("v{}\n", 19 - i);
        assert_eq!(String::from_utf8_lossy(fs.cat("file.md").unwrap()), expected);
    }
    let elapsed = start.elapsed();

    print_result("20 reverts (walk history backward)", 20, elapsed);
    assert!(elapsed.as_secs() < debug_limit(5), "too slow: {elapsed:?}");
}

#[test]
fn perf_dedup_identical_files() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();
    let file_count = 10_000;

    let content = "# Shared Template\n\nThis content is identical everywhere.\n";
    for i in 0..file_count {
        let path = format!("dup_{i:05}.md");
        fs.touch(&path, 0, 0).unwrap();
        fs.write_file(&path, content.as_bytes().to_vec()).unwrap();
    }

    let start = Instant::now();
    vcs.commit(&fs, "dedup benchmark", "root").unwrap();
    let elapsed = start.elapsed();

    let object_count = vcs.store.object_count();
    print_result(
        &format!("commit {file_count} identical files (dedup)"),
        1,
        elapsed,
    );
    println!("    objects in store: {object_count} (expect <<{file_count} due to dedup)");

    assert!(
        object_count < 100,
        "dedup not working: {object_count} objects for {file_count} identical files"
    );
    assert!(elapsed.as_secs() < debug_limit(10), "too slow: {elapsed:?}");
}

#[test]
fn perf_dedup_mixed_content() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    let templates = [
        "# Template A\n\nShared content type A.\n",
        "# Template B\n\nShared content type B.\n",
        "# Template C\n\nShared content type C.\n",
    ];

    let file_count = 9_000;
    for i in 0..file_count {
        let path = format!("f_{i:05}.md");
        fs.touch(&path, 0, 0).unwrap();
        fs.write_file(&path, templates[i % 3].as_bytes().to_vec()).unwrap();
    }

    let start = Instant::now();
    vcs.commit(&fs, "mixed dedup", "root").unwrap();
    let elapsed = start.elapsed();

    let object_count = vcs.store.object_count();
    print_result(
        &format!("commit {file_count} files (3 unique contents)"),
        1,
        elapsed,
    );
    println!("    objects in store: {object_count} (expect ~5: 3 blobs + 1 tree + 1 commit)");

    assert!(
        object_count < 20,
        "dedup for 3 templates should yield few objects, got {object_count}"
    );
    assert!(elapsed.as_secs() < debug_limit(10), "too slow: {elapsed:?}");
}
