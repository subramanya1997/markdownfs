use super::*;

#[test]
fn perf_grep_across_1k_files() {
    let mut fs = VirtualFs::new();
    let file_count = 1_000;

    for i in 0..file_count {
        let path = format!("s_{i:04}.md");
        fs.touch(&path, 0, 0).unwrap();
        let content = if i % 10 == 0 {
            format!("# File {i}\n\nTODO: fix this issue\n\nSome other content.\n")
        } else {
            format!("# File {i}\n\nEverything is fine here.\n\nNo issues.\n")
        };
        fs.write_file(&path, content.into_bytes()).unwrap();
    }

    let iterations = 50;
    let start = Instant::now();
    for _ in 0..iterations {
        let results = fs.grep("TODO", None, true, None).unwrap();
        assert_eq!(results.len(), 100);
    }
    let elapsed = start.elapsed();

    print_result(
        &format!("grep -r TODO (1K files, {iterations}x)"),
        iterations,
        elapsed,
    );
    assert!(elapsed.as_secs() < debug_limit(10), "too slow: {elapsed:?}");
}

#[test]
fn perf_grep_complex_regex() {
    let mut fs = VirtualFs::new();
    let file_count = 500;

    for i in 0..file_count {
        let path = format!("r_{i:04}.md");
        fs.touch(&path, 0, 0).unwrap();
        let content = format!(
            "# File {i}\n\nDate: 2024-01-{:02}\nEmail: user{i}@example.com\nVersion: v{}.{}.0\n",
            (i % 28) + 1, i / 100, i % 100
        );
        fs.write_file(&path, content.into_bytes()).unwrap();
    }

    let iterations = 20;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = fs.grep(r"\d{4}-\d{2}-\d{2}", None, true, None).unwrap();
    }
    let elapsed = start.elapsed();

    print_result(
        &format!("grep -r (complex regex, {file_count} files, {iterations}x)"),
        iterations,
        elapsed,
    );
    assert!(elapsed.as_secs() < debug_limit(10), "too slow: {elapsed:?}");
}

#[test]
fn perf_find_across_tree() {
    let mut fs = VirtualFs::new();

    for d in 0..10 {
        fs.mkdir(&format!("dir_{d}"), 0, 0).unwrap();
        for f in 0..100 {
            let path = format!("dir_{d}/file_{f:03}.md");
            fs.touch(&path, 0, 0).unwrap();
        }
    }

    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let results = fs.find(Some("."), Some("*.md"), None).unwrap();
        assert_eq!(results.len(), 1000);
    }
    let elapsed = start.elapsed();

    print_result(
        &format!("find -name *.md (1K files, {iterations}x)"),
        iterations,
        elapsed,
    );
    assert!(elapsed.as_secs() < debug_limit(60), "too slow: {elapsed:?}");
}

#[test]
fn perf_tree_large_hierarchy() {
    let mut fs = VirtualFs::new();

    for d in 0..20 {
        fs.mkdir(&format!("dir_{d:02}"), 0, 0).unwrap();
        for sd in 0..5 {
            fs.mkdir(&format!("dir_{d:02}/sub_{sd}"), 0, 0).unwrap();
            for f in 0..10 {
                fs.touch(&format!("dir_{d:02}/sub_{sd}/file_{f}.md"), 0, 0).unwrap();
            }
        }
    }

    let iterations = 50;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = fs.tree(None, "", None).unwrap();
    }
    let elapsed = start.elapsed();

    print_result(
        &format!("tree (20 dirs, 100 subdirs, 1K files, {iterations}x)"),
        iterations,
        elapsed,
    );
    assert!(elapsed.as_secs() < debug_limit(30), "too slow: {elapsed:?}");
}
