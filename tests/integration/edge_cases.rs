use super::*;

#[test]
fn test_unknown_command() {
    let mut fs = VirtualFs::new();
    let err = exec_err("foobar", &mut fs);
    assert!(err.contains("unknown command"));
}

#[test]
fn test_empty_ls_root() {
    let fs = VirtualFs::new();
    let entries = fs.ls(None).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn test_ls_nonexistent_path() {
    let mut fs = VirtualFs::new();
    let err = exec_err("ls nonexistent", &mut fs);
    assert!(err.contains("no such file"));
}

#[test]
fn test_touch_in_nonexistent_parent() {
    let mut fs = VirtualFs::new();
    let err = exec_err("touch nonexistent/file.md", &mut fs);
    assert!(err.contains("no such file"));
}

#[test]
fn test_mkdir_in_nonexistent_parent() {
    let mut fs = VirtualFs::new();
    let err = exec_err("mkdir nonexistent/child", &mut fs);
    assert!(err.contains("no such file"));
}

#[test]
fn test_mv_nonexistent_src_fails() {
    let mut fs = VirtualFs::new();
    let err = exec_err("mv ghost.md dest.md", &mut fs);
    assert!(err.contains("no such file"));
}

#[test]
fn test_many_files_same_directory() {
    let mut fs = VirtualFs::new();
    for i in 0..500 {
        fs.touch(&format!("file_{i:04}.md"), 0, 0).unwrap();
    }
    let entries = fs.ls(None).unwrap();
    assert_eq!(entries.len(), 500);
    // BTreeMap should keep them sorted
    assert!(entries[0].name < entries[499].name);
}

#[test]
fn test_large_file_content() {
    let mut fs = VirtualFs::new();
    let content: String = (0..10_000)
        .map(|i| format!("line {i}: Lorem ipsum dolor sit amet\n"))
        .collect();
    exec("touch big.md", &mut fs);
    fs.write_file("big.md", content.as_bytes().to_vec())
        .unwrap();
    let read_back = fs.cat("big.md").unwrap();
    assert_eq!(read_back.len(), content.len());
}

#[test]
fn test_deeply_nested_path() {
    let mut fs = VirtualFs::new();
    let mut path = String::new();
    for i in 0..30 {
        if i > 0 {
            path.push('/');
        }
        path.push_str(&format!("d{i}"));
    }
    fs.mkdir_p(&path, 0, 0).unwrap();
    let file_path = format!("{path}/deep.md");
    fs.touch(&file_path, 0, 0).unwrap();
    fs.write_file(&file_path, b"deep content".to_vec())
        .unwrap();
    assert_eq!(fs.cat(&file_path).unwrap(), b"deep content");
}

#[test]
fn test_stress_create_and_delete_cycle() {
    let mut fs = VirtualFs::new();
    for cycle in 0..20 {
        for i in 0..50 {
            let path = format!("cycle_{cycle}_file_{i}.md");
            fs.touch(&path, 0, 0).unwrap();
        }
        for i in 0..50 {
            let path = format!("cycle_{cycle}_file_{i}.md");
            fs.rm(&path).unwrap();
        }
    }
    let entries = fs.ls(None).unwrap();
    assert!(entries.is_empty(), "all files should be deleted");
}

#[test]
fn test_stress_rapid_commits() {
    let mut fs = VirtualFs::new();
    let mut vcs = markdownfs::vcs::Vcs::new();

    exec("touch file.md", &mut fs);
    for i in 0..50 {
        exec(&format!("write file.md version {i}"), &mut fs);
        vcs.commit(&fs, &format!("commit {i}"), "root").unwrap();
    }

    assert_eq!(vcs.log().len(), 50);
    assert_eq!(exec("cat file.md", &mut fs), "version 49");
}

#[test]
fn test_stress_deep_pipe_chain() {
    let mut fs = VirtualFs::new();
    exec("touch log.md", &mut fs);
    let mut content = String::new();
    for i in 0..1000 {
        content.push_str(&format!("ERROR line {i}\n"));
    }
    fs.write_file("log.md", content.into_bytes()).unwrap();
    let output = exec("cat log.md | grep ERROR | head -5 | wc -l", &mut fs);
    assert_eq!(output.trim(), "5");
}
