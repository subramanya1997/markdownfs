use super::*;

#[test]
fn agent_sees_engineering_files() {
    let (mut fs, _, _, _, _, mut agent) = setup();
    let content = run("cat engineering/design.md", &mut fs, &mut agent);
    assert!(content.contains("Architecture overview"));
}

#[test]
fn agent_cannot_access_finance() {
    let (mut fs, _, _, _, _, mut agent) = setup();
    let result = try_run("ls finance", &mut fs, &mut agent);
    assert!(result.is_err(), "agent cannot access finance/");
}

#[test]
fn agent_tree_filtered() {
    let (mut fs, _, _, _, _, mut agent) = setup();
    let tree = run("tree", &mut fs, &mut agent);

    assert!(tree.contains("design.md"), "agent sees engineering files");
    assert!(tree.contains("roadmap.md"));
    assert!(tree.contains("readme.md"), "agent sees public files");

    assert!(!tree.contains("budget.md"), "agent must NOT see finance files");
    assert!(!tree.contains("diary.md"), "agent must NOT see private diaries");
}

#[test]
fn agent_cannot_create_users() {
    let (mut fs, _, _, _, _, mut agent) = setup();
    let result = try_run("adduser hacker", &mut fs, &mut agent);
    assert!(result.is_err(), "agent should not be able to add users");
}

#[test]
fn agent_can_write_in_engineering() {
    let (mut fs, _, _, _, _, mut agent) = setup();
    run("touch engineering/agent-report.md", &mut fs, &mut agent);
    let ls = run("ls engineering", &mut fs, &mut agent);
    assert!(ls.contains("agent-report.md"));
}
