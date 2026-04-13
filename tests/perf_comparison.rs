//! Comprehensive performance comparison: markdownfs vs native filesystem
//!
//! Run with: cargo test --release --test perf_comparison -- --nocapture
//!
//! Measures markdownfs operations and compares against native fs operations
//! performed on the same machine for a fair baseline.

use markdownfs::auth::session::Session;
use markdownfs::cmd;
use markdownfs::cmd::parser;
use markdownfs::fs::VirtualFs;
use markdownfs::persist::PersistManager;
use markdownfs::vcs::Vcs;
use std::time::Instant;

struct BenchResult {
    name: String,
    markdownfs_us: f64,
    native_us: Option<f64>,
    speedup: Option<f64>,
}

impl BenchResult {
    fn print(&self) {
        let markdownfs_str = format_duration_us(self.markdownfs_us);
        match (self.native_us, self.speedup) {
            (Some(native), Some(speedup)) => {
                let native_str = format_duration_us(native);
                let arrow = if speedup >= 1.0 {
                    "\x1b[32m↑\x1b[0m"
                } else {
                    "\x1b[31m↓\x1b[0m"
                };
                println!(
                    "  {:<50} markdownfs: {:>12}   native: {:>12}   {arrow} {:.1}x",
                    self.name, markdownfs_str, native_str, speedup
                );
            }
            _ => {
                println!(
                    "  {:<50} markdownfs: {:>12}",
                    self.name, markdownfs_str
                );
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

fn exec(line: &str, fs: &mut VirtualFs) -> String {
    let pipeline = parser::parse_pipeline(line);
    let mut session = Session::root();
    cmd::execute_pipeline(&pipeline, fs, &mut session).unwrap()
}

// ═══════════════════════════════════════════════════════════════
//  NATIVE FILESYSTEM BENCHMARK HELPERS
// ═══════════════════════════════════════════════════════════════

fn make_tmp(suffix: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "markdownfs_{suffix}_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn cleanup(p: &std::path::Path) {
    let _ = std::fs::remove_dir_all(p);
}

fn native_bench_file_creation(count: usize) -> f64 {
    let tmp = make_tmp("create");
    let start = Instant::now();
    for i in 0..count {
        std::fs::write(tmp.join(format!("file_{i:05}.md")), "").unwrap();
    }
    let us = start.elapsed().as_micros() as f64 / count as f64;
    cleanup(&tmp);
    us
}

fn native_bench_file_creation_nested(count: usize, dirs: usize) -> f64 {
    let tmp = make_tmp("create_nested");
    for d in 0..dirs {
        std::fs::create_dir(tmp.join(format!("dir_{d:03}"))).unwrap();
    }
    let start = Instant::now();
    for i in 0..count {
        let d = i % dirs;
        std::fs::write(
            tmp.join(format!("dir_{d:03}/file_{i:05}.md")),
            "",
        )
        .unwrap();
    }
    let us = start.elapsed().as_micros() as f64 / count as f64;
    cleanup(&tmp);
    us
}

fn native_bench_file_write(count: usize, content: &[u8]) -> f64 {
    let tmp = make_tmp("write");
    for i in 0..count {
        std::fs::write(tmp.join(format!("f_{i:05}.md")), "").unwrap();
    }
    let start = Instant::now();
    for i in 0..count {
        std::fs::write(tmp.join(format!("f_{i:05}.md")), content).unwrap();
    }
    let us = start.elapsed().as_micros() as f64 / count as f64;
    cleanup(&tmp);
    us
}

fn native_bench_file_read(count: usize, content: &[u8]) -> f64 {
    let tmp = make_tmp("read");
    for i in 0..count {
        std::fs::write(tmp.join(format!("f_{i:05}.md")), content).unwrap();
    }
    let start = Instant::now();
    for i in 0..count {
        let data = std::fs::read(tmp.join(format!("f_{i:05}.md"))).unwrap();
        assert_eq!(data.len(), content.len());
    }
    let us = start.elapsed().as_micros() as f64 / count as f64;
    cleanup(&tmp);
    us
}

fn native_bench_file_read_same(content: &[u8], count: usize) -> f64 {
    let tmp = make_tmp("read_same");
    let path = tmp.join("hot.md");
    std::fs::write(&path, content).unwrap();
    let start = Instant::now();
    for _ in 0..count {
        let _ = std::fs::read(&path).unwrap();
    }
    let us = start.elapsed().as_micros() as f64 / count as f64;
    cleanup(&tmp);
    us
}

fn native_bench_ls(count: usize, iterations: usize) -> f64 {
    let tmp = make_tmp("ls");
    for i in 0..count {
        std::fs::write(tmp.join(format!("f_{i:05}.md")), "").unwrap();
    }
    let start = Instant::now();
    for _ in 0..iterations {
        let entries: Vec<_> = std::fs::read_dir(&tmp).unwrap().collect();
        assert_eq!(entries.len(), count);
    }
    let us = start.elapsed().as_micros() as f64 / iterations as f64;
    cleanup(&tmp);
    us
}

fn native_bench_stat(count: usize, iterations: usize) -> f64 {
    let tmp = make_tmp("stat");
    for i in 0..count {
        std::fs::write(tmp.join(format!("f_{i:05}.md")), "content").unwrap();
    }
    let start = Instant::now();
    for _ in 0..iterations {
        for i in 0..count {
            let _ = std::fs::metadata(tmp.join(format!("f_{i:05}.md"))).unwrap();
        }
    }
    let total = count * iterations;
    let us = start.elapsed().as_micros() as f64 / total as f64;
    cleanup(&tmp);
    us
}

fn native_bench_grep(file_count: usize, iterations: usize) -> f64 {
    let tmp = make_tmp("grep");
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
    let us = start.elapsed().as_micros() as f64 / iterations as f64;
    cleanup(&tmp);
    us
}

fn native_bench_find(dirs: usize, files_per_dir: usize, iterations: usize) -> f64 {
    let tmp = make_tmp("find");
    for d in 0..dirs {
        let dir = tmp.join(format!("dir_{d}"));
        std::fs::create_dir(&dir).unwrap();
        for f in 0..files_per_dir {
            std::fs::write(dir.join(format!("file_{f:03}.md")), "").unwrap();
        }
    }
    let start = Instant::now();
    for _ in 0..iterations {
        let mut count = 0;
        fn walk(dir: &std::path::Path, count: &mut usize) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, count);
                } else if path.extension().map_or(false, |e| e == "md") {
                    *count += 1;
                }
            }
        }
        walk(&tmp, &mut count);
        assert_eq!(count, dirs * files_per_dir);
    }
    let us = start.elapsed().as_micros() as f64 / iterations as f64;
    cleanup(&tmp);
    us
}

