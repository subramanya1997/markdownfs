use super::*;

#[test]
fn perf_file_creation_10k() {
    let mut fs = VirtualFs::new();
    let count = 10_000;

    let start = Instant::now();
    for i in 0..count {
        let path = format!("file_{i:05}.md");
        fs.touch(&path, 0, 0).unwrap();
    }
    let elapsed = start.elapsed();

    print_result("touch (10K files, flat dir)", count, elapsed);
    assert!(elapsed.as_secs() < debug_limit(5), "too slow: {elapsed:?}");
}

#[test]
fn perf_file_creation_nested_10k() {
    let mut fs = VirtualFs::new();
    let count = 10_000;

    for i in 0..100 {
        fs.mkdir(&format!("dir_{i:03}"), 0, 0).unwrap();
    }

    let start = Instant::now();
    for i in 0..count {
        let dir = i % 100;
        let path = format!("dir_{dir:03}/file_{i:05}.md");
        fs.touch(&path, 0, 0).unwrap();
    }
    let elapsed = start.elapsed();

    print_result("touch (10K files, 100 dirs)", count, elapsed);
    assert!(elapsed.as_secs() < debug_limit(5), "too slow: {elapsed:?}");
}

#[test]
fn perf_file_creation_50k_flat() {
    let mut fs = VirtualFs::new();
    let count = 50_000;

    let start = Instant::now();
    for i in 0..count {
        fs.touch(&format!("f_{i:06}.md"), 0, 0).unwrap();
    }
    let elapsed = start.elapsed();

    print_result("touch (50K files, flat dir)", count, elapsed);
    assert!(elapsed.as_secs() < debug_limit(30), "too slow: {elapsed:?}");
}

#[test]
fn perf_mkdir_p_deep() {
    let mut fs = VirtualFs::new();
    let count = 100;

    let start = Instant::now();
    for i in 0..count {
        let path = format!("root_{i}/a/b/c/d/e/f/g/h/i/j");
        fs.mkdir_p(&path, 0, 0).unwrap();
    }
    let elapsed = start.elapsed();

    print_result(
        &format!("mkdir -p (depth=10, {count} trees)"),
        count,
        elapsed,
    );
    assert!(elapsed.as_secs() < debug_limit(5), "too slow: {elapsed:?}");
}
