pub mod parser;

use crate::auth::perms::{has_sticky_bit, Access};
use crate::auth::session::Session;
use crate::auth::{ROOT_GID, ROOT_UID, WHEEL_GID};
use crate::error::VfsError;
use crate::fs::VirtualFs;
use parser::{ParsedCommand, Pipeline};

pub fn execute_pipeline(
    pipeline: &Pipeline,
    fs: &mut VirtualFs,
    session: &mut Session,
) -> Result<String, VfsError> {
    let mut stdin = String::new();
    for (i, cmd) in pipeline.commands.iter().enumerate() {
        let is_last = i == pipeline.commands.len() - 1;
        let has_stdin = i > 0;
        stdin = execute_command(cmd, fs, session, if has_stdin { Some(&stdin) } else { None })?;
        if !is_last && stdin.is_empty() {
            break;
        }
    }
    Ok(stdin)
}

fn execute_command(
    cmd: &ParsedCommand,
    fs: &mut VirtualFs,
    session: &mut Session,
    stdin: Option<&str>,
) -> Result<String, VfsError> {
    match cmd.program.as_str() {
        "ls" => cmd_ls(fs, &cmd.args, session),
        "cd" => cmd_cd(fs, &cmd.args, session),
        "pwd" => Ok(format!("{}\n", fs.pwd())),
        "mkdir" => cmd_mkdir(fs, &cmd.args, session),
        "touch" => cmd_touch(fs, &cmd.args, session),
        "cat" => cmd_cat(fs, &cmd.args, stdin, session),
        "rm" => cmd_rm(fs, &cmd.args, session),
        "rmdir" => cmd_rmdir(fs, &cmd.args, session),
        "mv" => cmd_mv(fs, &cmd.args, session),
        "cp" => cmd_cp(fs, &cmd.args, session),
        "stat" => cmd_stat(fs, &cmd.args, session),
        "tree" => cmd_tree(fs, &cmd.args, session),
        "find" => cmd_find(fs, &cmd.args, session),
        "grep" => cmd_grep(fs, &cmd.args, stdin, session),
        "head" => cmd_head(&cmd.args, stdin),
        "tail" => cmd_tail(&cmd.args, stdin),
        "wc" => cmd_wc(&cmd.args, stdin),
        "chmod" => cmd_chmod(fs, &cmd.args, session),
        "chown" => cmd_chown(fs, &cmd.args, session),
        "ln" => cmd_ln(fs, &cmd.args, session),
        "echo" => cmd_echo(&cmd.args),
        "write" => cmd_write(fs, &cmd.args, stdin, session),
        // User management
        "adduser" => cmd_adduser(fs, &cmd.args, session),
        "addagent" => cmd_addagent(fs, &cmd.args, session),
        "deluser" => cmd_deluser(fs, &cmd.args, session),
        "addgroup" => cmd_addgroup(fs, &cmd.args, session),
        "delgroup" => cmd_delgroup(fs, &cmd.args, session),
        "usermod" => cmd_usermod(fs, &cmd.args, session),
        "groups" => cmd_groups(fs, &cmd.args, session),
        "whoami" => Ok(format!("{}\n", session.username)),
        "id" => cmd_id(fs, &cmd.args, session),
        "su" => cmd_su(fs, &cmd.args, session),
        "delegate" => cmd_delegate(fs, &cmd.args, session),
        "undelegate" => cmd_undelegate(session),
        "help" => cmd_help(),
        "clear" => Ok("\x1b[2J\x1b[H".to_string()),
        name => Err(VfsError::UnknownCommand {
            name: name.to_string(),
        }),
    }
}

// ───── Permission helpers ─────

/// Check that the session has the given access on an inode.
/// Respects delegation: both principal and delegate must have access.
fn require_access(
    fs: &VirtualFs,
    inode_id: u64,
    session: &Session,
    access: Access,
    path: &str,
) -> Result<(), VfsError> {
    let inode = fs.get_inode(inode_id)?;
    if !session.has_permission(inode, access) {
        return Err(VfsError::PermissionDenied {
            path: path.to_string(),
        });
    }
    Ok(())
}

