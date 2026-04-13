use super::*;

// ════════════════════════════════════════════════════════════════
//  SETGID INHERITANCE
// ════════════════════════════════════════════════════════════════

#[test]
fn setgid_inherits_group() {
    let (mut fs, _, mut alice, ..) = setup();
    run("touch engineering/inherited.md", &mut fs, &mut alice);
    let stat = run("stat engineering/inherited.md", &mut fs, &mut alice);

    let eng_gid = fs.registry.lookup_gid("engineering").unwrap();
    assert!(
        stat.contains(&format!("Gid: {eng_gid}")),
        "file should inherit engineering group via setgid. stat: {stat}"
    );
}

#[test]
fn setgid_inherits_on_mkdir() {
    let (mut fs, _, mut alice, ..) = setup();
    run("mkdir engineering/subdir", &mut fs, &mut alice);
    let stat = run("stat engineering/subdir", &mut fs, &mut alice);

    let eng_gid = fs.registry.lookup_gid("engineering").unwrap();
    assert!(
        stat.contains(&format!("Gid: {eng_gid}")),
        "subdir should inherit engineering group via setgid. stat: {stat}"
    );
}

#[test]
fn setgid_file_inherits_from_finance() {
    let (mut fs, _, _, _, mut carol, _) = setup();
    run("touch finance/report.md", &mut fs, &mut carol);
    let stat = run("stat finance/report.md", &mut fs, &mut carol);

    let fin_gid = fs.registry.lookup_gid("finance").unwrap();
    assert!(
        stat.contains(&format!("Gid: {fin_gid}")),
        "file in finance/ should inherit finance group. stat: {stat}"
    );
}

// ════════════════════════════════════════════════════════════════
//  STICKY BIT — /tmp
// ════════════════════════════════════════════════════════════════

#[test]
fn sticky_bit_prevents_deletion_by_others() {
    let (mut fs, _, mut alice, mut bob, ..) = setup();
    let result = try_run("rm tmp/alice-tmp.md", &mut fs, &mut bob);
    assert!(
        result.is_err(),
        "bob should not delete alice's file in sticky dir"
    );

    run("rm tmp/alice-tmp.md", &mut fs, &mut alice);
    let ls = run("ls tmp", &mut fs, &mut alice);
    assert!(!ls.contains("alice-tmp.md"), "alice deleted her own file");
}

#[test]
fn sticky_bit_owner_can_delete_own() {
    let (mut fs, _, _, mut bob, ..) = setup();
    run("rm tmp/bob-tmp.md", &mut fs, &mut bob);
    let ls = run("ls tmp", &mut fs, &mut bob);
    assert!(!ls.contains("bob-tmp.md"));
}

#[test]
fn sticky_bit_root_can_delete_any() {
    let (mut fs, mut root, ..) = setup();
    run("rm tmp/alice-tmp.md", &mut fs, &mut root);
    let ls = run("ls tmp", &mut fs, &mut root);
    assert!(!ls.contains("alice-tmp.md"));
}

#[test]
fn sticky_bit_carol_cannot_delete_bob_tmp() {
    let (mut fs, _, _, _, mut carol, _) = setup();
    let result = try_run("rm tmp/bob-tmp.md", &mut fs, &mut carol);
    assert!(
        result.is_err(),
        "carol should not delete bob's file in sticky dir"
    );
}

// ════════════════════════════════════════════════════════════════
//  MODE 000 — TOTAL LOCKOUT
// ════════════════════════════════════════════════════════════════

#[test]
fn mode_000_blocks_everyone_except_root() {
    let (mut fs, mut root, mut alice, ..) = setup();
    run("touch public/locked.md", &mut fs, &mut root);
    run("write public/locked.md secret data", &mut fs, &mut root);
    run("chmod 000 public/locked.md", &mut fs, &mut root);

    // Root can still read (bypasses all checks)
    let content = run("cat public/locked.md", &mut fs, &mut root);
    assert!(content.contains("secret data"));

    // Alice cannot
    let result = try_run("cat public/locked.md", &mut fs, &mut alice);
    assert!(result.is_err(), "mode 000 should block alice");
}

#[test]
fn mode_000_dir_blocks_ls() {
    let (mut fs, mut root, mut alice, ..) = setup();
    run("mkdir locked-dir", &mut fs, &mut root);
    run("chmod 000 locked-dir", &mut fs, &mut root);

    let result = try_run("ls locked-dir", &mut fs, &mut alice);
    assert!(result.is_err(), "mode 000 dir should block alice's ls");

    // Root can still list it
    let _ = run("ls locked-dir", &mut fs, &mut root);
}

// ════════════════════════════════════════════════════════════════
//  EXECUTE PERMISSION ON DIRECTORIES
// ════════════════════════════════════════════════════════════════

#[test]
fn no_execute_on_dir_blocks_traversal() {
    let (mut fs, mut root, mut alice, ..) = setup();
    run("mkdir blocked", &mut fs, &mut root);
    run("mkdir blocked/inner", &mut fs, &mut root);
    run("cd blocked/inner", &mut fs, &mut root);
    run("touch deep.md", &mut fs, &mut root);
    run("cd /", &mut fs, &mut root);

    // Remove execute from blocked/ for others
    run("chmod 744 blocked", &mut fs, &mut root);

    // alice (other) has r-- on blocked/ → can see but not traverse
    let result = try_run("cat blocked/inner/deep.md", &mut fs, &mut alice);
    assert!(result.is_err(), "no execute on dir should block traversal");
}

#[test]
fn no_execute_on_dir_blocks_cd() {
    let (mut fs, mut root, mut alice, ..) = setup();
    run("mkdir noexec", &mut fs, &mut root);
    run("chmod 644 noexec", &mut fs, &mut root); // r-- for others, no x

    let result = try_run("cd noexec", &mut fs, &mut alice);
    assert!(result.is_err(), "no execute on dir should block cd");
}
