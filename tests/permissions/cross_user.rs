use super::*;

#[test]
fn all_users_see_different_trees() {
    let (mut fs, mut root, mut alice, mut bob, mut carol, mut agent) = setup();

    let root_tree = run("tree", &mut fs, &mut root);
    let alice_tree = run("tree", &mut fs, &mut alice);
    let bob_tree = run("tree", &mut fs, &mut bob);
    let carol_tree = run("tree", &mut fs, &mut carol);
    let agent_tree = run("tree", &mut fs, &mut agent);

    let root_lines = root_tree.lines().count();
    let alice_lines = alice_tree.lines().count();
    let bob_lines = bob_tree.lines().count();
    let carol_lines = carol_tree.lines().count();

    assert!(root_lines > alice_lines, "root should see more than alice");
    assert!(root_lines > bob_lines, "root should see more than bob");
    assert!(root_lines > carol_lines, "root should see more than carol");

    assert!(carol_tree.contains("budget.md"), "carol sees budget.md");
    assert!(!alice_tree.contains("budget.md"), "alice does NOT see budget.md");
    assert!(!bob_tree.contains("budget.md"), "bob does NOT see budget.md");
    assert!(!agent_tree.contains("budget.md"), "agent does NOT see budget.md");

    assert!(alice_tree.contains("diary.md"));
    assert!(!bob_tree.contains("diary.md"));
    assert!(!carol_tree.contains("diary.md"));
    assert!(!agent_tree.contains("diary.md"));

    assert!(bob_tree.contains("notes.md"));
    assert!(!alice_tree.contains("notes.md"));
    assert!(!carol_tree.contains("notes.md"));

    assert!(carol_tree.contains("personal.md"));
    assert!(!alice_tree.contains("personal.md"));
    assert!(!bob_tree.contains("personal.md"));
}

#[test]
fn all_users_find_different_md_files() {
    let (mut fs, mut root, mut alice, mut bob, mut carol, mut agent) = setup();

    let root_find = run("find . -name *.md", &mut fs, &mut root);
    let alice_find = run("find . -name *.md", &mut fs, &mut alice);
    let bob_find = run("find . -name *.md", &mut fs, &mut bob);
    let carol_find = run("find . -name *.md", &mut fs, &mut carol);
    let agent_find = run("find . -name *.md", &mut fs, &mut agent);

    let root_count = root_find.lines().count();
    let alice_count = alice_find.lines().count();
    let bob_count = bob_find.lines().count();
    let carol_count = carol_find.lines().count();
    let agent_count = agent_find.lines().count();

    assert_eq!(root_count, 9, "root should find all 9 .md files");
    assert!(alice_count < root_count, "alice finds fewer than root");
    assert!(bob_count < root_count, "bob finds fewer than root");
    assert!(carol_count < root_count, "carol finds fewer than root");
    assert!(agent_count < root_count, "agent finds fewer than root");

    assert!(!alice_find.contains("budget.md"));
    assert!(!alice_find.contains("home/bob/notes.md"));
    assert!(!alice_find.contains("personal.md"));

    assert!(carol_find.contains("budget.md"));
    assert!(carol_find.contains("design.md"), "eng files are 664, carol has other:r--");
}

#[test]
fn grep_isolation_across_users() {
    let (mut fs, mut root, mut alice, mut bob, mut carol, _) = setup();

    let root_grep = run("grep -r TODO .", &mut fs, &mut root);
    let alice_grep = run("grep -r TODO .", &mut fs, &mut alice);
    let bob_grep = run("grep -r TODO .", &mut fs, &mut bob);
    let carol_grep = run("grep -r TODO .", &mut fs, &mut carol);

    assert!(root_grep.contains("TODO"), "root finds TODO in bob's notes");
    assert!(bob_grep.contains("TODO"), "bob finds TODO in his own notes");
    assert!(!alice_grep.contains("TODO"), "alice must NOT see bob's private TODO");
    assert!(!carol_grep.contains("TODO"), "carol must NOT see bob's private TODO");

    let root_grep = run("grep -r 500K .", &mut fs, &mut root);
    let carol_grep = run("grep -r 500K .", &mut fs, &mut carol);
    let alice_grep = run("grep -r 500K .", &mut fs, &mut alice);
    let bob_grep = run("grep -r 500K .", &mut fs, &mut bob);

    assert!(root_grep.contains("500K"), "root finds 500K");
    assert!(carol_grep.contains("500K"), "carol finds 500K in her budget");
    assert!(!alice_grep.contains("500K"), "alice must NOT see finance data");
    assert!(!bob_grep.contains("500K"), "bob must NOT see finance data");
}
