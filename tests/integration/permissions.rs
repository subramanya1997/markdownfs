use super::*;
use markdownfs::auth::session::Session;
use markdownfs::cmd;
use markdownfs::cmd::parser;

#[test]
fn test_permission_denied_read() {
    let mut fs = VirtualFs::new();
    let mut root_session = Session::root();

    let pipeline = parser::parse_pipeline("touch secret.md");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut root_session).unwrap();
    let pipeline = parser::parse_pipeline("write secret.md top secret");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut root_session).unwrap();
    let pipeline = parser::parse_pipeline("chmod 600 secret.md");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut root_session).unwrap();

    fs.registry.add_user("alice", false).unwrap();
    let mut alice_session = Session::new(1, 2, vec![2], "alice".to_string());

    let pipeline = parser::parse_pipeline("cat secret.md");
    let result = cmd::execute_pipeline(&pipeline, &mut fs, &mut alice_session);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("permission denied"));
}

#[test]
fn test_permission_visibility() {
    let mut fs = VirtualFs::new();
    let mut root_session = Session::root();

    let pipeline = parser::parse_pipeline("touch public.md");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut root_session).unwrap();
    let pipeline = parser::parse_pipeline("touch private.md");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut root_session).unwrap();
    let pipeline = parser::parse_pipeline("chmod 600 private.md");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut root_session).unwrap();

    fs.registry.add_user("alice", false).unwrap();
    let mut alice_session = Session::new(1, 2, vec![2], "alice".to_string());

    let pipeline = parser::parse_pipeline("ls");
    let output = cmd::execute_pipeline(&pipeline, &mut fs, &mut alice_session).unwrap();
    assert!(output.contains("public.md"));
    assert!(!output.contains("private.md"));
}

#[test]
fn test_user_management() {
    let mut fs = VirtualFs::new();
    let mut root_session = Session::root();

    let pipeline = parser::parse_pipeline("adduser bob");
    let output = cmd::execute_pipeline(&pipeline, &mut fs, &mut root_session).unwrap();
    assert!(output.contains("bob"));
    assert!(output.contains("uid="));

    let pipeline = parser::parse_pipeline("whoami");
    let output = cmd::execute_pipeline(&pipeline, &mut fs, &mut root_session).unwrap();
    assert_eq!(output.trim(), "root");

    let pipeline = parser::parse_pipeline("id bob");
    let output = cmd::execute_pipeline(&pipeline, &mut fs, &mut root_session).unwrap();
    assert!(output.contains("uid=1(bob)"));

    let pipeline = parser::parse_pipeline("groups bob");
    let output = cmd::execute_pipeline(&pipeline, &mut fs, &mut root_session).unwrap();
    assert!(output.contains("bob"));
}

#[test]
fn test_su_and_ownership() {
    let mut fs = VirtualFs::new();
    let mut session = Session::root();

    let pipeline = parser::parse_pipeline("adduser alice");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut session).unwrap();
    let pipeline = parser::parse_pipeline("mkdir home");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut session).unwrap();
    let pipeline = parser::parse_pipeline("mkdir home/alice");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut session).unwrap();
    let pipeline = parser::parse_pipeline("chown alice: home/alice");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut session).unwrap();

    let pipeline = parser::parse_pipeline("su alice");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut session).unwrap();
    assert_eq!(session.username, "alice");

    let pipeline = parser::parse_pipeline("touch home/alice/notes.md");
    cmd::execute_pipeline(&pipeline, &mut fs, &mut session).unwrap();

    let info = fs.stat("home/alice/notes.md").unwrap();
    assert_eq!(info.uid, 1); // alice's uid
}