/// Require admin (root or wheel) — respects delegation intersection.
/// Both the principal and delegate must be admin.
fn require_admin(session: &Session) -> Result<(), VfsError> {
    let principal_admin =
        session.uid == ROOT_UID || session.groups.contains(&WHEEL_GID);
    if !principal_admin {
        return Err(VfsError::PermissionDenied {
            path: "admin operation".to_string(),
        });
    }
    if let Some(ref delegate) = session.delegate {
        let delegate_admin =
            delegate.uid == ROOT_UID || delegate.groups.contains(&WHEEL_GID);
        if !delegate_admin {
            return Err(VfsError::PermissionDenied {
                path: "admin operation (delegate lacks admin)".to_string(),
            });
        }
    }
    Ok(())
}

// ───── File commands ─────

fn cmd_ls(fs: &VirtualFs, args: &[String], session: &Session) -> Result<String, VfsError> {
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

    let _ = all;

    // Permission: Read + Execute on the directory
    let dir_path = path.unwrap_or(".");
    let dir_id = fs.resolve_path_checked(dir_path, session)?;
    require_access(fs, dir_id, session, Access::Read, dir_path)?;
    require_access(fs, dir_id, session, Access::Execute, dir_path)?;

    let all_entries = fs.ls(path)?;
    // Filter: only show entries the user can read — respects delegation intersection
    let entries: Vec<_> = all_entries
        .into_iter()
        .filter(|e| session.has_permission_bits(e.mode, e.uid, e.gid, Access::Read))
        .collect();

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
            let owner = fs.registry.user_name(e.uid).unwrap_or("?");
            let group = fs.registry.group_name(e.gid).unwrap_or("?");
            output.push_str(&format!(
                "{kind}{perms} {owner:<8} {group:<8} {:>8} {time} {}{}\n",
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

fn cmd_cd(fs: &mut VirtualFs, args: &[String], session: &Session) -> Result<String, VfsError> {
    let path = args.first().map(|s| s.as_str()).unwrap_or("/");
    // Permission: Execute on target directory
    let target = fs.resolve_path_checked(path, session)?;
    require_access(fs, target, session, Access::Execute, path)?;
    fs.cd(path)?;
    Ok(String::new())
}

fn cmd_mkdir(fs: &mut VirtualFs, args: &[String], session: &Session) -> Result<String, VfsError> {
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
            // mkdir -p: intermediate dirs may not exist yet, so we can't resolve parent.
            // Permission check happens inside mkdir_p for each existing parent.
            fs.mkdir_p(path, session.effective_uid(), session.effective_gid())?;
        } else {
            // Permission: Write + Execute on parent
            let (parent_id, _) = fs.resolve_parent_checked(path, session)?;
            require_access(fs, parent_id, session, Access::Write, path)?;
            require_access(fs, parent_id, session, Access::Execute, path)?;
            fs.mkdir(path, session.effective_uid(), session.effective_gid())?;
        }
    }
    Ok(String::new())
}

fn cmd_touch(fs: &mut VirtualFs, args: &[String], session: &Session) -> Result<String, VfsError> {
    if args.is_empty() {
        return Err(VfsError::InvalidArgs {
            message: "touch: missing operand".to_string(),
        });
    }
    for arg in args {
        if fs.resolve_path(arg).is_ok() {
            // File exists — update timestamp, need Write on file
            let id = fs.resolve_path_checked(arg, session)?;
            require_access(fs, id, session, Access::Write, arg)?;
            fs.touch(arg, session.effective_uid(), session.effective_gid())?;
        } else {
            // New file — need Write + Execute on parent
            let (parent_id, _) = fs.resolve_parent_checked(arg, session)?;
            require_access(fs, parent_id, session, Access::Write, arg)?;
            require_access(fs, parent_id, session, Access::Execute, arg)?;
            fs.touch(arg, session.effective_uid(), session.effective_gid())?;
        }
    }
    Ok(String::new())
}

fn cmd_cat(
    fs: &VirtualFs,
    args: &[String],
    stdin: Option<&str>,
    session: &Session,
) -> Result<String, VfsError> {
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
        // Permission: Read on file
        let id = fs.resolve_path_checked(path, session)?;
        require_access(fs, id, session, Access::Read, path)?;
        let content = fs.cat(path)?;
        output.push_str(&String::from_utf8_lossy(content));
    }
    Ok(output)
}

