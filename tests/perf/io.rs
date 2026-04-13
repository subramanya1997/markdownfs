use super::*;

#[test]
fn perf_write_small_files_10k() {
    let mut fs = VirtualFs::new();
    let count = 10_000;

    for i in 0..count {
        fs.touch(&format!("f_{i:05}.md"), 0, 0).unwrap();
    }

    let content = "# Title\n\nSome markdown content.\n\n- Item 1\n- Item 2\n- Item 3\n";

    let start = Instant::now();
    for i in 0..count {
        let path = format!("f_{i:05}.md");
        fs.write_file(&path, content.as_bytes().to_vec()).unwrap();
    }
    let elapsed = start.elapsed();

    print_result("write (10K files, 60B each)", count, elapsed);
    assert!(elapsed.as_secs() < debug_limit(5), "too slow: {elapsed:?}");
}

#[test]
fn perf_write_large_files_1k() {
    let mut fs = VirtualFs::new();
    let count = 1_000;

    let mut content = String::with_capacity(10_240);
    for i in 0..200 {
        content.push_str(&format!("## Section {i}\n\nLorem ipsum dolor sit amet.\n\n"));
    }

    for i in 0..count {
        fs.touch(&format!("big_{i:04}.md"), 0, 0).unwrap();
    }

    let start = Instant::now();
    for i in 0..count {
        let path = format!("big_{i:04}.md");
        fs.write_file(&path, content.as_bytes().to_vec()).unwrap();
    }
    let elapsed = start.elapsed();

    let total_mb = (count * content.len()) as f64 / (1024.0 * 1024.0);
    print_result(
        &format!("write (1K files, 10KB each, {total_mb:.1}MB total)"),
        count,
        elapsed,
    );
    assert!(elapsed.as_secs() < debug_limit(5), "too slow: {elapsed:?}");
}

#[test]
fn perf_write_100kb_files() {
    let mut fs = VirtualFs::new();
    let count = 100;

    let mut content = String::with_capacity(100_000);
    for i in 0..2000 {
        content.push_str(&format!("## Section {i}\n\nLorem ipsum dolor sit amet, consectetur adipiscing elit.\n\n"));
    }

    for i in 0..count {
        fs.touch(&format!("huge_{i:03}.md"), 0, 0).unwrap();
    }

    let start = Instant::now();
    for i in 0..count {
        fs.write_file(&format!("huge_{i:03}.md"), content.as_bytes().to_vec()).unwrap();
    }
    let elapsed = start.elapsed();

    let total_mb = (count * content.len()) as f64 / (1024.0 * 1024.0);
    print_result(
        &format!("write (100 files, ~100KB each, {total_mb:.1}MB total)"),
        count,
        elapsed,
    );
    assert!(elapsed.as_secs() < debug_limit(5), "too slow: {elapsed:?}");
}

#[test]
fn perf_overwrite_same_file_10k() {
    let mut fs = VirtualFs::new();
    fs.touch("target.md", 0, 0).unwrap();
    let count = 10_000;

    let start = Instant::now();
    for i in 0..count {
        fs.write_file("target.md", format!("version {i}\n").into_bytes()).unwrap();
    }
    let elapsed = start.elapsed();

    print_result("overwrite same file (10K times)", count, elapsed);
    assert!(elapsed.as_secs() < debug_limit(5), "too slow: {elapsed:?}");
}

#[test]
fn perf_cat_10k() {
    let mut fs = VirtualFs::new();
    let count = 10_000;
    let content = "# Hello\n\nWorld\n";

    for i in 0..count {
        let path = format!("r_{i:05}.md");
        fs.touch(&path, 0, 0).unwrap();
        fs.write_file(&path, content.as_bytes().to_vec()).unwrap();
    }

    let start = Instant::now();
    for i in 0..count {
        let path = format!("r_{i:05}.md");
        let data = fs.cat(&path).unwrap();
        assert_eq!(data.len(), content.len());
    }
    let elapsed = start.elapsed();

    print_result("cat (10K file reads)", count, elapsed);
    assert!(elapsed.as_secs() < debug_limit(5), "too slow: {elapsed:?}");
}

#[test]
fn perf_cat_same_file_100k() {
    let mut fs = VirtualFs::new();
    fs.touch("hot.md", 0, 0).unwrap();
    fs.write_file("hot.md", b"# Hot path content\n\nFrequently read.\n".to_vec()).unwrap();

    let count = 100_000;
    let start = Instant::now();
    for _ in 0..count {
        let _ = fs.cat("hot.md").unwrap();
    }
    let elapsed = start.elapsed();

    print_result("cat same file (100K reads, hot path)", count, elapsed);
    assert!(elapsed.as_secs() < debug_limit(10), "too slow: {elapsed:?}");
}
