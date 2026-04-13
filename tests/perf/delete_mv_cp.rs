use super::*;

#[test]
fn perf_rm_10k() {
    let mut fs = VirtualFs::new();
    let count = 10_000;

    for i in 0..count {
        fs.touch(&format!("f_{i:05}.md"), 0, 0).unwrap();
    }

    let start = Instant::now();
    for i in 0..count {
        fs.rm(&format!("f_{i:05}.md")).unwrap();
    }
    let elapsed = start.elapsed();

    print_result("rm (10K file deletions)", count, elapsed);
    assert!(elapsed.as_secs() < debug_limit(10), "too slow: {elapsed:?}");

    let entries = fs.ls(None).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn perf_rm_rf_tree() {
    let mut fs = VirtualFs::new();

    // Create 100 dirs x 10 files each
    for d in 0..100 {
        fs.mkdir(&format!("d_{d:03}"), 0, 0).unwrap();
        for f in 0..10 {
            fs.touch(&format!("d_{d:03}/f_{f}.md"), 0, 0).unwrap();
        }
    }

    let start = Instant::now();
    for d in 0..100 {
        fs.rm_rf(&format!("d_{d:03}")).unwrap();
    }
    let elapsed = start.elapsed();

    print_result("rm -r (100 dirs x 10 files each)", 100, elapsed);
    assert!(elapsed.as_secs() < debug_limit(5), "too slow: {elapsed:?}");
}

#[test]
fn perf_mv_1k() {
    let mut fs = VirtualFs::new();
    let count = 1_000;

    fs.mkdir("src", 0, 0).unwrap();
    fs.mkdir("dst", 0, 0).unwrap();

    for i in 0..count {
        fs.touch(&format!("src/f_{i:04}.md"), 0, 0).unwrap();
    }

    let start = Instant::now();
    for i in 0..count {
        fs.mv(&format!("src/f_{i:04}.md"), &format!("dst/f_{i:04}.md")).unwrap();
    }
    let elapsed = start.elapsed();

    print_result("mv (1K file renames)", count, elapsed);
    assert!(elapsed.as_secs() < debug_limit(5), "too slow: {elapsed:?}");

    assert!(fs.ls(Some("src")).unwrap().is_empty());
    assert_eq!(fs.ls(Some("dst")).unwrap().len(), count);
}

#[test]
fn perf_cp_1k() {
    let mut fs = VirtualFs::new();
    let count = 1_000;

    for i in 0..count {
        let path = format!("orig_{i:04}.md");
        fs.touch(&path, 0, 0).unwrap();
        fs.write_file(&path, format!("Content {i}").into_bytes()).unwrap();
    }

    fs.mkdir("copies", 0, 0).unwrap();

    let start = Instant::now();
    for i in 0..count {
        fs.cp(&format!("orig_{i:04}.md"), &format!("copies/copy_{i:04}.md"), 0, 0).unwrap();
    }
    let elapsed = start.elapsed();

    print_result("cp (1K file copies)", count, elapsed);
    assert!(elapsed.as_secs() < debug_limit(5), "too slow: {elapsed:?}");
    assert_eq!(fs.ls(Some("copies")).unwrap().len(), count);
}