fn native_bench_rm(count: usize) -> f64 {
    let tmp = make_tmp("rm");
    for i in 0..count {
        std::fs::write(tmp.join(format!("f_{i:05}.md")), "content").unwrap();
    }
    let start = Instant::now();
    for i in 0..count {
        std::fs::remove_file(tmp.join(format!("f_{i:05}.md"))).unwrap();
    }
    let us = start.elapsed().as_micros() as f64 / count as f64;
    cleanup(&tmp);
    us
}

fn native_bench_mv(count: usize) -> f64 {
    let tmp = make_tmp("mv");
    let src = tmp.join("src");
    let dst = tmp.join("dst");
    std::fs::create_dir(&src).unwrap();
    std::fs::create_dir(&dst).unwrap();
    for i in 0..count {
        std::fs::write(src.join(format!("f_{i:04}.md")), "").unwrap();
    }
    let start = Instant::now();
    for i in 0..count {
        std::fs::rename(
            src.join(format!("f_{i:04}.md")),
            dst.join(format!("f_{i:04}.md")),
        )
        .unwrap();
    }
    let us = start.elapsed().as_micros() as f64 / count as f64;
    cleanup(&tmp);
    us
}

fn native_bench_cp(count: usize, content: &[u8]) -> f64 {
    let tmp = make_tmp("cp");
    for i in 0..count {
        std::fs::write(tmp.join(format!("orig_{i:04}.md")), content).unwrap();
    }
    let copies = tmp.join("copies");
    std::fs::create_dir(&copies).unwrap();
    let start = Instant::now();
    for i in 0..count {
        std::fs::copy(
            tmp.join(format!("orig_{i:04}.md")),
            copies.join(format!("copy_{i:04}.md")),
        )
        .unwrap();
    }
    let us = start.elapsed().as_micros() as f64 / count as f64;
    cleanup(&tmp);
    us
}

