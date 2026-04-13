use mdvfs::cmd;
use mdvfs::cmd::parser;
use mdvfs::fs::VirtualFs;
use mdvfs::persist::PersistManager;
use mdvfs::vcs::Vcs;
use std::time::Instant;

fn exec(line: &str, fs: &mut VirtualFs) -> String {
    let pipeline = parser::parse_pipeline(line);
    cmd::execute_pipeline(&pipeline, fs).unwrap()
}

fn format_rate(count: usize, elapsed: std::time::Duration) -> String {
    let per_sec = count as f64 / elapsed.as_secs_f64();
    if per_sec > 1_000_000.0 {
        format!("{:.2}M ops/sec", per_sec / 1_000_000.0)
    } else if per_sec > 1_000.0 {
        format!("{:.2}K ops/sec", per_sec / 1_000.0)
    } else {
        format!("{:.2} ops/sec", per_sec)
    }
}

fn print_result(name: &str, count: usize, elapsed: std::time::Duration) {
    let per_op = elapsed / count as u32;
    println!(
        "  {:<40} {:>8} ops in {:>8.2?}  ({}, {:.2?}/op)",
        name,
        count,
        elapsed,
        format_rate(count, elapsed),
        per_op,
    );
}

// ─────────────────────────────────────────────
// File creation benchmarks
// ─────────────────────────────────────────────

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
    assert!(elapsed.as_secs() < 5, "too slow: {elapsed:?}");
}

#[test]
fn perf_file_creation_nested_10k() {
    let mut fs = VirtualFs::new();
    let count = 10_000;

    // Pre-create 100 directories
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
    assert!(elapsed.as_secs() < 5, "too slow: {elapsed:?}");
}

// ─────────────────────────────────────────────
// File write benchmarks
// ─────────────────────────────────────────────

#[test]
fn perf_write_small_files_10k() {
    let mut fs = VirtualFs::new();
    let count = 10_000;

    // Pre-create files
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
    assert!(elapsed.as_secs() < 5, "too slow: {elapsed:?}");
}

#[test]
fn perf_write_large_files_1k() {
    let mut fs = VirtualFs::new();
    let count = 1_000;

    // 10KB markdown content
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
    assert!(elapsed.as_secs() < 5, "too slow: {elapsed:?}");
}

// ─────────────────────────────────────────────
// File read benchmarks
// ─────────────────────────────────────────────

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
    assert!(elapsed.as_secs() < 5, "too slow: {elapsed:?}");
}

// ─────────────────────────────────────────────
// Directory listing benchmarks
// ─────────────────────────────────────────────

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
    assert!(elapsed.as_secs() < 10, "too slow: {elapsed:?}");
}

// ─────────────────────────────────────────────
// Path resolution benchmarks
// ─────────────────────────────────────────────

#[test]
fn perf_deep_path_resolution() {
    let mut fs = VirtualFs::new();
    let depth = 50;

    // Create a/b/c/d/.../file.md (50 levels deep)
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
    assert!(elapsed.as_secs() < 5, "too slow: {elapsed:?}");
}

// ─────────────────────────────────────────────
// Search benchmarks
// ─────────────────────────────────────────────

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
        let results = fs.grep("TODO", None, true).unwrap();
        assert_eq!(results.len(), 100); // 1000/10 = 100 matches
    }
    let elapsed = start.elapsed();

    print_result(
        &format!("grep -r TODO (1K files, {iterations}x)"),
        iterations,
        elapsed,
    );
    assert!(elapsed.as_secs() < 10, "too slow: {elapsed:?}");
}

#[test]
fn perf_find_across_tree() {
    let mut fs = VirtualFs::new();

    // Create a tree: 10 dirs x 100 files each
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
        let results = fs.find(Some("."), Some("*.md")).unwrap();
        assert_eq!(results.len(), 1000);
    }
    let elapsed = start.elapsed();

    print_result(
        &format!("find -name *.md (1K files, {iterations}x)"),
        iterations,
        elapsed,
    );
    assert!(elapsed.as_secs() < 10, "too slow: {elapsed:?}");
}

