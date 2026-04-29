#![cfg(feature = "fuser")]

use clap::Parser;
use markdownfs::config::{CompatibilityTarget, Config};
use markdownfs::db::MarkdownDb;
use markdownfs::fuse_mount;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Parser)]
#[command(name = "mdfs-mount", version, about = "Mount mdfs through FUSE")]
struct Cli {
    #[arg(long, env = "MARKDOWNFS_MOUNTPOINT")]
    mountpoint: PathBuf,

    #[arg(long)]
    read_only: bool,
}

fn main() {
    let cli = Cli::parse();
    let config = Config::from_env().with_compatibility_target(CompatibilityTarget::Posix);
    let db = match MarkdownDb::open(config) {
        Ok(db) => db,
        Err(err) => {
            eprintln!("failed to open database: {err}");
            std::process::exit(1);
        }
    };
    let fs = Arc::new(Mutex::new(db.snapshot_fs()));
    if let Err(err) = fuse_mount::mount(fs, cli.mountpoint, cli.read_only) {
        eprintln!("failed to mount filesystem: {err}");
        std::process::exit(1);
    }
}