fn cmd_rm(fs: &mut VirtualFs, args: &[String], session: &Session) -> Result<String, VfsError> {
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
        // Permission: Write + Execute on parent
        let (parent_id, _) = fs.resolve_parent_checked(path, session)?;
        require_access(fs, parent_id, session, Access::Write, path)?;
        require_access(fs, parent_id, session, Access::Execute, path)?;

        // Sticky bit check: only owner of file, owner of dir, or root can delete
        // With delegation: use effective identity
        let parent = fs.get_inode(parent_id)?;
        if has_sticky_bit(parent.mode) && !session.is_effectively_root() {
            let file_id = fs.resolve_path(path)?;
            let file_inode = fs.get_inode(file_id)?;
            let eff_uid = session.effective_uid();
            if file_inode.uid != eff_uid && parent.uid != eff_uid {
                return Err(VfsError::PermissionDenied {
                    path: path.to_string(),
                });
            }
        }

        if recursive {
            fs.rm_rf(path)?;
        } else {
            fs.rm(path)?;
        }
    }
    Ok(String::new())
}

fn cmd_rmdir(fs: &mut VirtualFs, args: &[String], session: &Session) -> Result<String, VfsError> {
    if args.is_empty() {
        return Err(VfsError::InvalidArgs {
            message: "rmdir: missing operand".to_string(),
        });
    }
    for path in args {
        let (parent_id, _) = fs.resolve_parent_checked(path, session)?;
        require_access(fs, parent_id, session, Access::Write, path)?;
        require_access(fs, parent_id, session, Access::Execute, path)?;
        fs.rmdir(path)?;
    }
    Ok(String::new())
}

fn cmd_mv(fs: &mut VirtualFs, args: &[String], session: &Session) -> Result<String, VfsError> {
    if args.len() < 2 {
        return Err(VfsError::InvalidArgs {
            message: "mv: need source and destination".to_string(),
        });
    }

    // Permission: W+X on source parent, W+X on dest parent
    let (src_parent, _) = fs.resolve_parent_checked(&args[0], session)?;
    require_access(fs, src_parent, session, Access::Write, &args[0])?;
    require_access(fs, src_parent, session, Access::Execute, &args[0])?;

    // Sticky bit on source parent — use effective identity for delegation
    let src_parent_inode = fs.get_inode(src_parent)?;
    if has_sticky_bit(src_parent_inode.mode) && !session.is_effectively_root() {
        let src_id = fs.resolve_path(&args[0])?;
        let src_inode = fs.get_inode(src_id)?;
        let eff_uid = session.effective_uid();
        if src_inode.uid != eff_uid && src_parent_inode.uid != eff_uid {
            return Err(VfsError::PermissionDenied {
                path: args[0].to_string(),
            });
        }
    }

    // Check destination parent
    if let Ok((dst_parent, _)) = fs.resolve_parent_checked(&args[1], session) {
        require_access(fs, dst_parent, session, Access::Write, &args[1])?;
        require_access(fs, dst_parent, session, Access::Execute, &args[1])?;
    }

    fs.mv(&args[0], &args[1])?;
    Ok(String::new())
}

fn cmd_cp(fs: &mut VirtualFs, args: &[String], session: &Session) -> Result<String, VfsError> {
    if args.len() < 2 {
        return Err(VfsError::InvalidArgs {
            message: "cp: need source and destination".to_string(),
        });
    }

    // Permission: Read on source
    let src_id = fs.resolve_path_checked(&args[0], session)?;
    require_access(fs, src_id, session, Access::Read, &args[0])?;

    // Permission: Write + Execute on destination parent
    if let Ok((dst_parent, _)) = fs.resolve_parent_checked(&args[1], session) {
        require_access(fs, dst_parent, session, Access::Write, &args[1])?;
        require_access(fs, dst_parent, session, Access::Execute, &args[1])?;
    }

    fs.cp(&args[0], &args[1], session.effective_uid(), session.effective_gid())?;
    Ok(String::new())
}

fn cmd_stat(fs: &VirtualFs, args: &[String], session: &Session) -> Result<String, VfsError> {
    if args.is_empty() {
        return Err(VfsError::InvalidArgs {
            message: "stat: missing operand".to_string(),
        });
    }
    let _id = fs.resolve_path_checked(&args[0], session)?;
    let info = fs.stat(&args[0])?;
    let owner = fs.registry.user_name(info.uid).unwrap_or("?");
    let group = fs.registry.group_name(info.gid).unwrap_or("?");
    Ok(format!(
        "  File: {}\n  Size: {}\n Blocks: {}\n IO Block: {}\n  Type: {}\n Inode: {}\n Links: {}\n  Mode: {:04o}\n   Uid: {} ({})\n   Gid: {} ({})\nCreated: {}.{}\nAccessed: {}.{}\nModified: {}.{}\n Changed: {}.{}\n",
        args[0], info.size, info.blocks, info.block_size, info.kind, info.inode_id, info.nlink, info.mode,
        info.uid, owner, info.gid, group,
        format_time(info.created), info.created_nanos,
        format_time(info.accessed), info.accessed_nanos,
        format_time(info.modified), info.modified_nanos,
        format_time(info.changed), info.changed_nanos,
    ))
}

