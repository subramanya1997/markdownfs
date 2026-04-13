pub mod parser;

use crate::error::VfsError;
use crate::fs::VirtualFs;
use parser::{ParsedCommand, Pipeline};

pub fn execute_pipeline(pipeline: &Pipeline, fs: &mut VirtualFs) -> Result<String, VfsError> {
    let mut stdin = String::new();
    for (i, cmd) in pipeline.commands.iter().enumerate() {
        let is_last = i == pipeline.commands.len() - 1;
        let has_stdin = i > 0;
        stdin = execute_command(cmd, fs, if has_stdin { Some(&stdin) } else { None })?;
        if !is_last && stdin.is_empty() {
            break;
        }
    }
    Ok(stdin)
}

fn execute_command(
    cmd: &ParsedCommand,
    fs: &mut VirtualFs,
    stdin: Option<&str>,
) -> Result<String, VfsError> {
    match cmd.program.as_str() {
        "ls" => cmd_ls(fs, &cmd.args),
        "cd" => cmd_cd(fs, &cmd.args),
        "pwd" => Ok(format!("{}\n", fs.pwd())),
        "mkdir" => cmd_mkdir(fs, &cmd.args),
        "touch" => cmd_touch(fs, &cmd.args),
        "cat" => cmd_cat(fs, &cmd.args, stdin),
        "rm" => cmd_rm(fs, &cmd.args),
        "rmdir" => cmd_rmdir(fs, &cmd.args),
        "mv" => cmd_mv(fs, &cmd.args),
        "cp" => cmd_cp(fs, &cmd.args),
        "stat" => cmd_stat(fs, &cmd.args),
        "tree" => cmd_tree(fs, &cmd.args),
        "find" => cmd_find(fs, &cmd.args),
        "grep" => cmd_grep(fs, &cmd.args, stdin),
        "head" => cmd_head(&cmd.args, stdin),
        "tail" => cmd_tail(&cmd.args, stdin),
        "wc" => cmd_wc(&cmd.args, stdin),
        "chmod" => cmd_chmod(fs, &cmd.args),
        "ln" => cmd_ln(fs, &cmd.args),
        "echo" => cmd_echo(&cmd.args),
        "write" => cmd_write(fs, &cmd.args, stdin),
        "help" => cmd_help(),
        "clear" => Ok("\x1b[2J\x1b[H".to_string()),
        name => Err(VfsError::UnknownCommand {
            name: name.to_string(),
        }),
    }
}

fn cmd_ls(fs: &VirtualFs, args: &[String]) -> Result<String, VfsError> {
    let mut long_format = false;
    let mut all = false;
    let mut path = None;

    for arg in args {
        match arg.as_str() {
            "-l" => long_format = true,
            "-a" => all = true,
            "-la" | "-al" => {
                long_format = true;
                all = true;
            }
            p => path = Some(p),
        }
    }

    let _ = all; // no hidden files in VFS currently

    let entries = fs.ls(path)?;
    let mut output = String::new();

    if long_format {
        for e in &entries {
            let kind = if e.is_dir {
                'd'
            } else if e.is_symlink {
                'l'
            } else {
                '-'
            };
            let perms = format_permissions(e.mode);
            let time = format_time(e.modified);
            output.push_str(&format!(
                "{kind}{perms} {:>8} {time} {}{}\n",
                e.size,
                e.name,
                if e.is_dir { "/" } else { "" }
            ));
        }
    } else {
        for e in &entries {
            output.push_str(&e.name);
            if e.is_dir {
                output.push('/');
            }
            output.push('\n');
        }
    }

    Ok(output)
}

fn cmd_cd(fs: &mut VirtualFs, args: &[String]) -> Result<String, VfsError> {
    let path = args.first().map(|s| s.as_str()).unwrap_or("/");
    fs.cd(path)?;
    Ok(String::new())
}

