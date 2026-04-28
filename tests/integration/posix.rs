use markdownfs::auth::session::Session;
use markdownfs::posix::{PosixFs, PosixSetAttr};

use super::*;

#[test]
fn test_posix_mode_allows_non_markdown_regular_files() {
    let mut fs = VirtualFs::new_posix();
    let root = Session::root();
    let mut posix = PosixFs::new(&mut fs, &root);

    let _fh = posix.create("/notes.txt", 0o644).unwrap();
    posix.write("/notes.txt", 0, b"plain text").unwrap();

    let stat = posix.getattr("/notes.txt").unwrap();
    assert_eq!(stat.kind, "file");
    assert_eq!(stat.size, 10);
}

#[test]
fn test_posix_offset_io_and_truncate() {
    let mut fs = VirtualFs::new_posix();
    let root = Session::root();
    let mut posix = PosixFs::new(&mut fs, &root);

    let _fh = posix.create("/blob.bin", 0o644).unwrap();
    posix.write("/blob.bin", 0, b"hello world").unwrap();
    posix.write("/blob.bin", 6, b"POSIX").unwrap();

    assert_eq!(posix.read("/blob.bin", 0, 11).unwrap(), b"hello POSIX");

    posix.truncate("/blob.bin", 5).unwrap();
    assert_eq!(posix.read("/blob.bin", 0, 16).unwrap(), b"hello");

    let stat = posix.getattr("/blob.bin").unwrap();
    assert_eq!(stat.size, 5);
}

#[test]
fn test_posix_hard_links_update_nlink() {
    let mut fs = VirtualFs::new_posix();
    let root = Session::root();
    let mut posix = PosixFs::new(&mut fs, &root);

    let _fh = posix.create("/alpha.txt", 0o644).unwrap();
    posix.write("/alpha.txt", 0, b"same inode").unwrap();
    posix.link("/alpha.txt", "/beta.txt").unwrap();

    let alpha = posix.getattr("/alpha.txt").unwrap();
    let beta = posix.getattr("/beta.txt").unwrap();
    assert_eq!(alpha.inode_id, beta.inode_id);
    assert_eq!(alpha.nlink, 2);

    posix.unlink("/alpha.txt").unwrap();
    assert_eq!(posix.read("/beta.txt", 0, 32).unwrap(), b"same inode");
    assert_eq!(posix.getattr("/beta.txt").unwrap().nlink, 1);
}

#[test]
fn test_posix_open_handle_survives_unlink_until_release() {
    let mut fs = VirtualFs::new_posix();
    let root = Session::root();

    {
        let mut posix = PosixFs::new(&mut fs, &root);
        let fh = posix.create("/temp.log", 0o644).unwrap();
        posix.write("/temp.log", 0, b"orphaned").unwrap();
        posix.unlink("/temp.log").unwrap();
        assert_eq!(posix.read_handle(fh, 32).unwrap(), b"orphaned");
        posix.release(fh).unwrap();
    }

    assert!(fs.resolve_path("/temp.log").is_err());
}

#[test]
fn test_posix_setattr_updates_mode_and_size() {
    let mut fs = VirtualFs::new_posix();
    let root = Session::root();
    let mut posix = PosixFs::new(&mut fs, &root);

    let _fh = posix.create("/state.json", 0o644).unwrap();
    posix.write("/state.json", 0, br#"{"ok":true}"#).unwrap();

    let stat = posix
        .setattr(
            "/state.json",
            PosixSetAttr {
                mode: Some(0o600),
                size: Some(5),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(stat.mode, 0o600);
    assert_eq!(stat.size, 5);
    assert_eq!(posix.read("/state.json", 0, 16).unwrap(), br#"{"ok""#);
}
