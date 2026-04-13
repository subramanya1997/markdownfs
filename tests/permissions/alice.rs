use super::*;

#[test]
fn alice_sees_public_and_engineering() {
    let (mut fs, _, mut alice, ..) = setup();
    let ls = run("ls", &mut fs, &mut alice);
    assert!(ls.contains("public/"), "alice should see public/");
    assert!(ls.contains("engineering/"), "alice should see engineering/");
    assert!(ls.contains("home/"), "alice should see home/");
    assert!(ls.contains("tmp/"), "alice should see tmp/");
}

#[test]
fn alice_cannot_see_finance_contents() {
    let (mut fs, _, mut alice, ..) = setup();
    let result = try_run("ls finance", &mut fs, &mut alice);
    assert!(result.is_err(), "alice should not be able to ls finance/");
}

#[test]
fn alice_reads_engineering_files() {
    let (mut fs, _, mut alice, ..) = setup();
    let content = run("cat engineering/design.md", &mut fs, &mut alice);
    assert!(content.contains("Architecture overview"));
    let content = run("cat engineering/roadmap.md", &mut fs, &mut alice);
    assert!(content.contains("Launch MVP"));
}

#[test]
fn alice_reads_own_private_files() {
    let (mut fs, _, mut alice, ..) = setup();
    let content = run("cat home/alice/diary.md", &mut fs, &mut alice);
    assert!(content.contains("designed a filesystem"));
}

#[test]
fn alice_cannot_read_bob_private_files() {
    let (mut fs, _, mut alice, ..) = setup();
    let result = try_run("cat home/bob/notes.md", &mut fs, &mut alice);
    assert!(
        result.is_err(),
        "alice should not read bob's private notes"
    );
}

#[test]
fn alice_cannot_read_carol_private_files() {
    let (mut fs, _, mut alice, ..) = setup();
    let result = try_run("cat home/carol/personal.md", &mut fs, &mut alice);
    assert!(
        result.is_err(),
        "alice should not read carol's private files"
    );
}

#[test]
fn alice_tree_hides_inaccessible() {
    let (mut fs, _, mut alice, ..) = setup();
    let tree = run("tree", &mut fs, &mut alice);

    assert!(tree.contains("readme.md"), "alice sees public/readme.md");
    assert!(tree.contains("design.md"), "alice sees engineering/design.md");
    assert!(tree.contains("roadmap.md"), "alice sees engineering/roadmap.md");
    assert!(tree.contains("diary.md"), "alice sees her own diary");
    assert!(tree.contains("alice-tmp.md"), "alice sees her tmp file");
    assert!(tree.contains("bob-tmp.md"), "alice sees bob's tmp file (644)");

    assert!(!tree.contains("budget.md"), "alice must NOT see finance/budget.md");
    assert!(!tree.contains("notes.md"), "alice must NOT see bob's notes.md (600)");
    assert!(
        !tree.contains("personal.md"),
        "alice must NOT see carol's personal.md (600)"
    );
}

#[test]
fn alice_find_hides_inaccessible() {
    let (mut fs, _, mut alice, ..) = setup();
    let found = run("find . -name *.md", &mut fs, &mut alice);

    assert!(found.contains("readme.md"));
    assert!(found.contains("design.md"));
    assert!(found.contains("roadmap.md"));
    assert!(found.contains("diary.md"));

    assert!(!found.contains("budget.md"), "alice must NOT find budget.md");
    assert!(
        !found.contains("home/bob/notes.md"),
        "alice must NOT find bob's notes"
    );
    assert!(
        !found.contains("personal.md"),
        "alice must NOT find carol's personal.md"
    );
}

#[test]
fn alice_grep_only_searches_accessible() {
    let (mut fs, _, mut alice, ..) = setup();
    let results = run("grep -r 500K .", &mut fs, &mut alice);
    assert!(
        !results.contains("budget.md"),
        "alice grep must NOT return finance/budget.md"
    );
    assert!(
        !results.contains("$500K"),
        "alice grep must NOT leak finance content"
    );
}

#[test]
fn alice_can_write_in_engineering() {
    let (mut fs, _, mut alice, ..) = setup();
    run("touch engineering/new-doc.md", &mut fs, &mut alice);
    let ls = run("ls engineering", &mut fs, &mut alice);
    assert!(ls.contains("new-doc.md"));
}

#[test]
fn alice_can_chmod_own_files() {
    let (mut fs, _, mut alice, ..) = setup();
    run("chmod 644 engineering/design.md", &mut fs, &mut alice);
    let stat = run("stat engineering/design.md", &mut fs, &mut alice);
    assert!(stat.contains("0644"));
}
