use mdvfs::cmd;
use mdvfs::cmd::parser;
use mdvfs::fs::VirtualFs;
use mdvfs::persist::PersistManager;
use mdvfs::vcs::Vcs;
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
                    "mdvfs v0.1.0 — Loaded from disk ({commit_count} commits, {} objects)",
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
        println!("mdvfs v0.1.0 — Markdown Virtual File System");
        (VirtualFs::new(), Vcs::new())
    };

    println!("Type 'help' for available commands, 'exit' to quit.\n");

    let mut rl = DefaultEditor::new().expect("failed to initialize readline");
    let history_path = dirs_home().map(|h| format!("{h}/.mdvfs_history"));

    if let Some(ref path) = history_path {
        let _ = rl.load_history(path);
    }

    loop {
        let prompt = format!("mdvfs:{} $ ", fs.pwd());
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
                        handle_edit(line, &mut fs, &mut vcs, &mut rl);
                    }
                    _ => {
                        let result = execute_line(line, &mut fs, &mut vcs);
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

fn handle_edit(line: &str, fs: &mut VirtualFs, vcs: &mut Vcs, rl: &mut DefaultEditor) {
    let path = line.strip_prefix("edit ").unwrap().trim();

    if path.is_empty() {
        eprintln!("mdvfs: edit: missing file path");
        return;
    }

    // Validate it's a .md file
    if !path.ends_with(".md") {
        eprintln!("mdvfs: only .md files are supported: '{path}'");
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

    // Remove trailing newline if content is non-empty
    if content.ends_with('\n') {
        content.pop();
    }

    // Create file if it doesn't exist
    if fs.resolve_path(path).is_err() {
        if let Err(e) = fs.touch(path, 0, 0) {
            eprintln!("{e}");
            return;
        }
    }

    if let Err(e) = fs.write_file(path, content.into_bytes()) {
        eprintln!("{e}");
        return;
    }

    // Auto-commit
    match vcs.commit(fs, &format!("edit {path}")) {
        Ok(id) => println!("[{}] edit {path}", id.short_hex()),
        Err(e) => eprintln!("auto-commit failed: {e}"),
    }
}

fn execute_line(
    line: &str,
    fs: &mut VirtualFs,
    vcs: &mut Vcs,
) -> Result<String, mdvfs::error::VfsError> {
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
                let id = vcs.commit(fs, msg)?;
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
                        "\x1b[33m{}\x1b[0m {} {}\n",
                        c.id.short_hex(),
                        time,
                        c.message
                    ));
                }
                return Ok(output);
            }
            "revert" => {
                if first.args.is_empty() {
                    return Err(mdvfs::error::VfsError::InvalidArgs {
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

    cmd::execute_pipeline(&pipeline, fs)
}

fn dirs_home() -> Option<String> {
    std::env::var("HOME").ok()
}