fn cmd_mkdir(fs: &mut VirtualFs, args: &[String]) -> Result<String, VfsError> {
    let mut parents = false;
    let mut paths = Vec::new();

    for arg in args {
        if arg == "-p" {
            parents = true;
        } else {
            paths.push(arg.as_str());
        }
    }

    if paths.is_empty() {
        return Err(VfsError::InvalidArgs {
            message: "mkdir: missing operand".to_string(),
        });
    }

    for path in paths {
        if parents {
            fs.mkdir_p(path, 0, 0)?;
        } else {
            fs.mkdir(path, 0, 0)?;
        }
    }
    Ok(String::new())
}

fn cmd_touch(fs: &mut VirtualFs, args: &[String]) -> Result<String, VfsError> {
    if args.is_empty() {
        return Err(VfsError::InvalidArgs {
            message: "touch: missing operand".to_string(),
        });
    }
    for arg in args {
        fs.touch(arg, 0, 0)?;
    }
    Ok(String::new())
}

fn cmd_cat(fs: &VirtualFs, args: &[String], stdin: Option<&str>) -> Result<String, VfsError> {
    if let Some(input) = stdin {
        if args.is_empty() {
            return Ok(input.to_string());
        }
    }

    if args.is_empty() {
        return Err(VfsError::InvalidArgs {
            message: "cat: missing operand".to_string(),
        });
    }

    let mut output = String::new();
    for path in args {
        let content = fs.cat(path)?;
        output.push_str(&String::from_utf8_lossy(content));
    }
    Ok(output)
}

fn cmd_rm(fs: &mut VirtualFs, args: &[String]) -> Result<String, VfsError> {
    let mut recursive = false;
    let mut paths = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-r" | "-rf" | "-fr" => recursive = true,
            p => paths.push(p),
        }
    }

    if paths.is_empty() {
        return Err(VfsError::InvalidArgs {
            message: "rm: missing operand".to_string(),
        });
    }

    for path in paths {
        if recursive {
            fs.rm_rf(path)?;
        } else {
            fs.rm(path)?;
        }
    }
    Ok(String::new())
}

fn cmd_rmdir(fs: &mut VirtualFs, args: &[String]) -> Result<String, VfsError> {
    if args.is_empty() {
        return Err(VfsError::InvalidArgs {
            message: "rmdir: missing operand".to_string(),
        });
    }
    for path in args {
        fs.rmdir(path)?;
    }
    Ok(String::new())
}

fn cmd_mv(fs: &mut VirtualFs, args: &[String]) -> Result<String, VfsError> {
    if args.len() < 2 {
        return Err(VfsError::InvalidArgs {
            message: "mv: need source and destination".to_string(),
        });
    }
    fs.mv(&args[0], &args[1])?;
    Ok(String::new())
}

fn cmd_cp(fs: &mut VirtualFs, args: &[String]) -> Result<String, VfsError> {
    if args.len() < 2 {
        return Err(VfsError::InvalidArgs {
            message: "cp: need source and destination".to_string(),
        });
    }
    fs.cp(&args[0], &args[1], 0, 0)?;
    Ok(String::new())
}

fn cmd_stat(fs: &VirtualFs, args: &[String]) -> Result<String, VfsError> {
    if args.is_empty() {
        return Err(VfsError::InvalidArgs {
            message: "stat: missing operand".to_string(),
        });
    }
    let info = fs.stat(&args[0])?;
    Ok(format!(
        "  File: {}\n  Size: {}\n  Type: {}\n Inode: {}\n  Mode: {:04o}\n   Uid: {}\n   Gid: {}\nCreated: {}\nModified: {}\n",
        args[0], info.size, info.kind, info.inode_id, info.mode,
        info.uid, info.gid,
        format_time(info.created), format_time(info.modified),
    ))
}

fn cmd_tree(fs: &VirtualFs, args: &[String]) -> Result<String, VfsError> {
    let path = args.first().map(|s| s.as_str());
    fs.tree(path, "")
}

fn cmd_find(fs: &VirtualFs, args: &[String]) -> Result<String, VfsError> {
    let mut path = None;
    let mut pattern = None;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "-name" => {
                i += 1;
                if i < args.len() {
                    pattern = Some(args[i].as_str());
                }
            }
            p => {
                if path.is_none() {
                    path = Some(p);
                }
            }
        }
        i += 1;
    }

    let results = fs.find(path, pattern)?;
    Ok(results.join("\n") + if results.is_empty() { "" } else { "\n" })
}