fn cmd_tree(fs: &VirtualFs, args: &[String], session: &Session) -> Result<String, VfsError> {
    let path = args.first().map(|s| s.as_str());
    if let Some(p) = path {
        let id = fs.resolve_path_checked(p, session)?;
        require_access(fs, id, session, Access::Read, p)?;
        require_access(fs, id, session, Access::Execute, p)?;
    }
    fs.tree(path, "", Some(session))
}

fn cmd_find(fs: &VirtualFs, args: &[String], session: &Session) -> Result<String, VfsError> {
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

    if let Some(p) = path {
        let id = fs.resolve_path_checked(p, session)?;
        require_access(fs, id, session, Access::Read, p)?;
        require_access(fs, id, session, Access::Execute, p)?;
    }

    let results = fs.find(path, pattern, Some(session))?;
    Ok(results.join("\n") + if results.is_empty() { "" } else { "\n" })
}

fn cmd_grep(
    fs: &VirtualFs,
    args: &[String],
    stdin: Option<&str>,
    session: &Session,
) -> Result<String, VfsError> {
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

    // Permission: Read on file/dir
    if let Some(p) = path {
        let id = fs.resolve_path_checked(p, session)?;
        require_access(fs, id, session, Access::Read, p)?;
    }

    let results = fs.grep(pattern, path, recursive, Some(session))?;
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

fn cmd_chmod(fs: &mut VirtualFs, args: &[String], session: &Session) -> Result<String, VfsError> {
    if args.len() < 2 {
        return Err(VfsError::InvalidArgs {
            message: "chmod: need mode and file".to_string(),
        });
    }
    let mode = u16::from_str_radix(&args[0], 8).map_err(|_| VfsError::InvalidArgs {
        message: format!("chmod: invalid mode: {}", args[0]),
    })?;

    // Permission: must be owner or root (delegation: both must qualify)
    let id = fs.resolve_path_checked(&args[1], session)?;
    let inode = fs.get_inode(id)?;
    if !session.is_effective_owner(inode.uid) {
        return Err(VfsError::PermissionDenied {
            path: args[1].to_string(),
        });
    }

    fs.chmod(&args[1], mode)?;
    Ok(String::new())
}

fn cmd_chown(fs: &mut VirtualFs, args: &[String], session: &Session) -> Result<String, VfsError> {
    if args.len() < 2 {
        return Err(VfsError::InvalidArgs {
            message: "chown: need owner[:group] and file".to_string(),
        });
    }

    let spec = &args[0];
    let path = &args[1];

    let (user_str, group_str) = if let Some(pos) = spec.find(':') {
        (Some(&spec[..pos]), Some(&spec[pos + 1..]))
    } else {
        (Some(spec.as_str()), None)
    };

    let id = fs.resolve_path_checked(path, session)?;
    let inode = fs.get_inode(id)?;

    // Changing uid: root only (delegation: must be effectively root)
    // Changing gid: root or owner (if member of target group)
    if let Some(user) = user_str {
        if !user.is_empty() {
            if !session.is_effectively_root() {
                return Err(VfsError::PermissionDenied {
                    path: path.to_string(),
                });
            }
            let new_uid = fs.registry.lookup_uid(user).ok_or_else(|| VfsError::AuthError {
                message: format!("no such user: {user}"),
            })?;
            let current_gid = inode.gid;
            fs.chown(path, new_uid, current_gid)?;
        }
    }

    if let Some(group) = group_str {
        if !group.is_empty() {
            let new_gid = fs.registry.lookup_gid(group).ok_or_else(|| VfsError::AuthError {
                message: format!("no such group: {group}"),
            })?;
            // Owner can change group if they're a member of the target group
            // Delegation: effective identity must be owner and in target group
            let inode = fs.get_inode(fs.resolve_path(path)?)?;
            if !session.is_effectively_root() {
                if !session.is_effective_owner(inode.uid) {
                    return Err(VfsError::PermissionDenied {
                        path: path.to_string(),
                    });
                }
                // Both principal and delegate must be in target group
                if !session.groups.contains(&new_gid) {
                    return Err(VfsError::PermissionDenied {
                        path: path.to_string(),
                    });
                }
                if let Some(ref delegate) = session.delegate {
                    if !delegate.groups.contains(&new_gid) {
                        return Err(VfsError::PermissionDenied {
                            path: path.to_string(),
                        });
                    }
                }
            }
            fs.chown(path, inode.uid, new_gid)?;
        }
    }

    Ok(String::new())
}

fn cmd_ln(fs: &mut VirtualFs, args: &[String], session: &Session) -> Result<String, VfsError> {
    let mut symlink = false;
    let mut targets = Vec::new();

    for arg in args {
        if arg == "-s" {
            symlink = true;
        } else {
            targets.push(arg.as_str());
        }
    }

    if targets.len() < 2 {
        return Err(VfsError::InvalidArgs {
            message: "ln: need target and link name".to_string(),
        });
    }

    // Permission: Write + Execute on link's parent
    let (parent_id, _) = fs.resolve_parent_checked(targets[1], session)?;
    require_access(fs, parent_id, session, Access::Write, targets[1])?;
    require_access(fs, parent_id, session, Access::Execute, targets[1])?;

    if symlink {
        fs.ln_s(targets[0], targets[1], session.effective_uid(), session.effective_gid())?;
    } else {
        let target_id = fs.resolve_path_checked(targets[0], session)?;
        require_access(fs, target_id, session, Access::Read, targets[0])?;
        fs.link(targets[0], targets[1])?;
    }
    Ok(String::new())
}

fn cmd_echo(args: &[String]) -> Result<String, VfsError> {
    Ok(args.join(" ") + "\n")
}

fn cmd_write(
    fs: &mut VirtualFs,
    args: &[String],
    stdin: Option<&str>,
    session: &Session,
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
        let (parent_id, _) = fs.resolve_parent_checked(&args[0], session)?;
        require_access(fs, parent_id, session, Access::Write, &args[0])?;
        fs.touch(&args[0], session.effective_uid(), session.effective_gid())?;
    } else {
        // File exists — need Write on file
        let id = fs.resolve_path_checked(&args[0], session)?;
        require_access(fs, id, session, Access::Write, &args[0])?;
    }
    fs.write_file(&args[0], content.into_bytes())?;
    Ok(String::new())
}

