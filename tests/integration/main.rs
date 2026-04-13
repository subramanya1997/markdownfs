use markdownfs::auth::session::Session;
use markdownfs::cmd;
use markdownfs::cmd::parser;
use markdownfs::fs::VirtualFs;

pub fn exec(line: &str, fs: &mut VirtualFs) -> String {
    let pipeline = parser::parse_pipeline(line);
    let mut session = Session::root();
    cmd::execute_pipeline(&pipeline, fs, &mut session).unwrap()
}

pub fn exec_err(line: &str, fs: &mut VirtualFs) -> String {
    let pipeline = parser::parse_pipeline(line);
    let mut session = Session::root();
    cmd::execute_pipeline(&pipeline, fs, &mut session)
        .unwrap_err()
        .to_string()
}

pub fn exec_s(line: &str, fs: &mut VirtualFs, session: &mut Session) -> String {
    let pipeline = parser::parse_pipeline(line);
    cmd::execute_pipeline(&pipeline, fs, session).unwrap()
}

mod dirs;
mod files;
mod nav;
mod rm_mv_cp;
mod metadata;
mod search;
mod pipes;
mod symlinks;
mod vcs;
mod persist;
mod permissions;
mod edge_cases;
mod workflows;
