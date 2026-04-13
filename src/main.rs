use markdownfs::auth::session::Session;
use markdownfs::config::Config;
use markdownfs::db::MarkdownDb;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

#[tokio::main]
async fn main() {
    let config = Config::from_env();
    let db = match MarkdownDb::open(config) {
        Ok(db) => {
            let commits = db.commit_count().await;
            let objects = db.object_count().await;
            if commits > 0 {
                println!(
                    "markdownfs v{} — Loaded from disk ({commits} commits, {objects} objects)",
                    env!("CARGO_PKG_VERSION")
                );
            } else {
                println!(
                    "markdownfs v{} — Markdown Virtual File System",
                    env!("CARGO_PKG_VERSION")
                );
            }
            db
        }
        Err(e) => {
            eprintln!("Warning: failed to load state: {e}");
            eprintln!("Starting fresh.\n");
            MarkdownDb::open_memory()
        }
    };

    let _save_handle = db.spawn_auto_save();

    let mut rl = DefaultEditor::new().expect("failed to initialize readline");
    let history_path = std::env::var("HOME")
        .ok()
        .map(|h| format!("{h}/.markdownfs_history"));

    if let Some(ref path) = history_path {
        let _ = rl.load_history(path);
    }

    let mut session = login_flow(&db, &mut rl).await;
    println!("\nType 'help' for available commands, 'exit' to quit.\n");

    loop {
        let pwd = db.pwd().await;
        let home_prefix = format!("/home/{}", session.username);
        let display_pwd = if pwd == home_prefix {
            "~".to_string()
        } else if let Some(rest) = pwd.strip_prefix(&home_prefix) {
            format!("~{rest}")
        } else {
            pwd
        };
        let prompt = format!("{}@markdownfs:{display_pwd} $ ", session.username);
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
                        handle_edit(line, &db, &mut rl, &session).await;
                    }
                    _ => match db.execute_command(line, &mut session).await {
                        Ok(output) => {
                            if !output.is_empty() {
                                print!("{output}");
                            }
                        }
                        Err(e) => eprintln!("{e}"),
                    },
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

    match db.save().await {
        Ok(()) => {
            let (_, dir) = db.persist_info();
            println!("State saved to {}/", dir.display());
        }
        Err(e) => eprintln!("Warning: failed to save state: {e}"),
    }

    if let Some(ref path) = history_path {
        let _ = rl.save_history(path);
    }

    println!("Goodbye!");
}

async fn login_flow(db: &MarkdownDb, rl: &mut DefaultEditor) -> Session {
    let has_users = db.has_users().await;

    if !has_users {
        println!("\nWelcome! Let's set up your account.");
        loop {
            match rl.readline("Admin username: ") {
                Ok(name) => {
                    let name = name.trim().to_string();
                    if name.is_empty() {
                        eprintln!("Username cannot be empty.");
                        continue;
                    }
                    match db.create_admin(&name).await {
                        Ok(session) => {
                            println!(
                                "\nCreated admin '{}' (uid={}, groups=[{}, wheel])",
                                session.username, session.uid, session.username
                            );
                            println!("Home directory: /home/{}", session.username);
                            return session;
                        }
                        Err(e) => eprintln!("{e}"),
                    }
                }
                Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
                    return Session::root();
                }
                Err(e) => {
                    eprintln!("readline error: {e}");
                    return Session::root();
                }
            }
        }
    } else {
        loop {
            match rl.readline("Login as: ") {
                Ok(name) => {
                    let name = name.trim().to_string();
                    if name.is_empty() {
                        continue;
                    }
                    match db.login(&name).await {
                        Ok(session) => {
                            println!(
                                "Logged in as '{}' (uid={}, gid={})",
                                session.username, session.uid, session.gid
                            );
                            return session;
                        }
                        Err(_) => eprintln!("Unknown user: {name}"),
                    }
                }
                Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
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

async fn handle_edit(
    line: &str,
    db: &MarkdownDb,
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

    if let Ok(content) = db.cat(path).await {
        let text = String::from_utf8_lossy(&content);
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
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("readline error: {e}");
                return;
            }
        }
    }

    if content.ends_with('\n') {
        content.pop();
    }

    if db.stat(path).await.is_err() {
        if let Err(e) = db.touch(path, session.uid, session.gid).await {
            eprintln!("{e}");
            return;
        }
    }

    if let Err(e) = db.write_file(path, content.into_bytes()).await {
        eprintln!("{e}");
        return;
    }

    match db.commit(&format!("edit {path}"), &session.username).await {
        Ok(hash) => println!("[{hash}] edit {path}"),
        Err(e) => eprintln!("auto-commit failed: {e}"),
    }
}
