use super::*;

#[test]
fn bob_sees_engineering_not_finance() {
    let (mut fs, _, _, mut bob, ..) = setup();
    let ls = run("ls", &mut fs, &mut bob);
    assert!(ls.contains("engineering/"));
    assert!(ls.contains("public/"));

    let result = try_run("ls finance", &mut fs, &mut bob);
    assert!(result.is_err(), "bob cannot ls finance/");
}

#[test]
fn bob_reads_own_notes() {
    let (mut fs, _, _, mut bob, ..) = setup();
    let content = run("cat home/bob/notes.md", &mut fs, &mut bob);
    assert!(content.contains("review the design doc"));
}

#[test]
fn bob_cannot_read_alice_diary() {
    let (mut fs, _, _, mut bob, ..) = setup();
    let result = try_run("cat home/alice/diary.md", &mut fs, &mut bob);
    assert!(result.is_err(), "bob should not read alice's diary");
}

#[test]
fn bob_tree_matches_permissions() {
    let (mut fs, _, _, mut bob, ..) = setup();
    let tree = run("tree", &mut fs, &mut bob);

    assert!(tree.contains("design.md"), "bob sees engineering files");
    assert!(tree.contains("roadmap.md"));
    assert!(tree.contains("notes.md"), "bob sees his own notes");
    assert!(tree.contains("readme.md"), "bob sees public files");

    assert!(!tree.contains("budget.md"), "bob must NOT see finance files");
    assert!(!tree.contains("diary.md"), "bob must NOT see alice's diary");
    assert!(
        !tree.contains("personal.md"),
        "bob must NOT see carol's personal.md"
    );
}

#[test]
fn bob_cannot_adduser() {
    let (mut fs, _, _, mut bob, ..) = setup();
    let result = try_run("adduser mallory", &mut fs, &mut bob);
    assert!(result.is_err(), "bob should not be able to adduser");
    assert!(result.unwrap_err().contains("permission denied"));
}

#[test]
fn bob_cannot_addgroup() {
    let (mut fs, _, _, mut bob, ..) = setup();
    let result = try_run("addgroup hackers", &mut fs, &mut bob);
    assert!(result.is_err(), "bob should not be able to addgroup");
}

#[test]
fn bob_cannot_deluser() {
    let (mut fs, _, _, mut bob, ..) = setup();
    let result = try_run("deluser alice", &mut fs, &mut bob);
    assert!(result.is_err(), "bob should not be able to deluser");
}

#[test]
fn bob_cannot_chmod_alice_file() {
    let (mut fs, _, _, mut bob, ..) = setup();
    let result = try_run("chmod 777 engineering/design.md", &mut fs, &mut bob);
    assert!(result.is_err(), "bob cannot chmod alice's file");
}
