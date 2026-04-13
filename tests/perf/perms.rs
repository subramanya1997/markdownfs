use super::*;

#[test]
fn perf_permission_checked_ls() {
    let mut fs = VirtualFs::new();
    let mut root = Session::root();

    let pipeline = parser::parse_pipeline("adduser alice");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut root).unwrap();

    let alice_user = fs.registry.get_user(1).unwrap();
    let mut alice = Session::new(
        alice_user.uid,
        alice_user.groups[0],
        alice_user.groups.clone(),
        alice_user.name.clone(),
    );

    for i in 0..1000 {
        fs.touch(&format!("f_{i:04}.md"), 0, 0).unwrap();
    }

    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let pipeline = parser::parse_pipeline("ls");
        let _ = cmd::execute_pipeline(&pipeline, &mut fs, &mut alice).unwrap();
    }
    let elapsed = start.elapsed();

    print_result(
        &format!("ls with permission filtering (1K files, {iterations}x)"),
        iterations,
        elapsed,
    );
    assert!(elapsed.as_secs() < debug_limit(30), "too slow: {elapsed:?}");
}

#[test]
fn perf_permission_checked_find() {
    let mut fs = VirtualFs::new();
    let mut root = Session::root();

    let pipeline = parser::parse_pipeline("adduser alice");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut root).unwrap();

    let alice_user = fs.registry.get_user(1).unwrap();
    let alice = Session::new(
        alice_user.uid,
        alice_user.groups[0],
        alice_user.groups.clone(),
        alice_user.name.clone(),
    );

    for d in 0..10 {
        fs.mkdir(&format!("d_{d}"), 0, 0).unwrap();
        for f in 0..50 {
            fs.touch(&format!("d_{d}/f_{f:03}.md"), 0, 0).unwrap();
        }
    }

    let iterations = 50;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = fs.find(Some("."), Some("*.md"), Some(&alice)).unwrap();
    }
    let elapsed = start.elapsed();

    print_result(
        &format!("find with permission filtering (500 files, {iterations}x)"),
        iterations,
        elapsed,
    );
    assert!(elapsed.as_secs() < debug_limit(30), "too slow: {elapsed:?}");
}
