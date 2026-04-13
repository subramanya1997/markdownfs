use super::*;
use markdownfs::auth::session::Session;
use markdownfs::vcs::Vcs;

#[test]
fn test_full_project_workflow() {
    let mut fs = VirtualFs::new();
    let mut vcs = Vcs::new();

    // Setup project
    exec("mkdir -p src/lib", &mut fs);
    exec("mkdir -p docs/api", &mut fs);
    exec("mkdir tests", &mut fs);

    exec("cd src", &mut fs);
    exec("touch main.md", &mut fs);
    exec("write main.md # Main entry point", &mut fs);
    exec("cd lib", &mut fs);
    exec("touch utils.md", &mut fs);
    exec("write utils.md # Utility functions\n\n## Helpers\n\n- parse()\n- format()", &mut fs);
    exec("cd /docs/api", &mut fs);
    exec("touch endpoints.md", &mut fs);
    exec("write endpoints.md # API Endpoints\n\n## GET /users\n## POST /users", &mut fs);
    exec("cd /tests", &mut fs);
    exec("touch test_api.md", &mut fs);
    exec("write test_api.md # API Tests\n\n- [ ] Test GET\n- [ ] Test POST", &mut fs);
    exec("cd /", &mut fs);
    exec("touch readme.md", &mut fs);
    exec("write readme.md # My Project\n\nA cool project.", &mut fs);

    let id1 = vcs.commit(&fs, "initial project setup", "root").unwrap();

    // Verify tree
    let tree = exec("tree", &mut fs);
    assert!(tree.contains("src/"));
    assert!(tree.contains("docs/"));
    assert!(tree.contains("tests/"));

    // Verify find
    let found = exec("find . -name *.md", &mut fs);
    assert_eq!(found.lines().count(), 5); // main, utils, endpoints, test_api, readme

    // Verify grep
    let grep = exec("grep -r API .", &mut fs);
    assert!(grep.contains("endpoints.md"));
    assert!(grep.contains("test_api.md"));

    // Make changes
    exec("write readme.md # My Project v2\n\nImproved.", &mut fs);
    vcs.commit(&fs, "update readme", "root").unwrap();

    // Revert
    vcs.revert(&mut fs, &id1.short_hex()).unwrap();
    assert!(exec("cat readme.md", &mut fs).contains("A cool project."));
}

#[test]
fn test_multiuser_workflow() {
    let mut fs = VirtualFs::new();
    let mut session = Session::root();

    // Admin creates users
    exec_s("adduser alice", &mut fs, &mut session);
    exec_s("adduser bob", &mut fs, &mut session);
    exec_s("addgroup engineering", &mut fs, &mut session);
    exec_s("usermod -aG engineering alice", &mut fs, &mut session);
    exec_s("usermod -aG engineering bob", &mut fs, &mut session);

    // Setup shared space
    exec_s("mkdir shared", &mut fs, &mut session);
    exec_s("chown root:engineering shared", &mut fs, &mut session);
    exec_s("chmod 2775 shared", &mut fs, &mut session);

    // Switch to alice
    exec_s("su alice", &mut fs, &mut session);
    assert_eq!(session.username, "alice");
    exec_s("touch shared/alice-work.md", &mut fs, &mut session);
    exec_s("write shared/alice-work.md Alice design notes", &mut fs, &mut session);
    exec_s("chmod 664 shared/alice-work.md", &mut fs, &mut session);

    // Switch back to root, then to bob
    session = Session::root();
    exec_s("su bob", &mut fs, &mut session);
    assert_eq!(session.username, "bob");
    exec_s("touch shared/bob-work.md", &mut fs, &mut session);

    // Bob can read alice's work (group permission via setgid inheritance)
    let content = exec_s("cat shared/alice-work.md", &mut fs, &mut session);
    assert!(content.contains("Alice design notes"));
}
