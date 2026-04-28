#![cfg(feature = "fuser")]

use clap::Parser;
use markdownfs::config::{CompatibilityTarget, Config};
use markdownfs::db::MarkdownDb;
use markdownfs::fuse_mount;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Parser)]
#[command(name = "markdownfs-mount", version, about = "Mount markdownfs through FUSE")]
struct Cli {
    #[arg(long, env = "MARKDOWNFS_MOUNTPOINT")]
    mountpoint: PathBuf,

    #[arg(long)]
    read_only: bool,
}

fn main() {
    let cli = Cli::parse();
    let config = Config::from_env().with_compatibility_target(CompatibilityTarget::Posix);
    let db = MarkdownDb::open(config).expect("failed to open database");
    let fs = Arc::new(Mutex::new(db.snapshot_fs()));
    fuse_mount::mount(fs, cli.mountpoint, cli.read_only).expect("failed to mount filesystem");
}
