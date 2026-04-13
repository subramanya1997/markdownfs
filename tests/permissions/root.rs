use super::*;

#[test]
fn root_sees_all_top_level() {
    let (mut fs, mut root, ..) = setup();
    let ls = run("ls", &mut fs, &mut root);
    assert!(ls.contains("public/"), "root should see public/");
    assert!(ls.contains("engineering/"), "root should see engineering/");
    assert!(ls.contains("finance/"), "root should see finance/");
    assert!(ls.contains("home/"), "root should see home/");
    assert!(ls.contains("tmp/"), "root should see tmp/");
}

#[test]
fn root_sees_all_in_tree() {
    let (mut fs, mut root, ..) = setup();
    let tree = run("tree", &mut fs, &mut root);
    assert!(tree.contains("readme.md"));
    assert!(tree.contains("design.md"));
    assert!(tree.contains("roadmap.md"));
    assert!(tree.contains("budget.md"));
    assert!(tree.contains("diary.md"));
    assert!(tree.contains("notes.md"));
    assert!(tree.contains("personal.md"));
    assert!(tree.contains("alice-tmp.md"));
    assert!(tree.contains("bob-tmp.md"));
}

#[test]
fn root_reads_all_files() {
    let (mut fs, mut root, ..) = setup();
    let content = run("cat home/alice/diary.md", &mut fs, &mut root);
    assert!(content.contains("designed a filesystem"));
    let content = run("cat finance/budget.md", &mut fs, &mut root);
    assert!(content.contains("$500K"));
}

#[test]
fn root_can_write_anywhere() {
    let (mut fs, mut root, ..) = setup();
    run("touch finance/root-added.md", &mut fs, &mut root);
    run(
        "write finance/root-added.md Root can write here",
        &mut fs,
        &mut root,
    );
    let content = run("cat finance/root-added.md", &mut fs, &mut root);
    assert!(content.contains("Root can write here"));
}

#[test]
fn root_can_delete_in_any_sticky_dir() {
    let (mut fs, mut root, ..) = setup();
    run("rm tmp/alice-tmp.md", &mut fs, &mut root);
    let ls = run("ls tmp", &mut fs, &mut root);
    assert!(!ls.contains("alice-tmp.md"));
}
