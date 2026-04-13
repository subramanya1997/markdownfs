use super::*;

#[test]
fn test_pipes() {
    let mut fs = VirtualFs::new();
    let output = exec("echo hello world | wc -w", &mut fs);
    assert_eq!(output.trim(), "2");
}

#[test]
fn test_head_tail() {
    let mut fs = VirtualFs::new();
    exec("touch data.md", &mut fs);
    exec("write data.md 1\n2\n3\n4\n5", &mut fs);
    let head = exec("cat data.md | head -2", &mut fs);
    assert_eq!(head.trim(), "1\n2");
    let tail = exec("cat data.md | tail -2", &mut fs);
    assert_eq!(tail.trim(), "4\n5");
}

#[test]
fn test_grep_pipe() {
    let mut fs = VirtualFs::new();
    exec("touch log.md", &mut fs);
    exec(
        "write log.md ERROR: disk full\nINFO: started\nERROR: timeout\nINFO: done",
        &mut fs,
    );
    let output = exec("cat log.md | grep ERROR | wc -l", &mut fs);
    assert_eq!(output.trim(), "2");
}

#[test]
fn test_echo_pipe_write() {
    let mut fs = VirtualFs::new();
    exec("touch out.md", &mut fs);
    exec("echo hello from pipe | write out.md", &mut fs);
    let content = exec("cat out.md", &mut fs);
    assert_eq!(content.trim(), "hello from pipe");
}

#[test]
fn test_pipe_head_tail_combined() {
    let mut fs = VirtualFs::new();
    exec("touch nums.md", &mut fs);
    let lines: Vec<String> = (1..=20).map(|i| format!("{i}")).collect();
    exec(
        &format!("write nums.md {}", lines.join("\n")),
        &mut fs,
    );
    let output = exec("cat nums.md | head -10 | tail -3", &mut fs);
    assert!(output.contains("8"));
    assert!(output.contains("9"));
    assert!(output.contains("10"));
}

#[test]
fn test_wc_lines() {
    let mut fs = VirtualFs::new();
    exec("touch file.md", &mut fs);
    exec("write file.md a\nb\nc\nd\ne", &mut fs);
    let output = exec("cat file.md | wc -l", &mut fs);
    assert_eq!(output.trim(), "5");
}

#[test]
fn test_wc_words() {
    let mut fs = VirtualFs::new();
    let output = exec("echo one two three four five | wc -w", &mut fs);
    assert_eq!(output.trim(), "5");
}

#[test]
fn test_echo() {
    let mut fs = VirtualFs::new();
    let output = exec("echo hello world", &mut fs);
    assert_eq!(output.trim(), "hello world");
}

#[test]
fn test_echo_empty() {
    let mut fs = VirtualFs::new();
    let output = exec("echo", &mut fs);
    assert!(output.trim().is_empty());
}

#[test]
fn test_help() {
    let mut fs = VirtualFs::new();
    let output = exec("help", &mut fs);
    assert!(output.contains("ls"));
    assert!(output.contains("mkdir"));
    assert!(output.contains("commit"));
}