// ───── User management commands ─────

fn cmd_adduser(
    fs: &mut VirtualFs,
    args: &[String],
    _session: &Session,
) -> Result<String, VfsError> {
    require_admin(_session)?;
    if args.is_empty() {
        return Err(VfsError::InvalidArgs {
            message: "adduser: missing username".to_string(),
        });
    }
    let name = &args[0];
    let (uid, _) = fs.registry.add_user(name, false)?;
    let gid = fs.registry.get_user(uid).map(|u| u.groups.first().copied().unwrap_or(0)).unwrap_or(0);

    let mut output = format!("User '{name}' created (uid={uid})\n");

    let _ = fs.mkdir_p("/home", ROOT_UID, ROOT_GID);
    let home_path = format!("/home/{name}");
    if fs.mkdir(&home_path, uid, gid).is_ok() {
        output.push_str(&format!("Home directory: /home/{name}\n"));
    }

    Ok(output)
}

fn cmd_addagent(
    fs: &mut VirtualFs,
    args: &[String],
    _session: &Session,
) -> Result<String, VfsError> {
    require_admin(_session)?;
    if args.is_empty() {
        return Err(VfsError::InvalidArgs {
            message: "addagent: missing agent name".to_string(),
        });
    }
    let (uid, raw_token) = fs.registry.add_user(&args[0], true)?;
    let token = raw_token.unwrap();
    Ok(format!(
        "Agent '{}' created (uid={uid})\nAPI token (save this — shown only once):\n  {token}\n",
        args[0]
    ))
}

fn cmd_deluser(
    fs: &mut VirtualFs,
    args: &[String],
    _session: &Session,
) -> Result<String, VfsError> {
    if !_session.is_effectively_root() {
        return Err(VfsError::PermissionDenied {
            path: "deluser: root only".to_string(),
        });
    }
    if args.is_empty() {
        return Err(VfsError::InvalidArgs {
            message: "deluser: missing username".to_string(),
        });
    }
    fs.registry.del_user(&args[0])?;
    Ok(format!("User '{}' deleted\n", args[0]))
}