fn cmd_grep(
    fs: &VirtualFs,
    args: &[String],
    stdin: Option<&str>,
) -> Result<String, VfsError> {
    // If piped input, grep over stdin
    if let Some(input) = stdin {
        if args.is_empty() {
            return Err(VfsError::InvalidArgs {
                message: "grep: missing pattern".to_string(),
            });
        }
        let re = regex::Regex::new(&args[0]).map_err(|e| VfsError::InvalidArgs {
            message: format!("invalid regex: {e}"),
        })?;
        let mut output = String::new();
        for line in input.lines() {
            if re.is_match(line) {
                output.push_str(line);
                output.push('\n');
            }
        }
        return Ok(output);
    }

    let mut recursive = false;
    let mut pattern = None;
    let mut path = None;

    for arg in args {
        match arg.as_str() {
            "-r" | "-R" => recursive = true,
            _ => {
                if pattern.is_none() {
                    pattern = Some(arg.as_str());
                } else if path.is_none() {
                    path = Some(arg.as_str());
                }
            }
        }
    }

    let pattern = pattern.ok_or_else(|| VfsError::InvalidArgs {
        message: "grep: missing pattern".to_string(),
    })?;

    let results = fs.grep(pattern, path, recursive)?;
    let mut output = String::new();
    for r in &results {
        output.push_str(&format!("{}:{}:{}\n", r.file, r.line_num, r.line));
    }
    Ok(output)
}

fn cmd_head(args: &[String], stdin: Option<&str>) -> Result<String, VfsError> {
    let input = stdin.ok_or_else(|| VfsError::InvalidArgs {
        message: "head: no input".to_string(),
    })?;

    let mut n = 10usize;
    for i in 0..args.len() {
        if args[i] == "-n" || args[i].starts_with('-') {
            if args[i] == "-n" && i + 1 < args.len() {
                n = args[i + 1].parse().unwrap_or(10);
            } else if args[i].starts_with('-') {
                n = args[i][1..].parse().unwrap_or(10);
            }
        }
    }

    let lines: Vec<&str> = input.lines().take(n).collect();
    Ok(lines.join("\n") + "\n")
}

fn cmd_tail(args: &[String], stdin: Option<&str>) -> Result<String, VfsError> {
    let input = stdin.ok_or_else(|| VfsError::InvalidArgs {
        message: "tail: no input".to_string(),
    })?;

    let mut n = 10usize;
    for i in 0..args.len() {
        if args[i] == "-n" || args[i].starts_with('-') {
            if args[i] == "-n" && i + 1 < args.len() {
                n = args[i + 1].parse().unwrap_or(10);
            } else if args[i].starts_with('-') {
                n = args[i][1..].parse().unwrap_or(10);
            }
        }
    }

    let all_lines: Vec<&str> = input.lines().collect();
    let start = all_lines.len().saturating_sub(n);
    let lines = &all_lines[start..];
    Ok(lines.join("\n") + "\n")
}

fn cmd_wc(args: &[String], stdin: Option<&str>) -> Result<String, VfsError> {
    let input = stdin.ok_or_else(|| VfsError::InvalidArgs {
        message: "wc: no input".to_string(),
    })?;

    let lines = input.lines().count();
    let words = input.split_whitespace().count();
    let bytes = input.len();

    let count_lines = args.contains(&"-l".to_string());
    let count_words = args.contains(&"-w".to_string());
    let count_bytes = args.contains(&"-c".to_string());

    if count_lines {
        Ok(format!("{lines}\n"))
    } else if count_words {
        Ok(format!("{words}\n"))
    } else if count_bytes {
        Ok(format!("{bytes}\n"))
    } else {
        Ok(format!("{lines} {words} {bytes}\n"))
    }
}

