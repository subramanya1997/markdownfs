use markdownfs::auth::session::Session;
use markdownfs::auth::ROOT_UID;
use markdownfs::cmd;
use markdownfs::cmd::parser;
use markdownfs::fs::VirtualFs;
use markdownfs::persist::PersistManager;
use markdownfs::vcs::Vcs;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

fn main() {
    let cwd = std::env::current_dir().expect("failed to get current directory");
    let persist = PersistManager::new(&cwd);

    let (mut fs, mut vcs) = if persist.state_exists() {
        match persist.load() {
            Ok((fs, vcs)) => {
                let commit_count = vcs.commits.len();
                println!(
                    "markdownfs v0.1.0 — Loaded from disk ({commit_count} commits, {} objects)",
                    vcs.store.object_count()
                );
                (fs, vcs)
            }
            Err(e) => {
                eprintln!("Warning: failed to load state: {e}");
                eprintln!("Starting fresh.\n");
                (VirtualFs::new(), Vcs::new())
            }
        }
    } else {
        println!("markdownfs v0.1.0 — Markdown Virtual File System");
        (VirtualFs::new(), Vcs::new())
    };

    let mut rl = DefaultEditor::new().expect("failed to initialize readline");
    let history_path = dirs_home().map(|h| format!("{h}/.markdownfs_history"));

    if let Some(ref path) = history_path {
        let _ = rl.load_history(path);
    }

    // ─── Login flow ───
    let mut session = login_flow(&mut fs, &mut rl);
    println!(
        "Logged in as '{}' (uid={}, gid={})\n",
        session.username, session.uid, session.gid
    );
    println!("Type 'help' for available commands, 'exit' to quit.\n");

    loop {
        let prompt = format!("{}@markdownfs:{} $ ", session.username, fs.pwd());
        match rl.readline(&prompt) {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(line);

                match line {
                    "exit" | "quit" => break,
                    _ if line.starts_with("edit ") => {
                        handle_edit(line, &mut fs, &mut vcs, &mut rl, &session);
                    }
                    _ => {
                        let result = execute_line(line, &mut fs, &mut vcs, &mut session);
                        match result {
                            Ok(output) => {
                                if !output.is_empty() {
                                    print!("{output}");
                                }
                            }
                            Err(e) => eprintln!("{e}"),
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
            }
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("readline error: {e}");
                break;
            }
        }
    }

    // Save state to disk on exit
    match persist.save(&fs, &vcs) {
        Ok(()) => println!("State saved to {}/", persist.data_dir().display()),
        Err(e) => eprintln!("Warning: failed to save state: {e}"),
    }

    if let Some(ref path) = history_path {
        let _ = rl.save_history(path);
    }

    println!("Goodbye!");
}

fn login_flow(fs: &mut VirtualFs, rl: &mut DefaultEditor) -> Session {
    // Check if there are any non-root users
    let has_users = fs.registry.list_users().iter().any(|u| u.uid != ROOT_UID);

    if !has_users {
        // First run: create admin user
        println!("No users found. Let's create an admin account.");
        loop {
            match rl.readline("Admin username: ") {
                Ok(name) => {
                    let name = name.trim().to_string();
                    if name.is_empty() {
                        eprintln!("Username cannot be empty.");
                        continue;
                    }
                    match fs.registry.add_user(&name, false) {
                        Ok((uid, _)) => {
                            // Add to wheel group for admin privileges
                            let _ = fs.registry.usermod_add_group(&name, "wheel");
                            let user = fs.registry.get_user(uid).unwrap();
                            return Session::new(
                                user.uid,
                                user.groups.first().copied().unwrap_or(0),
                                user.groups.clone(),
                                user.name.clone(),
                            );
                        }
                        Err(e) => eprintln!("{e}"),
                    }
                }
                Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
                    // Fall back to root
                    return Session::root();
                }
                Err(e) => {
                    eprintln!("readline error: {e}");
                    return Session::root();
                }
            }
        }
    } else {
        // Existing users — prompt for login
        loop {
            match rl.readline("Login as: ") {
                Ok(name) => {
                    let name = name.trim().to_string();
                    if name.is_empty() {
                        continue;
                    }
                    if let Some(uid) = fs.registry.lookup_uid(&name) {
                        let user = fs.registry.get_user(uid).unwrap();
                        return Session::new(
                            user.uid,
                            user.groups.first().copied().unwrap_or(0),
                            user.groups.clone(),
                            user.name.clone(),
                        );
                    } else {
                        eprintln!("Unknown user: {name}");
                    }
                }
                Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
                    // Fall back to root
                    return Session::root();
                }
                Err(e) => {
                    eprintln!("readline error: {e}");
                    return Session::root();
                }
            }
        }
    }
}

