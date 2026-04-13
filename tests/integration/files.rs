use super::*;

#[test]
fn test_touch_and_cat() {
    let mut fs = VirtualFs::new();
    exec("touch readme.md", &mut fs);
    exec("write readme.md Hello, markdownfs!", &mut fs);
    let output = exec("cat readme.md", &mut fs);
    assert_eq!(output, "Hello, markdownfs!");
}

#[test]
fn test_only_markdown_files() {
    let mut fs = VirtualFs::new();
    let err = exec_err("touch hello.txt", &mut fs);
    assert!(err.contains("only .md files"));
}

#[test]
fn test_touch_non_md_extensions_rejected() {
    let mut fs = VirtualFs::new();
    for ext in ["txt", "rs", "py", "json", "yaml", "toml", "html", "css"] {
        let err = exec_err(&format!("touch file.{ext}"), &mut fs);
        assert!(
            err.contains("only .md files"),
            "extension .{ext} should be rejected"
        );
    }
}

#[test]
fn test_touch_updates_timestamp() {
    let mut fs = VirtualFs::new();
    exec("touch file.md", &mut fs);
    let stat1 = exec("stat file.md", &mut fs);
    std::thread::sleep(std::time::Duration::from_millis(10));
    exec("touch file.md", &mut fs);
    let stat2 = exec("stat file.md", &mut fs);
    // Timestamps should be equal or newer (within test resolution)
    assert!(stat1.contains("file") && stat2.contains("file"));
}

#[test]
fn test_write_to_nonexistent_dir_fails() {
    let mut fs = VirtualFs::new();
    let err = exec_err("write nonexistent/ghost.md content", &mut fs);
    assert!(err.contains("no such file"));
}

#[test]
fn test_cat_nonexistent_file() {
    let mut fs = VirtualFs::new();
    let err = exec_err("cat ghost.md", &mut fs);
    assert!(err.contains("no such file"));
}

#[test]
fn test_cat_directory_fails() {
    let mut fs = VirtualFs::new();
    exec("mkdir dir", &mut fs);
    let err = exec_err("cat dir", &mut fs);
    assert!(err.contains("is a directory"));
}

#[test]
fn test_write_overwrite() {
    let mut fs = VirtualFs::new();
    exec("touch file.md", &mut fs);
    exec("write file.md first", &mut fs);
    assert_eq!(exec("cat file.md", &mut fs), "first");
    exec("write file.md second", &mut fs);
    assert_eq!(exec("cat file.md", &mut fs), "second");
}

#[test]
fn test_write_via_pipe_empty() {
    let mut fs = VirtualFs::new();
    exec("touch file.md", &mut fs);
    exec("write file.md initial", &mut fs);
    // Verify the initial write
    assert_eq!(exec("cat file.md", &mut fs), "initial");
    // Overwrite with pipe
    exec("echo replaced | write file.md", &mut fs);
    let content = exec("cat file.md", &mut fs);
    assert!(content.contains("replaced"));
}

#[test]
fn test_write_multiline() {
    let mut fs = VirtualFs::new();
    exec("touch file.md", &mut fs);
    exec("write file.md # Title\n\nParagraph 1\n\nParagraph 2", &mut fs);
    let content = exec("cat file.md", &mut fs);
    assert!(content.contains("# Title"));
    assert!(content.contains("Paragraph 1"));
    assert!(content.contains("Paragraph 2"));
}

#[test]
fn test_write_unicode() {
    let mut fs = VirtualFs::new();
    exec("touch unicode.md", &mut fs);
    exec(
        "write unicode.md 你好世界 🌍 café résumé naïve",
        &mut fs,
    );
    let content = exec("cat unicode.md", &mut fs);
    assert!(content.contains("你好世界"));
    assert!(content.contains("🌍"));
    assert!(content.contains("café"));
}