fn cmd_addgroup(
    fs: &mut VirtualFs,
    args: &[String],
    _session: &Session,
) -> Result<String, VfsError> {
    require_admin(_session)?;
    if args.is_empty() {
        return Err(VfsError::InvalidArgs {
            message: "addgroup: missing group name".to_string(),
        });
    }
    let gid = fs.registry.add_group(&args[0])?;
    Ok(format!("Group '{}' created (gid={gid})\n", args[0]))
}

fn cmd_delgroup(
    fs: &mut VirtualFs,
    args: &[String],
    _session: &Session,
) -> Result<String, VfsError> {
    if !_session.is_effectively_root() {
        return Err(VfsError::PermissionDenied {
            path: "delgroup: root only".to_string(),
        });
    }
    if args.is_empty() {
        return Err(VfsError::InvalidArgs {
            message: "delgroup: missing group name".to_string(),
        });
    }
    fs.registry.del_group(&args[0])?;
    Ok(format!("Group '{}' deleted\n", args[0]))
}

fn cmd_usermod(
    fs: &mut VirtualFs,
    args: &[String],
    _session: &Session,
) -> Result<String, VfsError> {
    require_admin(_session)?;

    // usermod -aG <group> <user>  or  usermod -rG <group> <user>
    if args.len() < 3 {
        return Err(VfsError::InvalidArgs {
            message: "usermod: usage: usermod -aG <group> <user> | usermod -rG <group> <user>"
                .to_string(),
        });
    }

    match args[0].as_str() {
        "-aG" => {
            fs.registry.usermod_add_group(&args[2], &args[1])?;
            Ok(format!("Added '{}' to group '{}'\n", args[2], args[1]))
        }
        "-rG" => {
            fs.registry.usermod_remove_group(&args[2], &args[1])?;
            Ok(format!(
                "Removed '{}' from group '{}'\n",
                args[2], args[1]
            ))
        }
        _ => Err(VfsError::InvalidArgs {
            message: "usermod: usage: usermod -aG <group> <user> | usermod -rG <group> <user>"
                .to_string(),
        }),
    }
}

fn cmd_groups(
    fs: &VirtualFs,
    args: &[String],
    session: &Session,
) -> Result<String, VfsError> {
    let username = if args.is_empty() {
        &session.username
    } else {
        // Only root or the user themselves can see others' groups
        if session.uid != ROOT_UID && args[0] != session.username {
            return Err(VfsError::PermissionDenied {
                path: "groups".to_string(),
            });
        }
        &args[0]
    };

    let uid = fs
        .registry
        .lookup_uid(username)
        .ok_or_else(|| VfsError::AuthError {
            message: format!("no such user: {username}"),
        })?;
    let user = fs.registry.get_user(uid).unwrap();

    let group_names: Vec<&str> = user
        .groups
        .iter()
        .filter_map(|&gid| fs.registry.group_name(gid))
        .collect();
    Ok(format!("{}\n", group_names.join(" ")))
}

fn cmd_id(
    fs: &VirtualFs,
    args: &[String],
    session: &Session,
) -> Result<String, VfsError> {
    let username = if args.is_empty() {
        &session.username
    } else {
        &args[0]
    };

    let uid = fs
        .registry
        .lookup_uid(username)
        .ok_or_else(|| VfsError::AuthError {
            message: format!("no such user: {username}"),
        })?;
    let user = fs.registry.get_user(uid).unwrap();

    let primary_gid = user.groups.first().copied().unwrap_or(0);
    let primary_group = fs.registry.group_name(primary_gid).unwrap_or("?");

    let groups_str: Vec<String> = user
        .groups
        .iter()
        .filter_map(|&gid| {
            fs.registry
                .group_name(gid)
                .map(|name| format!("{gid}({name})"))
        })
        .collect();

    Ok(format!(
        "uid={uid}({}) gid={primary_gid}({primary_group}) groups={}\n",
        user.name,
        groups_str.join(",")
    ))
}

