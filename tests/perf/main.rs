use markdownfs::auth::session::Session;
use markdownfs::cmd;
use markdownfs::cmd::parser;
use markdownfs::fs::VirtualFs;
use std::time::Instant;

pub fn exec(line: &str, fs: &mut VirtualFs) -> String {
    let pipeline = parser::parse_pipeline(line);
    let mut session = Session::root();
    cmd::execute_pipeline(&pipeline, fs, &mut session).unwrap()
}

pub fn format_rate(count: usize, elapsed: std::time::Duration) -> String {
    let per_sec = count as f64 / elapsed.as_secs_f64();
    if per_sec > 1_000_000.0 {
        format!("{:.2}M ops/sec", per_sec / 1_000_000.0)
    } else if per_sec > 1_000.0 {
        format!("{:.2}K ops/sec", per_sec / 1_000.0)
    } else {
        format!("{:.2} ops/sec", per_sec)
    }
}

pub fn print_result(name: &str, count: usize, elapsed: std::time::Duration) {
    let per_op = elapsed / count as u32;
    println!(
        "  {:<50} {:>8} ops in {:>8.2?}  ({}, {:.2?}/op)",
        name,
        count,
        elapsed,
        format_rate(count, elapsed),
        per_op,
    );
}

pub fn debug_limit(secs: u64) -> u64 {
    if cfg!(debug_assertions) { secs * 6 } else { secs }
}

mod creation;
mod io;
mod listing;
mod search;
mod vcs;
mod persistence;
mod pipes;
mod workloads;
mod delete_mv_cp;
mod perms;
