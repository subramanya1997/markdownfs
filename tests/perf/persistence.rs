use super::*;
use markdownfs::persist::PersistManager;
use markdownfs::vcs::Vcs;

#[test]
fn perf_persist_save_load_10k() {
    let tmp = std::env::temp_dir().join(format!("markdownfs_perf_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let persist = PersistManager::new(&tmp);

    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();
    let file_count = 10_000;

    let content = "# Perf\n\nBenchmark file.\n";
    for i in 0..file_count {
        let path = format!("p_{i:05}.md");
        fs.touch(&path, 0, 0).unwrap();
        fs.write_file(&path, content.as_bytes().to_vec()).unwrap();
    }
    vcs.commit(&fs, "bulk", "root").unwrap();

    let start = Instant::now();
    persist.save(&fs, &vcs).unwrap();
    let save_elapsed = start.elapsed();

    let file_size = std::fs::metadata(tmp.join(".vfs/state.bin"))
        .unwrap()
        .len();
    print_result(&format!("save ({file_count} files)"), 1, save_elapsed);
    println!(
        "    state.bin size: {:.2} MB",
        file_size as f64 / (1024.0 * 1024.0)
    );

    let start = Instant::now();
    let (fs2, vcs2) = persist.load().unwrap();
    let load_elapsed = start.elapsed();

    print_result(&format!("load ({file_count} files)"), 1, load_elapsed);
    assert_eq!(vcs2.commits.len(), 1);

    let data = fs2.cat("p_00000.md").unwrap();
    assert_eq!(String::from_utf8_lossy(data), content);

    assert!(save_elapsed.as_secs() < debug_limit(10), "save too slow: {save_elapsed:?}");
    assert!(load_elapsed.as_secs() < debug_limit(10), "load too slow: {load_elapsed:?}");

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn perf_persist_large_state() {
    let tmp = std::env::temp_dir().join(format!("markdownfs_perflg_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let persist = PersistManager::new(&tmp);

    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    // Create a substantial state: dirs + files + multiple commits
    for d in 0..10 {
        fs.mkdir(&format!("dir_{d}"), 0, 0).unwrap();
        for f in 0..100 {
            let path = format!("dir_{d}/f_{f:03}.md");
            fs.touch(&path, 0, 0).unwrap();
            fs.write_file(&path, format!("# File {d}/{f}\nContent.\n").into_bytes()).unwrap();
        }
    }
    for c in 0..10 {
        let path = format!("dir_{}/f_{:03}.md", c % 10, c);
        fs.write_file(&path, format!("# Updated at commit {c}\n").into_bytes()).unwrap();
        vcs.commit(&fs, &format!("commit {c}"), "root").unwrap();
    }

    let start = Instant::now();
    persist.save(&fs, &vcs).unwrap();
    let save_elapsed = start.elapsed();

    let start = Instant::now();
    let (_, vcs2) = persist.load().unwrap();
    let load_elapsed = start.elapsed();

    print_result("save (1K files + 10 commits)", 1, save_elapsed);
    print_result("load (1K files + 10 commits)", 1, load_elapsed);
    assert_eq!(vcs2.commits.len(), 10);

    assert!(save_elapsed.as_secs() < debug_limit(10), "save too slow");
    assert!(load_elapsed.as_secs() < debug_limit(10), "load too slow");

    let _ = std::fs::remove_dir_all(&tmp);
}
