use markdownfs::auth::session::Session;
use markdownfs::cmd;
use markdownfs::cmd::parser;
use markdownfs::fs::VirtualFs;

mod root;
mod alice;
mod bob;
mod carol;
mod agent;
mod write_enforce;
mod special_bits;
mod ownership;
mod cross_user;
mod adversarial;

fn run(line: &str, fs: &mut VirtualFs, session: &mut Session) -> String {
    let pipeline = parser::parse_pipeline(line);
    cmd::execute_pipeline(&pipeline, fs, session).unwrap()
}

#[allow(dead_code)]
fn run_err(line: &str, fs: &mut VirtualFs, session: &mut Session) -> String {
    let pipeline = parser::parse_pipeline(line);
    cmd::execute_pipeline(&pipeline, fs, session)
        .unwrap_err()
        .to_string()
}

fn try_run(line: &str, fs: &mut VirtualFs, session: &mut Session) -> Result<String, String> {
    let pipeline = parser::parse_pipeline(line);
    cmd::execute_pipeline(&pipeline, fs, session).map_err(|e| e.to_string())
}

/// Build a multi-tenant filesystem:
///
///   /
///   ├── public/            (root:root 755)  — everyone can read
///   │   └── readme.md      (root:root 644)
///   ├── engineering/        (root:eng  2775) — setgid, eng group can write
///   │   ├── design.md      (alice:eng 664)
///   │   └── roadmap.md     (bob:eng   664)
///   ├── finance/            (root:fin  2770) — fin group only, no others
///   │   └── budget.md      (carol:fin 660)
///   ├── home/
///   │   ├── alice/          (alice:alice 750)
///   │   │   └── diary.md   (alice:alice 600)
///   │   ├── bob/            (bob:bob 750)
///   │   │   └── notes.md   (bob:bob 600)
///   │   └── carol/          (carol:carol 750)
///   │       └── personal.md (carol:carol 600)
///   └── tmp/                (root:root 1777) — sticky bit
///       ├── alice-tmp.md   (alice:alice 644)
///       └── bob-tmp.md     (bob:bob 644)
///
/// Users:
///   - root (uid=0) — superuser
///   - alice (uid=1) — member of: alice, engineering (regular user; wheel = admin)
///   - bob (uid=2) — member of: bob, engineering
///   - carol (uid=3) — member of: carol, finance
///   - agent-x (uid=4) — agent, member of: agent-x, engineering
fn setup() -> (VirtualFs, Session, Session, Session, Session, Session) {
    let mut fs = VirtualFs::new();
    let mut root = Session::root();

    // Create groups
    run("addgroup engineering", &mut fs, &mut root);
    run("addgroup finance", &mut fs, &mut root);

    // Create users
    run("adduser alice", &mut fs, &mut root);
    run("adduser bob", &mut fs, &mut root);
    run("adduser carol", &mut fs, &mut root);
    run("addagent agent-x", &mut fs, &mut root);

    // Group memberships
    run("usermod -aG engineering alice", &mut fs, &mut root);
    run("usermod -aG engineering bob", &mut fs, &mut root);
    run("usermod -aG finance carol", &mut fs, &mut root);
    run("usermod -aG engineering agent-x", &mut fs, &mut root);

    // Build directory structure
    // /public (755 root:root)
    run("mkdir public", &mut fs, &mut root);
    run("touch public/readme.md", &mut fs, &mut root);
    run("write public/readme.md # Welcome to markdownfs", &mut fs, &mut root);

    // /engineering (2775 root:eng) — setgid so new files inherit eng group
    run("mkdir engineering", &mut fs, &mut root);
    run("chown root:engineering engineering", &mut fs, &mut root);
    run("chmod 2775 engineering", &mut fs, &mut root);

    // /finance (2770 root:fin) — no other access
    run("mkdir finance", &mut fs, &mut root);
    run("chown root:finance finance", &mut fs, &mut root);
    run("chmod 2770 finance", &mut fs, &mut root);

    // /home directories (adduser already creates /home and /home/<user>,
    // so use mkdir -p to avoid AlreadyExists errors)
    run("mkdir -p home/alice", &mut fs, &mut root);
    run("chown alice:alice home/alice", &mut fs, &mut root);
    run("chmod 750 home/alice", &mut fs, &mut root);

    run("mkdir -p home/bob", &mut fs, &mut root);
    run("chown bob:bob home/bob", &mut fs, &mut root);
    run("chmod 750 home/bob", &mut fs, &mut root);

    run("mkdir -p home/carol", &mut fs, &mut root);
    run("chown carol:carol home/carol", &mut fs, &mut root);
    run("chmod 750 home/carol", &mut fs, &mut root);

    // /tmp (1777 root:root) — sticky bit
    run("mkdir tmp", &mut fs, &mut root);
    run("chmod 1777 tmp", &mut fs, &mut root);

    // Create sessions for each user
    let alice_user = fs.registry.get_user(1).unwrap();
    let alice = Session::new(
        alice_user.uid,
        alice_user.groups[0],
        alice_user.groups.clone(),
        alice_user.name.clone(),
    );

    let bob_user = fs.registry.get_user(2).unwrap();
    let bob = Session::new(
        bob_user.uid,
        bob_user.groups[0],
        bob_user.groups.clone(),
        bob_user.name.clone(),
    );

    let carol_user = fs.registry.get_user(3).unwrap();
    let carol = Session::new(
        carol_user.uid,
        carol_user.groups[0],
        carol_user.groups.clone(),
        carol_user.name.clone(),
    );

    let agent_user = fs.registry.get_user(4).unwrap();
    let agent = Session::new(
        agent_user.uid,
        agent_user.groups[0],
        agent_user.groups.clone(),
        agent_user.name.clone(),
    );

    // Now create files as specific users

    // Alice writes in engineering
    let mut alice_mut = alice.clone();
    run("touch engineering/design.md", &mut fs, &mut alice_mut);
    run(
        "write engineering/design.md # System Design\n\nArchitecture overview here.",
        &mut fs,
        &mut alice_mut,
    );
    run("chmod 664 engineering/design.md", &mut fs, &mut alice_mut);

    // Bob writes in engineering
    let mut bob_mut = bob.clone();
    run("touch engineering/roadmap.md", &mut fs, &mut bob_mut);
    run(
        "write engineering/roadmap.md # Roadmap\n\nQ1: Launch MVP",
        &mut fs,
        &mut bob_mut,
    );
    run("chmod 664 engineering/roadmap.md", &mut fs, &mut bob_mut);

    // Carol writes in finance
    let mut carol_mut = carol.clone();
    run("touch finance/budget.md", &mut fs, &mut carol_mut);
    run(
        "write finance/budget.md # Q1 Budget\n\nTotal: $500K",
        &mut fs,
        &mut carol_mut,
    );
    run("chmod 660 finance/budget.md", &mut fs, &mut carol_mut);

    // Private home files
    run("touch home/alice/diary.md", &mut fs, &mut alice_mut);
    run(
        "write home/alice/diary.md Dear diary, today I designed a filesystem.",
        &mut fs,
        &mut alice_mut,
    );
    run("chmod 600 home/alice/diary.md", &mut fs, &mut alice_mut);

    run("touch home/bob/notes.md", &mut fs, &mut bob_mut);
    run(
        "write home/bob/notes.md TODO: review the design doc",
        &mut fs,
        &mut bob_mut,
    );
    run("chmod 600 home/bob/notes.md", &mut fs, &mut bob_mut);

    run("touch home/carol/personal.md", &mut fs, &mut carol_mut);
    run(
        "write home/carol/personal.md My private financial notes.",
        &mut fs,
        &mut carol_mut,
    );
    run("chmod 600 home/carol/personal.md", &mut fs, &mut carol_mut);

    // Tmp files
    run("touch tmp/alice-tmp.md", &mut fs, &mut alice_mut);
    run(
        "write tmp/alice-tmp.md scratch work",
        &mut fs,
        &mut alice_mut,
    );

    run("touch tmp/bob-tmp.md", &mut fs, &mut bob_mut);
    run("write tmp/bob-tmp.md draft notes", &mut fs, &mut bob_mut);

    (fs, root, alice, bob, carol, agent)
}