fn handle_edit(
    line: &str,
    fs: &mut VirtualFs,
    vcs: &mut Vcs,
    rl: &mut DefaultEditor,
    session: &Session,
) {
    let path = line.strip_prefix("edit ").unwrap().trim();

    if path.is_empty() {
        eprintln!("markdownfs: edit: missing file path");
        return;
    }

    if !path.ends_with(".md") {
        eprintln!("markdownfs: only .md files are supported: '{path}'");
        return;
    }

    // Show current content if file exists
    if let Ok(content) = fs.cat(path) {
        let text = String::from_utf8_lossy(content);
        if !text.is_empty() {
            println!("--- Current content of {path} ---");
            for (i, line) in text.lines().enumerate() {
                println!("{:>4} | {line}", i + 1);
            }
            println!("--- End ---");
        }
    }

    println!("Enter new content (type EOF on a blank line to finish, CANCEL to abort):");

    let mut content = String::new();
    let mut line_num = 1;
    loop {
        match rl.readline(&format!("{line_num:>4} | ")) {
            Ok(input) => {
                if input.trim() == "EOF" {
                    break;
                }
                if input.trim() == "CANCEL" {
                    println!("Edit cancelled.");
                    return;
                }
                content.push_str(&input);
                content.push('\n');
                line_num += 1;
            }
            Err(ReadlineError::Interrupted) => {
                println!("\nEdit cancelled.");
                return;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(e) => {
                eprintln!("readline error: {e}");
                return;
            }
        }
    }

    if content.ends_with('\n') {
        content.pop();
    }

    // Create file if it doesn't exist
    if fs.resolve_path(path).is_err() {
        if let Err(e) = fs.touch(path, session.uid, session.gid) {
            eprintln!("{e}");
            return;
        }
    }

    if let Err(e) = fs.write_file(path, content.into_bytes()) {
        eprintln!("{e}");
        return;
    }

    // Auto-commit with author
    match vcs.commit(fs, &format!("edit {path}"), &session.username) {
        Ok(id) => println!("[{}] edit {path}", id.short_hex()),
        Err(e) => eprintln!("auto-commit failed: {e}"),
    }
}

fn execute_line(
    line: &str,
    fs: &mut VirtualFs,
    vcs: &mut Vcs,
    session: &mut Session,
) -> Result<String, markdownfs::error::VfsError> {
    let pipeline = parser::parse_pipeline(line);
    if pipeline.commands.is_empty() {
        return Ok(String::new());
    }

    // Check for VCS commands (not pipeable)
    if let Some(first) = pipeline.commands.first() {
        match first.program.as_str() {
            "commit" => {
                let msg = if first.args.is_empty() {
                    "snapshot"
                } else {
                    &first.args.join(" ")
                };
                let id = vcs.commit(fs, msg, &session.username)?;
                return Ok(format!("[{}] {msg}\n", id.short_hex()));
            }
            "log" => {
                let commits = vcs.log();
                if commits.is_empty() {
                    return Ok("No commits yet.\n".to_string());
                }
                let mut output = String::new();
                for c in commits {
                    let time = chrono::DateTime::from_timestamp(c.timestamp as i64, 0)
                        .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "???".to_string());
                    output.push_str(&format!(
                        "\x1b[33m{}\x1b[0m {} \x1b[36m{}\x1b[0m {}\n",
                        c.id.short_hex(),
                        time,
                        c.author,
                        c.message
                    ));
                }
                return Ok(output);
            }
            "revert" => {
                if first.args.is_empty() {
                    return Err(markdownfs::error::VfsError::InvalidArgs {
                        message: "revert: need commit hash prefix".to_string(),
                    });
                }
                vcs.revert(fs, &first.args[0])?;
                return Ok(format!("Reverted to {}\n", first.args[0]));
            }
            "status" => {
                return vcs.status(fs);
            }
            _ => {}
        }
    }

    cmd::execute_pipeline(&pipeline, fs, session)
}

fn dirs_home() -> Option<String> {
    std::env::var("HOME").ok()
}