fn native_bench_mkdir_p(count: usize, depth: usize) -> f64 {
    let tmp = make_tmp("mkdir_p");
    let start = Instant::now();
    for i in 0..count {
        let mut path = tmp.join(format!("root_{i}"));
        for d in 0..depth {
            path = path.join(format!("d{d}"));
        }
        std::fs::create_dir_all(&path).unwrap();
    }
    let us = start.elapsed().as_micros() as f64 / count as f64;
    cleanup(&tmp);
    us
}

fn native_bench_save_load(file_count: usize) -> (f64, f64, u64) {
    let tmp = make_tmp("persist");
    let content = "# Perf\n\nBenchmark file.\n";
    let start = Instant::now();
    for i in 0..file_count {
        std::fs::write(tmp.join(format!("p_{i:05}.md")), content).unwrap();
    }
    let save_us = start.elapsed().as_micros() as f64;
    let total_size: u64 = std::fs::read_dir(&tmp)
        .unwrap()
        .map(|e| e.unwrap().metadata().unwrap().len())
        .sum();
    let start = Instant::now();
    for i in 0..file_count {
        let _ = std::fs::read(tmp.join(format!("p_{i:05}.md"))).unwrap();
    }
    let load_us = start.elapsed().as_micros() as f64;
    cleanup(&tmp);
    (save_us, load_us, total_size)
}

fn native_bench_overwrite_same(count: usize) -> f64 {
    let tmp = make_tmp("overwrite");
    let path = tmp.join("target.md");
    std::fs::write(&path, "").unwrap();
    let start = Instant::now();
    for i in 0..count {
        std::fs::write(&path, format!("version {i}\n")).unwrap();
    }
    let us = start.elapsed().as_micros() as f64 / count as f64;
    cleanup(&tmp);
    us
}

// ═══════════════════════════════════════════════════════════════
//  MAIN COMPARISON BENCHMARK
// ═══════════════════════════════════════════════════════════════

