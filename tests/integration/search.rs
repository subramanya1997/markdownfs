use super::*;

#[test]
fn test_find() {
    let mut fs = VirtualFs::new();
    exec("mkdir -p a/b", &mut fs);
    exec("cd a", &mut fs);
    exec("touch x.md", &mut fs);
    exec("cd b", &mut fs);
    exec("touch y.md", &mut fs);
    exec("cd /", &mut fs);
    let output = exec("find . -name *.md", &mut fs);
    assert!(output.contains("x.md"));
    assert!(output.contains("y.md"));
}

#[test]
fn test_find_no_pattern() {
    let mut fs = VirtualFs::new();
    exec("mkdir dir", &mut fs);
    exec("cd dir", &mut fs);
    exec("touch a.md", &mut fs);
    exec("cd /", &mut fs);
    let output = exec("find .", &mut fs);
    assert!(output.contains("dir"));
    assert!(output.contains("a.md"));
}

#[test]
fn test_find_specific_name() {
    let mut fs = VirtualFs::new();
    exec("mkdir -p src/utils", &mut fs);
    exec("cd src", &mut fs);
    exec("touch main.md", &mut fs);
    exec("cd utils", &mut fs);
    exec("touch helpers.md", &mut fs);
    exec("cd /", &mut fs);
    let output = exec("find . -name helpers.md", &mut fs);
    assert!(output.contains("helpers.md"));
    assert!(!output.contains("main.md"));
}

#[test]
fn test_grep() {
    let mut fs = VirtualFs::new();
    exec("touch notes.md", &mut fs);
    exec("write notes.md TODO: fix this\nDONE: that\nTODO: another", &mut fs);
    let output = exec("grep TODO notes.md", &mut fs);
    assert!(output.contains("TODO: fix this"));
    assert!(output.contains("TODO: another"));
    assert!(!output.contains("DONE"));
}

#[test]
fn test_grep_recursive() {
    let mut fs = VirtualFs::new();
    exec("mkdir -p a/b", &mut fs);
    exec("touch a.md", &mut fs);
    exec("write a.md ERROR: top level", &mut fs);
    exec("cd a", &mut fs);
    exec("touch mid.md", &mut fs);
    exec("write mid.md INFO: middle level", &mut fs);
    exec("cd b", &mut fs);
    exec("touch deep.md", &mut fs);
    exec("write deep.md ERROR: deep level", &mut fs);
    exec("cd /", &mut fs);
    let output = exec("grep -r ERROR .", &mut fs);
    assert!(output.contains("a.md"));
    assert!(output.contains("deep.md"));
    assert!(!output.contains("mid.md"));
}

#[test]
fn test_grep_no_match() {
    let mut fs = VirtualFs::new();
    exec("touch file.md", &mut fs);
    exec("write file.md hello world", &mut fs);
    let output = exec("grep NONEXISTENT file.md", &mut fs);
    assert!(output.trim().is_empty());
}

#[test]
fn test_grep_regex() {
    let mut fs = VirtualFs::new();
    exec("touch data.md", &mut fs);
    exec("write data.md alpha 123\nbeta abc\ngamma 456", &mut fs);
    let output = exec("grep [0-9]{3} data.md", &mut fs);
    assert!(output.contains("alpha 123"));
    assert!(output.contains("gamma 456"));
    assert!(!output.contains("beta abc"));
}