// ─────────────────────────────────────────────
// VCS benchmarks
// ─────────────────────────────────────────────

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
    let id = vcs.commit(&fs, "big commit").unwrap();
    let elapsed = start.elapsed();

    print_result(
        &format!("commit ({file_count} files)"),
        1,
        elapsed,
    );
    println!("    commit hash: {}", id.short_hex());
    println!("    objects in store: {}", vcs.store.object_count());
    assert!(elapsed.as_secs() < 10, "too slow: {elapsed:?}");
}

#[test]
fn perf_sequential_commits_100() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();
    let commit_count = 100;

    // Start with 100 files
    for i in 0..100 {
        let path = format!("f_{i:03}.md");
        fs.touch(&path, 0, 0).unwrap();
        fs.write_file(&path, format!("# File {i}\n").into_bytes())
            .unwrap();
    }

    let start = Instant::now();
    for c in 0..commit_count {
        // Modify one file per commit
        let path = format!("f_{:03}.md", c % 100);
        fs.write_file(&path, format!("# File updated at commit {c}\n").into_bytes())
            .unwrap();
        vcs.commit(&fs, &format!("commit {c}")).unwrap();
    }
    let elapsed = start.elapsed();

    print_result(
        &format!("{commit_count} sequential commits (100 files)"),
        commit_count,
        elapsed,
    );
    assert_eq!(vcs.log().len(), commit_count);
    assert!(elapsed.as_secs() < 10, "too slow: {elapsed:?}");
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
    let id1 = vcs.commit(&fs, "v1").unwrap();

    // Modify all files
    for i in 0..file_count {
        let path = format!("f_{i:05}.md");
        fs.write_file(&path, b"# Modified\n".to_vec()).unwrap();
    }
    vcs.commit(&fs, "v2").unwrap();

    let start = Instant::now();
    vcs.revert(&mut fs, &id1.short_hex()).unwrap();
    let elapsed = start.elapsed();

    print_result(
        &format!("revert ({file_count} files)"),
        1,
        elapsed,
    );

    // Verify correctness
    let data = fs.cat("f_00000.md").unwrap();
    assert_eq!(String::from_utf8_lossy(data), content);
    assert!(elapsed.as_secs() < 10, "too slow: {elapsed:?}");
}

// ─────────────────────────────────────────────
// Content deduplication benchmark
// ─────────────────────────────────────────────

#[test]
fn perf_dedup_identical_files() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();
    let file_count = 10_000;

    // All files have the same content — should dedup to 1 blob
    let content = "# Shared Template\n\nThis content is identical everywhere.\n";
    for i in 0..file_count {
        let path = format!("dup_{i:05}.md");
        fs.touch(&path, 0, 0).unwrap();
        fs.write_file(&path, content.as_bytes().to_vec()).unwrap();
    }

    let start = Instant::now();
    vcs.commit(&fs, "dedup benchmark").unwrap();
    let elapsed = start.elapsed();

    let object_count = vcs.store.object_count();
    print_result(
        &format!("commit {file_count} identical files (dedup)"),
        1,
        elapsed,
    );
    println!("    objects in store: {object_count} (expect <<{file_count} due to dedup)");

    // With perfect dedup, we should have:
    // 1 blob (shared content) + 1 tree (root dir) + 1 commit = 3 objects
    // In practice, tree serialization varies, but it should be WAY less than 10K
    assert!(
        object_count < 100,
        "dedup not working: {object_count} objects for {file_count} identical files"
    );
    assert!(elapsed.as_secs() < 10, "too slow: {elapsed:?}");
}

// ─────────────────────────────────────────────
// Persistence benchmarks
// ─────────────────────────────────────────────

#[test]
fn perf_persist_save_load_10k() {
    let tmp = std::env::temp_dir().join(format!("mdvfs_perf_{}", std::process::id()));
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
    vcs.commit(&fs, "bulk").unwrap();

    // Benchmark save
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

    // Benchmark load
    let start = Instant::now();
    let (fs2, vcs2) = persist.load().unwrap();
    let load_elapsed = start.elapsed();

    print_result(&format!("load ({file_count} files)"), 1, load_elapsed);
    assert_eq!(vcs2.commits.len(), 1);

    // Verify correctness
    let data = fs2.cat("p_00000.md").unwrap();
    assert_eq!(String::from_utf8_lossy(data), content);

    assert!(save_elapsed.as_secs() < 10, "save too slow: {save_elapsed:?}");
    assert!(load_elapsed.as_secs() < 10, "load too slow: {load_elapsed:?}");

    let _ = std::fs::remove_dir_all(&tmp);
}

