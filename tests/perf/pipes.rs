use super::*;

#[test]
fn perf_pipe_chain() {
    let mut fs = VirtualFs::new();

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
    assert!(elapsed.as_secs() < debug_limit(10), "too slow: {elapsed:?}");
}

#[test]
fn perf_pipe_grep_wc() {
    let mut fs = VirtualFs::new();

    let mut content = String::new();
    for i in 0..50_000 {
        content.push_str(&format!("log entry {i}: status=ok\n"));
    }
    fs.touch("big-log.md", 0, 0).unwrap();
    fs.write_file("big-log.md", content.into_bytes()).unwrap();

    let iterations = 20;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = exec("cat big-log.md | grep status=ok | wc -l", &mut fs);
    }
    let elapsed = start.elapsed();

    print_result(
        &format!("pipe: cat|grep|wc ({iterations}x, 50K lines)"),
        iterations,
        elapsed,
    );
    assert!(elapsed.as_secs() < debug_limit(30), "too slow: {elapsed:?}");
}
