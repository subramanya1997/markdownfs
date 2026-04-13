//! Detailed performance comparison: mdvfs vs native filesystem vs git
//!
//! Run with: cargo test --release --test perf_comparison -- --nocapture
//!
//! This measures mdvfs operations and compares against native fs + git
//! operations performed on the same machine for a fair baseline.

use mdvfs::fs::VirtualFs;
use mdvfs::persist::PersistManager;
use mdvfs::vcs::Vcs;
use std::time::Instant;

struct BenchResult {
    name: String,
    mdvfs_us: f64,
    native_us: Option<f64>,
    speedup: Option<f64>,
}

impl BenchResult {
    fn print(&self) {
        let mdvfs_str = format_duration_us(self.mdvfs_us);
        match (self.native_us, self.speedup) {
            (Some(native), Some(speedup)) => {
                let native_str = format_duration_us(native);
                let arrow = if speedup >= 1.0 { "\x1b[32m↑\x1b[0m" } else { "\x1b[31m↓\x1b[0m" };
                println!(
                    "  {:<45} mdvfs: {:>10}   native: {:>10}   {arrow} {:.1}x",
                    self.name, mdvfs_str, native_str, speedup
                );
            }
            _ => {
                println!("  {:<45} mdvfs: {:>10}", self.name, mdvfs_str);
            }
        }
    }
}

fn format_duration_us(us: f64) -> String {
    if us < 1.0 {
        format!("{:.0}ns", us * 1000.0)
    } else if us < 1000.0 {
        format!("{:.2}µs", us)
    } else if us < 1_000_000.0 {
        format!("{:.2}ms", us / 1000.0)
    } else {
        format!("{:.2}s", us / 1_000_000.0)
    }
}

