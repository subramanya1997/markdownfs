use super::*;

#[test]
fn perf_ls_large_dir() {
    let mut fs = VirtualFs::new();
    let file_count = 10_000;

    for i in 0..file_count {
        fs.touch(&format!("f_{i:05}.md"), 0, 0).unwrap();
    }

    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let entries = fs.ls(None).unwrap();
        assert_eq!(entries.len(), file_count);
    }
    let elapsed = start.elapsed();

    print_result(
        &format!("ls (dir with {file_count} entries, {iterations}x)"),
        iterations,
        elapsed,
    );
    assert!(elapsed.as_secs() < debug_limit(10), "too slow: {elapsed:?}");
}

#[test]
fn perf_ls_many_dirs() {
    let mut fs = VirtualFs::new();
    let dir_count = 100;
    let files_per_dir = 50;

    for d in 0..dir_count {
        fs.mkdir(&format!("dir_{d:03}"), 0, 0).unwrap();
        for f in 0..files_per_dir {
            fs.touch(&format!("dir_{d:03}/f_{f:03}.md"), 0, 0).unwrap();
        }
    }

    let iterations = 50;
    let start = Instant::now();
    for _ in 0..iterations {
        for d in 0..dir_count {
            let entries = fs.ls(Some(&format!("dir_{d:03}"))).unwrap();
            assert_eq!(entries.len(), files_per_dir);
        }
    }
    let elapsed = start.elapsed();

    print_result(
        &format!("ls ({dir_count} dirs x {files_per_dir} files, {iterations}x)"),
        iterations * dir_count,
        elapsed,
    );
    assert!(elapsed.as_secs() < debug_limit(30), "too slow: {elapsed:?}");
}

#[test]
fn perf_deep_path_resolution() {
    let mut fs = VirtualFs::new();
    let depth = 50;

    let mut path = String::new();
    for i in 0..depth {
        if i > 0 {
            path.push('/');
        }
        path.push_str(&format!("d{i}"));
    }
    fs.mkdir_p(&path, 0, 0).unwrap();
    let file_path = format!("{path}/deep.md");
    fs.touch(&file_path, 0, 0).unwrap();
    fs.write_file(&file_path, b"deep content".to_vec()).unwrap();

    let iterations = 10_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let data = fs.cat(&file_path).unwrap();
        assert_eq!(data, b"deep content");
    }
    let elapsed = start.elapsed();

    print_result(
        &format!("resolve path (depth={depth}, {iterations}x)"),
        iterations,
        elapsed,
    );
    assert!(elapsed.as_secs() < debug_limit(5), "too slow: {elapsed:?}");
}

#[test]
fn perf_shallow_path_resolution_100k() {
    let mut fs = VirtualFs::new();
    fs.mkdir("src", 0, 0).unwrap();
    fs.touch("src/main.md", 0, 0).unwrap();
    fs.write_file("src/main.md", b"# Main".to_vec()).unwrap();

    let count = 100_000;
    let start = Instant::now();
    for _ in 0..count {
        let _ = fs.cat("src/main.md").unwrap();
    }
    let elapsed = start.elapsed();

    print_result("resolve path (depth=1, 100K)", count, elapsed);
    assert!(elapsed.as_secs() < debug_limit(10), "too slow: {elapsed:?}");
}
