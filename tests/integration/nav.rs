use super::*;

#[test]
fn test_cd_and_pwd() {
    let mut fs = VirtualFs::new();
    exec("mkdir project", &mut fs);
    exec("cd project", &mut fs);
    let pwd = exec("pwd", &mut fs);
    assert_eq!(pwd.trim(), "/project");
    exec("cd /", &mut fs);
    assert_eq!(exec("pwd", &mut fs).trim(), "/");
}

#[test]
fn test_nested_dirs() {
    let mut fs = VirtualFs::new();
    exec("mkdir -p a/b/c", &mut fs);
    exec("cd a/b/c", &mut fs);
    let pwd = exec("pwd", &mut fs);
    assert_eq!(pwd.trim(), "/a/b/c");
}

#[test]
fn test_cd_dotdot() {
    let mut fs = VirtualFs::new();
    exec("mkdir -p a/b/c", &mut fs);
    exec("cd a/b/c", &mut fs);
    exec("cd ..", &mut fs);
    assert_eq!(exec("pwd", &mut fs).trim(), "/a/b");
    exec("cd ..", &mut fs);
    assert_eq!(exec("pwd", &mut fs).trim(), "/a");
    exec("cd ..", &mut fs);
    assert_eq!(exec("pwd", &mut fs).trim(), "/");
}

#[test]
fn test_cd_dotdot_at_root() {
    let mut fs = VirtualFs::new();
    exec("cd ..", &mut fs);
    assert_eq!(exec("pwd", &mut fs).trim(), "/");
}

#[test]
fn test_cd_absolute_path() {
    let mut fs = VirtualFs::new();
    exec("mkdir -p a/b/c", &mut fs);
    exec("cd a/b/c", &mut fs);
    exec("cd /a", &mut fs);
    assert_eq!(exec("pwd", &mut fs).trim(), "/a");
}

#[test]
fn test_cd_nonexistent_dir_fails() {
    let mut fs = VirtualFs::new();
    let err = exec_err("cd nowhere", &mut fs);
    assert!(err.contains("no such file"));
}

#[test]
fn test_cd_to_file_fails() {
    let mut fs = VirtualFs::new();
    exec("touch file.md", &mut fs);
    let err = exec_err("cd file.md", &mut fs);
    assert!(err.contains("not a directory"));
}

#[test]
fn test_ls_specific_dir() {
    let mut fs = VirtualFs::new();
    exec("mkdir docs", &mut fs);
    exec("cd docs", &mut fs);
    exec("touch a.md", &mut fs);
    exec("touch b.md", &mut fs);
    exec("cd /", &mut fs);
    let output = exec("ls docs", &mut fs);
    assert!(output.contains("a.md"));
    assert!(output.contains("b.md"));
}
