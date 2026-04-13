use super::*;
use markdownfs::vcs::Vcs;

#[test]
fn perf_mixed_workload() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    let start = Instant::now();
    let mut ops = 0;

    // Phase 1: Project setup
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

    vcs.commit(&fs, "initial project setup", "root").unwrap();
    ops += 1;

    // Phase 2: Iterative development
    for cycle in 0..20 {
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

        let test_path = format!("tests/test_{cycle:02}.md");
        fs.touch(&test_path, 0, 0).unwrap();
        fs.write_file(
            &test_path,
            format!("# Test {cycle}\n\n- [ ] Test A\n- [ ] Test B\n")
                .into_bytes(),
        )
        .unwrap();
        ops += 2;

        vcs.commit(&fs, &format!("development cycle {cycle}"), "root").unwrap();
        ops += 1;

        if cycle % 5 == 0 {
            let _ = fs.grep("Updated", None, true, None).unwrap();
            let _ = fs.find(Some("."), Some("*.md"), None).unwrap();
            let _ = fs.ls(Some("src")).unwrap();
            ops += 3;
        }
    }

    // Phase 3: Revert
    let commits = vcs.log();
    let mid_commit = &commits[commits.len() / 2];
    vcs.revert(&mut fs, &mid_commit.id.short_hex()).unwrap();
    ops += 1;

    let elapsed = start.elapsed();

    print_result(&format!("mixed workload ({ops} operations)"), ops, elapsed);
    println!("    commits: {}", vcs.log().len());
    println!("    objects in store: {}", vcs.store.object_count());
    assert!(elapsed.as_secs() < debug_limit(10), "too slow: {elapsed:?}");
}

#[test]
fn perf_mixed_multiuser_workload() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();
    let mut root = Session::root();

    // Setup users
    let pipeline = parser::parse_pipeline("adduser alice");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut root).unwrap();
    let pipeline = parser::parse_pipeline("adduser bob");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut root).unwrap();

    let alice_user = fs.registry.get_user(1).unwrap();
    let mut alice = Session::new(
        alice_user.uid,
        alice_user.groups[0],
        alice_user.groups.clone(),
        alice_user.name.clone(),
    );

    let bob_user = fs.registry.get_user(2).unwrap();
    let mut bob = Session::new(
        bob_user.uid,
        bob_user.groups[0],
        bob_user.groups.clone(),
        bob_user.name.clone(),
    );

    // Create shared workspace
    let pipeline = parser::parse_pipeline("mkdir shared");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut root).unwrap();
    let pipeline = parser::parse_pipeline("chmod 777 shared");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut root).unwrap();

    let start = Instant::now();
    let mut ops = 0;

    // Alternating user operations
    for i in 0..100 {
        let session = if i % 2 == 0 { &mut alice } else { &mut bob };
        let username = if i % 2 == 0 { "alice" } else { "bob" };

        let path = format!("shared/{username}_{i:03}.md");
        let pipeline = parser::parse_pipeline(&format!("touch {path}"));
        cmd::execute_pipeline(&pipeline, &mut fs, session).unwrap();

        let pipeline = parser::parse_pipeline(&format!("write {path} Work by {username} #{i}"));
        cmd::execute_pipeline(&pipeline, &mut fs, session).unwrap();
        ops += 2;

        if i % 10 == 0 {
            vcs.commit(&fs, &format!("checkpoint {i}"), username).unwrap();
            ops += 1;
        }
    }

    let elapsed = start.elapsed();

    print_result(
        &format!("multi-user mixed workload ({ops} ops, 2 users)"),
        ops,
        elapsed,
    );
    assert!(elapsed.as_secs() < debug_limit(10), "too slow: {elapsed:?}");
}