fn cmd_chmod(fs: &mut VirtualFs, args: &[String]) -> Result<String, VfsError> {
    if args.len() < 2 {
        return Err(VfsError::InvalidArgs {
            message: "chmod: need mode and file".to_string(),
        });
    }
    let mode = u16::from_str_radix(&args[0], 8).map_err(|_| VfsError::InvalidArgs {
        message: format!("chmod: invalid mode: {}", args[0]),
    })?;
    fs.chmod(&args[1], mode)?;
    Ok(String::new())
}

fn cmd_ln(fs: &mut VirtualFs, args: &[String]) -> Result<String, VfsError> {
    let mut symlink = false;
    let mut targets = Vec::new();

    for arg in args {
        if arg == "-s" {
            symlink = true;
        } else {
            targets.push(arg.as_str());
        }
    }

    if !symlink {
        return Err(VfsError::InvalidArgs {
            message: "ln: only symbolic links supported (use -s)".to_string(),
        });
    }

    if targets.len() < 2 {
        return Err(VfsError::InvalidArgs {
            message: "ln: need target and link name".to_string(),
        });
    }

    fs.ln_s(targets[0], targets[1], 0, 0)?;
    Ok(String::new())
}

fn cmd_echo(args: &[String]) -> Result<String, VfsError> {
    Ok(args.join(" ") + "\n")
}

fn cmd_write(
    fs: &mut VirtualFs,
    args: &[String],
    stdin: Option<&str>,
) -> Result<String, VfsError> {
    if args.is_empty() {
        return Err(VfsError::InvalidArgs {
            message: "write: need file path".to_string(),
        });
    }

    let content = if let Some(input) = stdin {
        input.to_string()
    } else if args.len() > 1 {
        args[1..].join(" ")
    } else {
        return Err(VfsError::InvalidArgs {
            message: "write: need content (pipe input or provide as args)".to_string(),
        });
    };

    // Create file if it doesn't exist
    if fs.resolve_path(&args[0]).is_err() {
        fs.touch(&args[0], 0, 0)?;
    }
    fs.write_file(&args[0], content.into_bytes())?;
    Ok(String::new())
}

fn cmd_help() -> Result<String, VfsError> {
    Ok(r#"mdvfs — Markdown Virtual File System

File commands:
  ls [-l] [path]           List directory contents
  cd [path]                Change directory
  pwd                      Print working directory
  mkdir [-p] <path>        Create directory
  touch <file.md>          Create empty markdown file
  cat <file>               Display file contents
  write <file> [content]   Write content to file (or pipe: echo "text" | write file.md)
  rm [-r] <path>           Remove file or directory
  rmdir <path>             Remove empty directory
  mv <src> <dst>           Move/rename
  cp <src> <dst>           Copy file
  stat <path>              File information
  tree [path]              Directory tree view
  find [path] [-name pat]  Find files by pattern
  grep [-r] <pattern> [path]  Search file contents
  head [-n N]              First N lines (pipe)
  tail [-n N]              Last N lines (pipe)
  wc [-l|-w|-c]            Count lines/words/bytes (pipe)
  chmod <mode> <path>      Change permissions
  ln -s <target> <link>    Create symbolic link
  echo <text>              Print text
  edit <file.md>           Edit file (multi-line input, auto-commits)

VCS commands:
  commit <message>         Commit current state
  log                      Show commit history
  revert <hash>            Revert to a commit
  status                   Show modified files

Pipes:  grep "TODO" notes/ | head -5 | wc -l
"#
    .to_string())
}

fn format_permissions(mode: u16) -> String {
    let mut s = String::with_capacity(9);
    for shift in [6, 3, 0] {
        let bits = (mode >> shift) & 0o7;
        s.push(if bits & 4 != 0 { 'r' } else { '-' });
        s.push(if bits & 2 != 0 { 'w' } else { '-' });
        s.push(if bits & 1 != 0 { 'x' } else { '-' });
    }
    s
}

fn format_time(epoch: u64) -> String {
    let dt = chrono::DateTime::from_timestamp(epoch as i64, 0);
    match dt {
        Some(d) => d.format("%b %d %H:%M").to_string(),
        None => "???".to_string(),
    }
}