fn cmd_su(
    fs: &VirtualFs,
    args: &[String],
    session: &mut Session,
) -> Result<String, VfsError> {
    if args.is_empty() {
        return Err(VfsError::InvalidArgs {
            message: "su: missing username".to_string(),
        });
    }

    let target = &args[0];

    // Root can su to anyone; wheel members can su to anyone
    // Delegation: the principal (not delegate) must be root or wheel
    if session.uid != ROOT_UID && !session.groups.contains(&WHEEL_GID) {
        return Err(VfsError::PermissionDenied {
            path: format!("su: must be root or wheel member to switch user"),
        });
    }

    let uid = fs
        .registry
        .lookup_uid(target)
        .ok_or_else(|| VfsError::AuthError {
            message: format!("no such user: {target}"),
        })?;
    let user = fs.registry.get_user(uid).unwrap();

    session.uid = user.uid;
    session.gid = user.groups.first().copied().unwrap_or(0);
    session.groups = user.groups.clone();
    session.username = user.name.clone();

    Ok(format!("Switched to user '{}'\n", user.name))
}

fn cmd_delegate(
    fs: &VirtualFs,
    args: &[String],
    session: &mut Session,
) -> Result<String, VfsError> {
    if args.is_empty() {
        return Err(VfsError::InvalidArgs {
            message: "delegate: usage: delegate <user> or delegate :<group>".to_string(),
        });
    }

    // Only agents or admins can delegate
    let is_agent = fs
        .registry
        .get_user(session.uid)
        .map(|u| u.is_agent)
        .unwrap_or(false);
    if !is_agent && session.uid != ROOT_UID && !session.groups.contains(&WHEEL_GID) {
        return Err(VfsError::PermissionDenied {
            path: "delegate: only agents or admins can delegate".to_string(),
        });
    }

    let target = &args[0];

    if let Some(group_name) = target.strip_prefix(':') {
        // Delegate for a group: permissions limited to what a generic member of that group can do
        let gid =
            fs.registry
                .lookup_gid(group_name)
                .ok_or_else(|| VfsError::AuthError {
                    message: format!("no such group: {group_name}"),
                })?;
        session.delegate = Some(crate::auth::session::DelegateContext {
            // Use a synthetic uid that won't match any file owner — forces group/other bits only
            uid: u32::MAX,
            gid,
            groups: vec![gid],
            username: format!(":{group_name}"),
        });
        Ok(format!(
            "Now acting on behalf of group '{group_name}' (effective: intersection)\n"
        ))
    } else {
        // Delegate for a specific user
        let uid =
            fs.registry
                .lookup_uid(target)
                .ok_or_else(|| VfsError::AuthError {
                    message: format!("no such user: {target}"),
                })?;
        let user = fs.registry.get_user(uid).unwrap();
        session.delegate = Some(crate::auth::session::DelegateContext {
            uid: user.uid,
            gid: user.groups.first().copied().unwrap_or(0),
            groups: user.groups.clone(),
            username: user.name.clone(),
        });
        Ok(format!(
            "Now acting on behalf of '{}' (effective: intersection)\n",
            user.name
        ))
    }
}

fn cmd_undelegate(session: &mut Session) -> Result<String, VfsError> {
    if session.delegate.is_none() {
        return Ok("No active delegation\n".to_string());
    }
    let name = session
        .delegate
        .as_ref()
        .map(|d| d.username.clone())
        .unwrap_or_default();
    session.delegate = None;
    Ok(format!("Delegation for '{name}' ended\n"))
}

// ───── Help ─────

fn cmd_help() -> Result<String, VfsError> {
    Ok(r#"markdownfs — Markdown Virtual File System

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
  chmod <mode> <path>      Change permissions (owner or root)
  chown <user:group> <path>  Change ownership (root or owner)
  ln -s <target> <link>    Create symbolic link
  echo <text>              Print text
  edit <file.md>           Edit file (multi-line input, auto-commits)

User management:
  adduser <name>           Create user (admin)
  addagent <name>          Create agent with API token (admin)
  deluser <name>           Delete user (root only)
  addgroup <name>          Create group (admin)
  delgroup <name>          Delete group (root only)
  usermod -aG <grp> <user> Add user to group (admin)
  usermod -rG <grp> <user> Remove user from group (admin)
  groups [user]            Show group memberships
  whoami                   Show current user
  id [user]                Show user identity
  su <user>                Switch user (root or wheel)

VCS commands:
  commit <message>         Commit current state
  log                      Show commit history
  revert <hash>            Revert to a commit
  status                   Show modified files

Pipes:  grep "TODO" notes/ | head -5 | wc -l
"#
    .to_string())
}

// ───── Formatting ─────

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