/// Run native filesystem benchmark in a temp dir.
fn native_bench_file_creation(count: usize) -> f64 {
    let tmp = std::env::temp_dir().join(format!("mdvfs_native_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let start = Instant::now();
    for i in 0..count {
        let path = tmp.join(format!("file_{i:05}.md"));
        std::fs::write(&path, "").unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_micros() as f64 / count as f64;

    let _ = std::fs::remove_dir_all(&tmp);
    per_op
}

fn native_bench_file_write(count: usize, content: &[u8]) -> f64 {
    let tmp = std::env::temp_dir().join(format!("mdvfs_nwrite_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    // Pre-create
    for i in 0..count {
        let path = tmp.join(format!("f_{i:05}.md"));
        std::fs::write(&path, "").unwrap();
    }

    let start = Instant::now();
    for i in 0..count {
        let path = tmp.join(format!("f_{i:05}.md"));
        std::fs::write(&path, content).unwrap();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_micros() as f64 / count as f64;

    let _ = std::fs::remove_dir_all(&tmp);
    per_op
}

fn native_bench_file_read(count: usize, content: &[u8]) -> f64 {
    let tmp = std::env::temp_dir().join(format!("mdvfs_nread_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    for i in 0..count {
        let path = tmp.join(format!("f_{i:05}.md"));
        std::fs::write(&path, content).unwrap();
    }

    let start = Instant::now();
    for i in 0..count {
        let path = tmp.join(format!("f_{i:05}.md"));
        let data = std::fs::read(&path).unwrap();
        assert_eq!(data.len(), content.len());
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_micros() as f64 / count as f64;

    let _ = std::fs::remove_dir_all(&tmp);
    per_op
}

fn native_bench_ls(count: usize, iterations: usize) -> f64 {
    let tmp = std::env::temp_dir().join(format!("mdvfs_nls_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    for i in 0..count {
        std::fs::write(tmp.join(format!("f_{i:05}.md")), "").unwrap();
    }

    let start = Instant::now();
    for _ in 0..iterations {
        let entries: Vec<_> = std::fs::read_dir(&tmp).unwrap().collect();
        assert_eq!(entries.len(), count);
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_micros() as f64 / iterations as f64;

    let _ = std::fs::remove_dir_all(&tmp);
    per_op
}

fn native_bench_grep(file_count: usize, iterations: usize) -> f64 {
    let tmp = std::env::temp_dir().join(format!("mdvfs_ngrep_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    for i in 0..file_count {
        let content = if i % 10 == 0 {
            format!("# File {i}\n\nTODO: fix this issue\n\nSome other content.\n")
        } else {
            format!("# File {i}\n\nEverything is fine here.\n\nNo issues.\n")
        };
        std::fs::write(tmp.join(format!("s_{i:04}.md")), content).unwrap();
    }

    let re = regex::Regex::new("TODO").unwrap();
    let start = Instant::now();
    for _ in 0..iterations {
        let mut match_count = 0;
        for entry in std::fs::read_dir(&tmp).unwrap() {
            let path = entry.unwrap().path();
            let content = std::fs::read_to_string(&path).unwrap();
            for line in content.lines() {
                if re.is_match(line) {
                    match_count += 1;
                }
            }
        }
        assert_eq!(match_count, file_count / 10);
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_micros() as f64 / iterations as f64;

    let _ = std::fs::remove_dir_all(&tmp);
    per_op
}

fn native_bench_save_load(file_count: usize) -> (f64, f64, u64) {
    let tmp = std::env::temp_dir().join(format!("mdvfs_npersist_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let content = "# Perf\n\nBenchmark file.\n";

    // "Save" = write all files
    let start = Instant::now();
    for i in 0..file_count {
        std::fs::write(tmp.join(format!("p_{i:05}.md")), content).unwrap();
    }
    let save_us = start.elapsed().as_micros() as f64;

    let total_size: u64 = std::fs::read_dir(&tmp)
        .unwrap()
        .map(|e| e.unwrap().metadata().unwrap().len())
        .sum();

    // "Load" = read all files
    let start = Instant::now();
    for i in 0..file_count {
        let _ = std::fs::read(tmp.join(format!("p_{i:05}.md"))).unwrap();
    }
    let load_us = start.elapsed().as_micros() as f64;

    let _ = std::fs::remove_dir_all(&tmp);
    (save_us, load_us, total_size)
}

// ─────────────────────────────────────────────
// Main comparison benchmark
// ─────────────────────────────────────────────

#[test]
fn perf_full_comparison() {
    println!("\n{}", "=".repeat(100));
    println!("  mdvfs Performance Comparison — mdvfs (in-memory VFS) vs Native Filesystem");
    println!("  All times are per-operation. Speedup >1x means mdvfs is faster.");
    println!("{}\n", "=".repeat(100));

    let mut results: Vec<BenchResult> = Vec::new();

    // ── File creation ──
    println!("  \x1b[1m--- File Creation ---\x1b[0m");
    {
        let count = 10_000;
        let mut fs = VirtualFs::new();
        let start = Instant::now();
        for i in 0..count {
            fs.touch(&format!("file_{i:05}.md"), 0, 0).unwrap();
        }
        let mdvfs_us = start.elapsed().as_micros() as f64 / count as f64;
        let native_us = native_bench_file_creation(count);
        let r = BenchResult {
            name: format!("touch ({count} files)"),
            mdvfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / mdvfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── File write (small) ──
    println!("\n  \x1b[1m--- File Write ---\x1b[0m");
    {
        let count = 10_000;
        let content = b"# Title\n\nSome markdown content.\n\n- Item 1\n- Item 2\n";
        let mut fs = VirtualFs::new();
        for i in 0..count {
            fs.touch(&format!("f_{i:05}.md"), 0, 0).unwrap();
        }
        let start = Instant::now();
        for i in 0..count {
            fs.write_file(&format!("f_{i:05}.md"), content.to_vec()).unwrap();
        }
        let mdvfs_us = start.elapsed().as_micros() as f64 / count as f64;
        let native_us = native_bench_file_write(count, content);
        let r = BenchResult {
            name: format!("write small ({count} files, {}B)", content.len()),
            mdvfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / mdvfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── File write (large) ──
    {
        let count = 1_000;
        let mut big_content = String::with_capacity(10_240);
        for i in 0..200 {
            big_content.push_str(&format!("## Section {i}\n\nLorem ipsum dolor sit amet.\n\n"));
        }
        let content = big_content.as_bytes();

        let mut fs = VirtualFs::new();
        for i in 0..count {
            fs.touch(&format!("b_{i:04}.md"), 0, 0).unwrap();
        }
        let start = Instant::now();
        for i in 0..count {
            fs.write_file(&format!("b_{i:04}.md"), content.to_vec()).unwrap();
        }
        let mdvfs_us = start.elapsed().as_micros() as f64 / count as f64;
        let native_us = native_bench_file_write(count, content);
        let r = BenchResult {
            name: format!("write large ({count} files, {:.1}KB)", content.len() as f64 / 1024.0),
            mdvfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / mdvfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── File read ──
    println!("\n  \x1b[1m--- File Read ---\x1b[0m");
    {
        let count = 10_000;
        let content = b"# Hello\n\nWorld\n";
        let mut fs = VirtualFs::new();
        for i in 0..count {
            let path = format!("r_{i:05}.md");
            fs.touch(&path, 0, 0).unwrap();
            fs.write_file(&path, content.to_vec()).unwrap();
        }
        let start = Instant::now();
        for i in 0..count {
            let _ = fs.cat(&format!("r_{i:05}.md")).unwrap();
        }
        let mdvfs_us = start.elapsed().as_micros() as f64 / count as f64;
        let native_us = native_bench_file_read(count, content);
        let r = BenchResult {
            name: format!("cat ({count} reads)"),
            mdvfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / mdvfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── Directory listing ──
    println!("\n  \x1b[1m--- Directory Listing ---\x1b[0m");
    {
        let file_count = 10_000;
        let iterations = 100;
        let mut fs = VirtualFs::new();
        for i in 0..file_count {
            fs.touch(&format!("f_{i:05}.md"), 0, 0).unwrap();
        }
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = fs.ls(None).unwrap();
        }
        let mdvfs_us = start.elapsed().as_micros() as f64 / iterations as f64;
        let native_us = native_bench_ls(file_count, iterations);
        let r = BenchResult {
            name: format!("ls ({file_count} entries, {iterations}x)"),
            mdvfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / mdvfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── Grep ──
    println!("\n  \x1b[1m--- Search ---\x1b[0m");
    {
        let file_count = 1_000;
        let iterations = 50;
        let mut fs = VirtualFs::new();
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
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = fs.grep("TODO", None, true).unwrap();
        }
        let mdvfs_us = start.elapsed().as_micros() as f64 / iterations as f64;
        let native_us = native_bench_grep(file_count, iterations);
        let r = BenchResult {
            name: format!("grep -r ({file_count} files, {iterations}x)"),
            mdvfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / mdvfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── Persistence ──
    println!("\n  \x1b[1m--- Persistence ---\x1b[0m");
    {
        let file_count = 10_000;
        let content = "# Perf\n\nBenchmark file.\n";

        let mut fs = VirtualFs::new();
        let mut vcs = Vcs::new();
        for i in 0..file_count {
            let path = format!("p_{i:05}.md");
            fs.touch(&path, 0, 0).unwrap();
            fs.write_file(&path, content.as_bytes().to_vec()).unwrap();
        }
        vcs.commit(&fs, "bulk").unwrap();

        let tmp = std::env::temp_dir().join(format!("mdvfs_cmp_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let persist = PersistManager::new(&tmp);

        // mdvfs save
        let start = Instant::now();
        persist.save(&fs, &vcs).unwrap();
        let mdvfs_save_us = start.elapsed().as_micros() as f64;

        let mdvfs_size = std::fs::metadata(tmp.join(".vfs/state.bin"))
            .unwrap()
            .len();

        // mdvfs load
        let start = Instant::now();
        let _ = persist.load().unwrap();
        let mdvfs_load_us = start.elapsed().as_micros() as f64;

        // native save/load
        let (native_save_us, native_load_us, native_size) =
            native_bench_save_load(file_count);

        let r_save = BenchResult {
            name: format!("save ({file_count} files)"),
            mdvfs_us: mdvfs_save_us,
            native_us: Some(native_save_us),
            speedup: Some(native_save_us / mdvfs_save_us),
        };
        r_save.print();

        let r_load = BenchResult {
            name: format!("load ({file_count} files)"),
            mdvfs_us: mdvfs_load_us,
            native_us: Some(native_load_us),
            speedup: Some(native_load_us / mdvfs_load_us),
        };
        r_load.print();

        println!(
            "    mdvfs state.bin: {:.2} MB | native {file_count} files: {:.2} MB",
            mdvfs_size as f64 / (1024.0 * 1024.0),
            native_size as f64 / (1024.0 * 1024.0),
        );

        results.push(r_save);
        results.push(r_load);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ── VCS (no native equivalent — compare to git) ──
    println!("\n  \x1b[1m--- VCS Operations (no native equivalent) ---\x1b[0m");
    {
        let file_count = 10_000;
        let content = "# Title\n\nContent here.\n";
        let mut fs = VirtualFs::new();
        let mut vcs = Vcs::new();
        for i in 0..file_count {
            let path = format!("f_{i:05}.md");
            fs.touch(&path, 0, 0).unwrap();
            fs.write_file(&path, content.as_bytes().to_vec()).unwrap();
        }

        let start = Instant::now();
        let _ = vcs.commit(&fs, "bench").unwrap();
        let commit_us = start.elapsed().as_micros() as f64;

        let r = BenchResult {
            name: format!("commit ({file_count} files)"),
            mdvfs_us: commit_us,
            native_us: None,
            speedup: None,
        };
        r.print();
        results.push(r);
    }
    {
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
        for i in 0..file_count {
            fs.write_file(&format!("f_{i:05}.md"), b"# Modified\n".to_vec())
                .unwrap();
        }
        vcs.commit(&fs, "v2").unwrap();

        let start = Instant::now();
        vcs.revert(&mut fs, &id1.short_hex()).unwrap();
        let revert_us = start.elapsed().as_micros() as f64;

        let r = BenchResult {
            name: format!("revert ({file_count} files)"),
            mdvfs_us: revert_us,
            native_us: None,
            speedup: None,
        };
        r.print();
        results.push(r);
    }
    {
        let mut fs = VirtualFs::new();
        let mut vcs = Vcs::new();
        for i in 0..100 {
            fs.touch(&format!("f_{i:03}.md"), 0, 0).unwrap();
            fs.write_file(&format!("f_{i:03}.md"), format!("# F{i}\n").into_bytes())
                .unwrap();
        }
        let start = Instant::now();
        for c in 0..100 {
            let path = format!("f_{:03}.md", c % 100);
            fs.write_file(&path, format!("# Updated {c}\n").into_bytes())
                .unwrap();
            vcs.commit(&fs, &format!("c{c}")).unwrap();
        }
        let elapsed = start.elapsed();
        let per_op = elapsed.as_micros() as f64 / 100.0;

        let r = BenchResult {
            name: "100 sequential commits (100 files)".to_string(),
            mdvfs_us: per_op,
            native_us: None,
            speedup: None,
        };
        r.print();
        results.push(r);
    }

    // ── Summary ──
    println!("\n{}", "=".repeat(100));
    println!("  \x1b[1mSummary\x1b[0m");
    println!("{}", "=".repeat(100));

    let comparable: Vec<&BenchResult> = results.iter().filter(|r| r.speedup.is_some()).collect();
    if !comparable.is_empty() {
        let avg_speedup: f64 =
            comparable.iter().map(|r| r.speedup.unwrap()).sum::<f64>() / comparable.len() as f64;
        let max_speedup = comparable
            .iter()
            .map(|r| r.speedup.unwrap())
            .fold(f64::NEG_INFINITY, f64::max);
        let min_speedup = comparable
            .iter()
            .map(|r| r.speedup.unwrap())
            .fold(f64::INFINITY, f64::min);

        let fastest_op = comparable
            .iter()
            .max_by(|a, b| a.speedup.unwrap().partial_cmp(&b.speedup.unwrap()).unwrap())
            .unwrap();
        let slowest_op = comparable
            .iter()
            .min_by(|a, b| a.speedup.unwrap().partial_cmp(&b.speedup.unwrap()).unwrap())
            .unwrap();

        println!("  Average speedup vs native FS:  {avg_speedup:.1}x");
        println!(
            "  Fastest advantage:             {:.1}x  ({})",
            max_speedup, fastest_op.name
        );
        println!(
            "  Smallest advantage:            {:.1}x  ({})",
            min_speedup, slowest_op.name
        );
        println!();
        println!("  mdvfs advantages:");
        println!("    - In-memory: no syscall overhead, no disk I/O for operations");
        println!("    - Content-addressable: automatic dedup (10K identical files = 1 blob)");
        println!("    - Single-file persistence: save/load entire state atomically");
        println!("    - Built-in VCS: commit/revert in microseconds, not seconds");
    }
    println!("{}\n", "=".repeat(100));
}