#[test]
fn perf_full_comparison() {
    println!("\n{}", "═".repeat(110));
    println!(
        "  markdownfs Performance Comparison — markdownfs (in-memory VFS) vs Native Filesystem"
    );
    println!("  All times are per-operation unless noted. Speedup >1x means markdownfs is faster.");
    println!("{}\n", "═".repeat(110));

    let mut results: Vec<BenchResult> = Vec::new();

    // ── File creation (flat) ──
    println!("  \x1b[1m━━━ File Creation ━━━\x1b[0m");
    {
        let count = 10_000;
        let mut fs = VirtualFs::new();
        let start = Instant::now();
        for i in 0..count {
            fs.touch(&format!("file_{i:05}.md"), 0, 0).unwrap();
        }
        let markdownfs_us = start.elapsed().as_micros() as f64 / count as f64;
        let native_us = native_bench_file_creation(count);
        let r = BenchResult {
            name: format!("touch ({count} files, flat)"),
            markdownfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / markdownfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── File creation (nested) ──
    {
        let count = 10_000;
        let dirs = 100;
        let mut fs = VirtualFs::new();
        for d in 0..dirs {
            fs.mkdir(&format!("dir_{d:03}"), 0, 0).unwrap();
        }
        let start = Instant::now();
        for i in 0..count {
            let d = i % dirs;
            fs.touch(&format!("dir_{d:03}/file_{i:05}.md"), 0, 0).unwrap();
        }
        let markdownfs_us = start.elapsed().as_micros() as f64 / count as f64;
        let native_us = native_bench_file_creation_nested(count, dirs);
        let r = BenchResult {
            name: format!("touch ({count} files, {dirs} dirs)"),
            markdownfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / markdownfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── mkdir -p ──
    {
        let count = 100;
        let depth = 10;
        let mut fs = VirtualFs::new();
        let start = Instant::now();
        for i in 0..count {
            let mut path = format!("root_{i}");
            for d in 0..depth {
                path = format!("{path}/d{d}");
            }
            fs.mkdir_p(&path, 0, 0).unwrap();
        }
        let markdownfs_us = start.elapsed().as_micros() as f64 / count as f64;
        let native_us = native_bench_mkdir_p(count, depth);
        let r = BenchResult {
            name: format!("mkdir -p (depth={depth}, {count}x)"),
            markdownfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / markdownfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── File write (small) ──
    println!("\n  \x1b[1m━━━ File Write ━━━\x1b[0m");
    {
        let count = 10_000;
        let content = b"# Title\n\nSome markdown content.\n\n- Item 1\n- Item 2\n";
        let mut fs = VirtualFs::new();
        for i in 0..count {
            fs.touch(&format!("f_{i:05}.md"), 0, 0).unwrap();
        }
        let start = Instant::now();
        for i in 0..count {
            fs.write_file(&format!("f_{i:05}.md"), content.to_vec())
                .unwrap();
        }
        let markdownfs_us = start.elapsed().as_micros() as f64 / count as f64;
        let native_us = native_bench_file_write(count, content);
        let r = BenchResult {
            name: format!("write small ({count} files, {}B)", content.len()),
            markdownfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / markdownfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── File write (large) ──
    {
        let count = 1_000;
        let mut big_content = String::with_capacity(10_240);
        for i in 0..200 {
            big_content.push_str(&format!(
                "## Section {i}\n\nLorem ipsum dolor sit amet.\n\n"
            ));
        }
        let content = big_content.as_bytes();

        let mut fs = VirtualFs::new();
        for i in 0..count {
            fs.touch(&format!("b_{i:04}.md"), 0, 0).unwrap();
        }
        let start = Instant::now();
        for i in 0..count {
            fs.write_file(&format!("b_{i:04}.md"), content.to_vec())
                .unwrap();
        }
        let markdownfs_us = start.elapsed().as_micros() as f64 / count as f64;
        let native_us = native_bench_file_write(count, content);
        let r = BenchResult {
            name: format!(
                "write large ({count} files, {:.1}KB)",
                content.len() as f64 / 1024.0
            ),
            markdownfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / markdownfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── Overwrite same file ──
    {
        let count = 10_000;
        let mut fs = VirtualFs::new();
        fs.touch("target.md", 0, 0).unwrap();
        let start = Instant::now();
        for i in 0..count {
            fs.write_file("target.md", format!("v{i}\n").into_bytes())
                .unwrap();
        }
        let markdownfs_us = start.elapsed().as_micros() as f64 / count as f64;
        let native_us = native_bench_overwrite_same(count);
        let r = BenchResult {
            name: format!("overwrite same file ({count}x)"),
            markdownfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / markdownfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── File read (many files) ──
    println!("\n  \x1b[1m━━━ File Read ━━━\x1b[0m");
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
        let markdownfs_us = start.elapsed().as_micros() as f64 / count as f64;
        let native_us = native_bench_file_read(count, content);
        let r = BenchResult {
            name: format!("cat ({count} different files)"),
            markdownfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / markdownfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── File read (hot path — same file) ──
    {
        let count = 100_000;
        let content = b"# Hot path content\n\nFrequently read.\n";
        let mut fs = VirtualFs::new();
        fs.touch("hot.md", 0, 0).unwrap();
        fs.write_file("hot.md", content.to_vec()).unwrap();
        let start = Instant::now();
        for _ in 0..count {
            let _ = fs.cat("hot.md").unwrap();
        }
        let markdownfs_us = start.elapsed().as_micros() as f64 / count as f64;
        let native_us = native_bench_file_read_same(content, count);
        let r = BenchResult {
            name: format!("cat same file ({count}x, hot path)"),
            markdownfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / markdownfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── Directory listing ──
    println!("\n  \x1b[1m━━━ Directory Listing ━━━\x1b[0m");
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
        let markdownfs_us = start.elapsed().as_micros() as f64 / iterations as f64;
        let native_us = native_bench_ls(file_count, iterations);
        let r = BenchResult {
            name: format!("ls ({file_count} entries, {iterations}x)"),
            markdownfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / markdownfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── Stat ──
    {
        let count = 100;
        let iterations = 100;
        let mut fs = VirtualFs::new();
        for i in 0..count {
            fs.touch(&format!("f_{i:04}.md"), 0, 0).unwrap();
        }
        let start = Instant::now();
        for _ in 0..iterations {
            for i in 0..count {
                let _ = fs.stat(&format!("f_{i:04}.md")).unwrap();
            }
        }
        let total = count * iterations;
        let markdownfs_us = start.elapsed().as_micros() as f64 / total as f64;
        let native_us = native_bench_stat(count, iterations);
        let r = BenchResult {
            name: format!("stat ({count} files, {iterations}x each)"),
            markdownfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / markdownfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── Search: grep ──
    println!("\n  \x1b[1m━━━ Search ━━━\x1b[0m");
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
            let _ = fs.grep("TODO", None, true, None).unwrap();
        }
        let markdownfs_us = start.elapsed().as_micros() as f64 / iterations as f64;
        let native_us = native_bench_grep(file_count, iterations);
        let r = BenchResult {
            name: format!("grep -r ({file_count} files, {iterations}x)"),
            markdownfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / markdownfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── Search: find ──
    {
        let dirs = 10;
        let files_per_dir = 100;
        let iterations = 50;
        let mut fs = VirtualFs::new();
        for d in 0..dirs {
            fs.mkdir(&format!("dir_{d}"), 0, 0).unwrap();
            for f in 0..files_per_dir {
                fs.touch(&format!("dir_{d}/file_{f:03}.md"), 0, 0).unwrap();
            }
        }
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = fs.find(Some("."), Some("*.md"), None).unwrap();
        }
        let markdownfs_us = start.elapsed().as_micros() as f64 / iterations as f64;
        let native_us = native_bench_find(dirs, files_per_dir, iterations);
        let r = BenchResult {
            name: format!(
                "find -name *.md ({} files, {iterations}x)",
                dirs * files_per_dir
            ),
            markdownfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / markdownfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── Delete ──
    println!("\n  \x1b[1m━━━ Delete ━━━\x1b[0m");
    {
        let count = 5_000;
        let mut fs = VirtualFs::new();
        for i in 0..count {
            fs.touch(&format!("f_{i:05}.md"), 0, 0).unwrap();
        }
        let start = Instant::now();
        for i in 0..count {
            fs.rm(&format!("f_{i:05}.md")).unwrap();
        }
        let markdownfs_us = start.elapsed().as_micros() as f64 / count as f64;
        let native_us = native_bench_rm(count);
        let r = BenchResult {
            name: format!("rm ({count} files)"),
            markdownfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / markdownfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── Move ──
    println!("\n  \x1b[1m━━━ Move / Copy ━━━\x1b[0m");
    {
        let count = 1_000;
        let mut fs = VirtualFs::new();
        fs.mkdir("src", 0, 0).unwrap();
        fs.mkdir("dst", 0, 0).unwrap();
        for i in 0..count {
            fs.touch(&format!("src/f_{i:04}.md"), 0, 0).unwrap();
        }
        let start = Instant::now();
        for i in 0..count {
            fs.mv(&format!("src/f_{i:04}.md"), &format!("dst/f_{i:04}.md"))
                .unwrap();
        }
        let markdownfs_us = start.elapsed().as_micros() as f64 / count as f64;
        let native_us = native_bench_mv(count);
        let r = BenchResult {
            name: format!("mv ({count} renames)"),
            markdownfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / markdownfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── Copy ──
    {
        let count = 1_000;
        let content = b"# Content to copy\n\nSome data here.\n";
        let mut fs = VirtualFs::new();
        fs.mkdir("copies", 0, 0).unwrap();
        for i in 0..count {
            let path = format!("orig_{i:04}.md");
            fs.touch(&path, 0, 0).unwrap();
            fs.write_file(&path, content.to_vec()).unwrap();
        }
        let start = Instant::now();
        for i in 0..count {
            fs.cp(
                &format!("orig_{i:04}.md"),
                &format!("copies/copy_{i:04}.md"),
                0,
                0,
            )
            .unwrap();
        }
        let markdownfs_us = start.elapsed().as_micros() as f64 / count as f64;
        let native_us = native_bench_cp(count, content);
        let r = BenchResult {
            name: format!("cp ({count} copies, {}B)", content.len()),
            markdownfs_us,
            native_us: Some(native_us),
            speedup: Some(native_us / markdownfs_us),
        };
        r.print();
        results.push(r);
    }

    // ── Persistence ──
    println!("\n  \x1b[1m━━━ Persistence ━━━\x1b[0m");
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
        vcs.commit(&fs, "bulk", "root").unwrap();

        let tmp = make_tmp("cmp_persist");
        let persist = PersistManager::new(&tmp);

        let start = Instant::now();
        persist.save(&fs, &vcs).unwrap();
        let markdownfs_save_us = start.elapsed().as_micros() as f64;

        let markdownfs_size = std::fs::metadata(tmp.join(".vfs/state.bin"))
            .unwrap()
            .len();

        let start = Instant::now();
        let _ = persist.load().unwrap();
        let markdownfs_load_us = start.elapsed().as_micros() as f64;

        let (native_save_us, native_load_us, native_size) = native_bench_save_load(file_count);

        let r_save = BenchResult {
            name: format!("save ({file_count} files, single binary)"),
            markdownfs_us: markdownfs_save_us,
            native_us: Some(native_save_us),
            speedup: Some(native_save_us / markdownfs_save_us),
        };
        r_save.print();

        let r_load = BenchResult {
            name: format!("load ({file_count} files)"),
            markdownfs_us: markdownfs_load_us,
            native_us: Some(native_load_us),
            speedup: Some(native_load_us / markdownfs_load_us),
        };
        r_load.print();

        println!(
            "    markdownfs state.bin: {:.2} MB | native {file_count} files: {:.2} MB | compression: {:.1}x",
            markdownfs_size as f64 / (1024.0 * 1024.0),
            native_size as f64 / (1024.0 * 1024.0),
            native_size as f64 / markdownfs_size as f64,
        );

        results.push(r_save);
        results.push(r_load);

        cleanup(&tmp);
    }

    // ── VCS Operations (no native equivalent) ──
    println!("\n  \x1b[1m━━━ VCS Operations (no native equivalent) ━━━\x1b[0m");
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
        let _ = vcs.commit(&fs, "bench", "root").unwrap();
        let commit_us = start.elapsed().as_micros() as f64;

        let r = BenchResult {
            name: format!("commit ({file_count} files)"),
            markdownfs_us: commit_us,
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
        let id1 = vcs.commit(&fs, "v1", "root").unwrap();
        for i in 0..file_count {
            fs.write_file(&format!("f_{i:05}.md"), b"# Modified\n".to_vec())
                .unwrap();
        }
        vcs.commit(&fs, "v2", "root").unwrap();

        let start = Instant::now();
        vcs.revert(&mut fs, &id1.short_hex()).unwrap();
        let revert_us = start.elapsed().as_micros() as f64;

        let r = BenchResult {
            name: format!("revert ({file_count} files)"),
            markdownfs_us: revert_us,
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
            vcs.commit(&fs, &format!("c{c}"), "root").unwrap();
        }
        let elapsed = start.elapsed();
        let per_op = elapsed.as_micros() as f64 / 100.0;

        let r = BenchResult {
            name: "100 sequential commits (100 files)".to_string(),
            markdownfs_us: per_op,
            native_us: None,
            speedup: None,
        };
        r.print();
        results.push(r);
    }

    // ── Deduplication ──
    println!("\n  \x1b[1m━━━ Deduplication ━━━\x1b[0m");
    {
        let file_count = 10_000;
        let content = "# Shared Template\n\nThis content is identical everywhere.\n";
        let mut fs = VirtualFs::new();
        let mut vcs = Vcs::new();
        for i in 0..file_count {
            fs.touch(&format!("dup_{i:05}.md"), 0, 0).unwrap();
            fs.write_file(&format!("dup_{i:05}.md"), content.as_bytes().to_vec())
                .unwrap();
        }
        vcs.commit(&fs, "dedup", "root").unwrap();

        let object_count = vcs.store.object_count();
        println!(
            "  {:<50} {file_count} identical files → {object_count} objects (1 blob + 1 tree + 1 commit)",
            "content-addressable dedup"
        );
        assert!(object_count < 10);
    }

    // ── Pipe Performance ──
    println!("\n  \x1b[1m━━━ Pipe Processing ━━━\x1b[0m");
    {
        let mut fs = VirtualFs::new();
        let mut content = String::new();
        for i in 0..10_000 {
            if i % 3 == 0 {
                content.push_str(&format!("ERROR: failed at {i}\n"));
            } else {
                content.push_str(&format!("INFO: ok at {i}\n"));
            }
        }
        fs.touch("log.md", 0, 0).unwrap();
        fs.write_file("log.md", content.into_bytes()).unwrap();

        let iterations = 100;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = exec("cat log.md | grep ERROR | head -10 | wc -l", &mut fs);
        }
        let us = start.elapsed().as_micros() as f64 / iterations as f64;
        let r = BenchResult {
            name: format!("cat|grep|head|wc (10K lines, {iterations}x)"),
            markdownfs_us: us,
            native_us: None,
            speedup: None,
        };
        r.print();
        results.push(r);
    }

    // ═══════════════════════════════════════════════════════════════
    //  SUMMARY
    // ═══════════════════════════════════════════════════════════════

    println!("\n{}", "═".repeat(110));
    println!("  \x1b[1mSummary\x1b[0m");
    println!("{}", "═".repeat(110));

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
            .max_by(|a, b| {
                a.speedup
                    .unwrap()
                    .partial_cmp(&b.speedup.unwrap())
                    .unwrap()
            })
            .unwrap();
        let slowest_op = comparable
            .iter()
            .min_by(|a, b| {
                a.speedup
                    .unwrap()
                    .partial_cmp(&b.speedup.unwrap())
                    .unwrap()
            })
            .unwrap();

        println!(
            "  Benchmarks compared:               {}/{} operations",
            comparable.len(),
            results.len()
        );
        println!("  Average speedup vs native FS:      {avg_speedup:.1}x");
        println!(
            "  Fastest advantage:                 {:.1}x  ({})",
            max_speedup, fastest_op.name
        );
        println!(
            "  Smallest advantage:                {:.1}x  ({})",
            min_speedup, slowest_op.name
        );
        println!();
        println!("  \x1b[1mKey advantages:\x1b[0m");
        println!("    • In-memory: no syscall overhead, no disk I/O for operations");
        println!("    • Content-addressable: automatic dedup (10K identical files = 1 blob)");
        println!("    • Single-file persistence: save/load entire state atomically");
        println!("    • Built-in VCS: commit/revert in microseconds, not seconds");
        println!("    • Permission-aware: filtered views without separate access control layer");
        println!("    • Zero-copy reads: cat returns reference to in-memory content");
    }
    println!("{}\n", "═".repeat(110));
}
