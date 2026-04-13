use super::*;

// ════════════════════════════════════════════════════════════════
//  CHMOD / CHOWN RESTRICTIONS
// ════════════════════════════════════════════════════════════════

#[test]
fn only_owner_can_chmod() {
    let (mut fs, _, mut alice, mut bob, ..) = setup();
    run("chmod 644 engineering/design.md", &mut fs, &mut alice);

    let result = try_run("chmod 777 engineering/design.md", &mut fs, &mut bob);
    assert!(result.is_err(), "bob cannot chmod alice's file");
}

#[test]
fn only_root_can_chown_uid() {
    let (mut fs, _, mut alice, ..) = setup();
    let result = try_run(
        "chown bob: engineering/design.md",
        &mut fs,
        &mut alice,
    );
    assert!(result.is_err(), "alice cannot chown uid to bob");
}

#[test]
fn root_can_chown() {
    let (mut fs, mut root, ..) = setup();
    run("chown bob:engineering engineering/design.md", &mut fs, &mut root);
    let stat = run("stat engineering/design.md", &mut fs, &mut root);
    assert!(stat.contains("Uid: 2"), "uid should be bob's (2). stat: {stat}");
}

#[test]
fn carol_cannot_chmod_engineering_files() {
    let (mut fs, _, _, _, mut carol, _) = setup();
    let result = try_run("chmod 777 engineering/design.md", &mut fs, &mut carol);
    assert!(result.is_err(), "carol cannot chmod engineering files");
}

// ════════════════════════════════════════════════════════════════
//  GROUP-BASED ACCESS
// ════════════════════════════════════════════════════════════════

#[test]
fn group_read_access() {
    let (mut fs, mut root, _, mut bob, ..) = setup();
    // Create a file readable only by engineering group
    run("touch engineering/secret-eng.md", &mut fs, &mut root);
    run("write engineering/secret-eng.md Engineering secrets", &mut fs, &mut root);
    run("chown root:engineering engineering/secret-eng.md", &mut fs, &mut root);
    run("chmod 640 engineering/secret-eng.md", &mut fs, &mut root);

    // bob is in engineering group — should be able to read
    let content = run("cat engineering/secret-eng.md", &mut fs, &mut bob);
    assert!(content.contains("Engineering secrets"));
}

#[test]
fn non_group_cannot_read_group_file() {
    let (mut fs, mut root, _, _, mut carol, _) = setup();
    run("touch engineering/eng-only.md", &mut fs, &mut root);
    run("write engineering/eng-only.md For engineers only", &mut fs, &mut root);
    run("chown root:engineering engineering/eng-only.md", &mut fs, &mut root);
    run("chmod 640 engineering/eng-only.md", &mut fs, &mut root);

    // carol is NOT in engineering group — cannot read (other bits are 0)
    let result = try_run("cat engineering/eng-only.md", &mut fs, &mut carol);
    assert!(result.is_err(), "carol not in engineering, should not read 640 file");
}

// ════════════════════════════════════════════════════════════════
//  USER MANAGEMENT EDGE CASES
// ════════════════════════════════════════════════════════════════

#[test]
fn cannot_create_duplicate_user() {
    let (mut fs, mut root, ..) = setup();
    let result = try_run("adduser alice", &mut fs, &mut root);
    assert!(result.is_err(), "duplicate user should fail");
}

#[test]
fn cannot_create_duplicate_group() {
    let (mut fs, mut root, ..) = setup();
    let result = try_run("addgroup engineering", &mut fs, &mut root);
    assert!(result.is_err(), "duplicate group should fail");
}

#[test]
fn cannot_delete_root_user() {
    let (mut fs, mut root, ..) = setup();
    let result = try_run("deluser root", &mut fs, &mut root);
    assert!(result.is_err(), "cannot delete root user");
}

#[test]
fn whoami_reflects_su() {
    let (mut fs, mut root, ..) = setup();
    let output = run("whoami", &mut fs, &mut root);
    assert_eq!(output.trim(), "root");

    run("su alice", &mut fs, &mut root);
    let output = run("whoami", &mut fs, &mut root);
    assert_eq!(output.trim(), "alice");
}

#[test]
fn id_shows_groups() {
    let (mut fs, mut root, ..) = setup();
    let output = run("id alice", &mut fs, &mut root);
    assert!(output.contains("uid=1(alice)"));
    assert!(output.contains("engineering") || output.contains("wheel"));
}

#[test]
fn groups_shows_all_memberships() {
    let (mut fs, mut root, ..) = setup();
    let output = run("groups alice", &mut fs, &mut root);
    assert!(output.contains("alice"));
}
