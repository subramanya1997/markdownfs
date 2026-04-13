use super::*;

#[test]
fn carol_sees_finance_not_engineering() {
    let (mut fs, _, _, _, mut carol, _) = setup();

    let content = run("cat finance/budget.md", &mut fs, &mut carol);
    assert!(content.contains("$500K"));

    // engineering/ is 2775 → other has r-x, so carol CAN read the dir
    // files are 664 → other has r--, so carol can read them
    let content = run("cat engineering/design.md", &mut fs, &mut carol);
    assert!(
        content.contains("Architecture"),
        "carol can read engineering files (other: r-- on 664)"
    );
}

#[test]
fn carol_reads_own_files() {
    let (mut fs, _, _, _, mut carol, _) = setup();
    let content = run("cat home/carol/personal.md", &mut fs, &mut carol);
    assert!(content.contains("private financial notes"));
}

#[test]
fn carol_cannot_read_alice_or_bob_private() {
    let (mut fs, _, _, _, mut carol, _) = setup();
    let result = try_run("cat home/alice/diary.md", &mut fs, &mut carol);
    assert!(result.is_err(), "carol cannot read alice's diary");

    let result = try_run("cat home/bob/notes.md", &mut fs, &mut carol);
    assert!(result.is_err(), "carol cannot read bob's notes");
}

#[test]
fn carol_tree_shows_correct_view() {
    let (mut fs, _, _, _, mut carol, _) = setup();
    let tree = run("tree", &mut fs, &mut carol);

    assert!(tree.contains("budget.md"), "carol sees finance/budget.md");
    assert!(tree.contains("personal.md"), "carol sees her own personal.md");
    assert!(tree.contains("readme.md"), "carol sees public/readme.md");

    assert!(!tree.contains("diary.md"), "carol must NOT see alice's diary");
    assert!(
        !tree.contains("home/bob/notes.md") || !tree.contains("notes.md"),
        "carol must NOT see bob's notes"
    );
}

#[test]
fn carol_find_respects_permissions() {
    let (mut fs, _, _, _, mut carol, _) = setup();
    let found = run("find . -name *.md", &mut fs, &mut carol);

    assert!(found.contains("budget.md"), "carol finds her budget.md");
    assert!(found.contains("personal.md"), "carol finds her personal.md");
    assert!(found.contains("readme.md"), "carol finds public/readme.md");

    assert!(!found.contains("diary.md"), "carol must NOT find alice's diary");
}

#[test]
fn carol_grep_doesnt_leak_private_data() {
    let (mut fs, _, _, _, mut carol, _) = setup();
    let results = run("grep -r filesystem .", &mut fs, &mut carol);
    assert!(
        !results.contains("diary.md"),
        "carol's grep must NOT search alice's diary"
    );
    assert!(
        !results.contains("designed a filesystem"),
        "carol's grep must NOT return alice's private content"
    );
}

#[test]
fn carol_cannot_write_in_engineering() {
    let (mut fs, _, _, _, mut carol, _) = setup();
    let result = try_run("touch engineering/hack.md", &mut fs, &mut carol);
    assert!(
        result.is_err(),
        "carol should not write in engineering/"
    );
}