// ─────────────────────────────────────────────
// Pipe benchmarks
// ─────────────────────────────────────────────

#[test]
fn perf_pipe_chain() {
    let mut fs = VirtualFs::new();

    // Create a file with many lines
    let mut content = String::new();
    for i in 0..10_000 {
        if i % 3 == 0 {
            content.push_str(&format!("ERROR: something failed at line {i}\n"));
        } else {
            content.push_str(&format!("INFO: normal operation at line {i}\n"));
        }
    }
    fs.touch("log.md", 0, 0).unwrap();
    fs.write_file("log.md", content.into_bytes()).unwrap();

    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let output = exec("cat log.md | grep ERROR | head -10 | wc -l", &mut fs);
        assert_eq!(output.trim(), "10");
    }
    let elapsed = start.elapsed();

    print_result(
        &format!("pipe: cat|grep|head|wc ({iterations}x, 10K lines)"),
        iterations,
        elapsed,
    );
    assert!(elapsed.as_secs() < 10, "too slow: {elapsed:?}");
}

// ─────────────────────────────────────────────
// Mixed workload (realistic simulation)
// ─────────────────────────────────────────────

#[test]
fn perf_mixed_workload() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    let start = Instant::now();
    let mut ops = 0;

    // Phase 1: Project setup (create dirs + files)
    fs.mkdir("src", 0, 0).unwrap();
    fs.mkdir("docs", 0, 0).unwrap();
    fs.mkdir("tests", 0, 0).unwrap();
    ops += 3;

    for i in 0..50 {
        let path = format!("src/module_{i:02}.md");
        fs.touch(&path, 0, 0).unwrap();
        fs.write_file(
            &path,
            format!("# Module {i}\n\n## API\n\nSome documentation.\n")
                .into_bytes(),
        )
        .unwrap();
        ops += 2;
    }

    vcs.commit(&fs, "initial project setup").unwrap();
    ops += 1;

    // Phase 2: Iterative development (edit + commit cycles)
    for cycle in 0..20 {
        // Edit a few files
        for f in 0..5 {
            let idx = (cycle * 5 + f) % 50;
            let path = format!("src/module_{idx:02}.md");
            fs.write_file(
                &path,
                format!("# Module {idx} (v{cycle})\n\n## Updated API\n\nNew content.\n")
                    .into_bytes(),
            )
            .unwrap();
            ops += 1;
        }

        // Create a test file
        let test_path = format!("tests/test_{cycle:02}.md");
        fs.touch(&test_path, 0, 0).unwrap();
        fs.write_file(
            &test_path,
            format!("# Test {cycle}\n\n- [ ] Test A\n- [ ] Test B\n")
                .into_bytes(),
        )
        .unwrap();
        ops += 2;

        // Commit
        vcs.commit(&fs, &format!("development cycle {cycle}")).unwrap();
        ops += 1;

        // Occasionally search
        if cycle % 5 == 0 {
            let _ = fs.grep("Updated", None, true).unwrap();
            let _ = fs.find(Some("."), Some("*.md")).unwrap();
            let _ = fs.ls(Some("src")).unwrap();
            ops += 3;
        }
    }

    // Phase 3: Revert to an earlier commit
    let commits = vcs.log();
    let mid_commit = &commits[commits.len() / 2];
    vcs.revert(&mut fs, &mid_commit.id.short_hex()).unwrap();
    ops += 1;

    let elapsed = start.elapsed();

    print_result(&format!("mixed workload ({ops} operations)"), ops, elapsed);
    println!("    commits: {}", vcs.log().len());
    println!("    objects in store: {}", vcs.store.object_count());
    assert!(elapsed.as_secs() < 10, "too slow: {elapsed:?}");
}
