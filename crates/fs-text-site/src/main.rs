//! `fs-text-site build <source> <output>` and `fs-text-site build-all <sites-root> <output-root>`.
//!
//! The modification time of the output comes from `SITE_MTIME` (seconds since 1970) or
//! `--mtime N`, never from the clock. Exit code 0 = built, 1 = refused (the list is on stderr).

use fs_text_site::{build, build_all, BuildOptions, FolderSource};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, UNIX_EPOCH};

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut mtime = std::env::var("SITE_MTIME").ok();
    let mut revision = None;
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].clone();
        let flag = flag.as_str();
        if flag == "--mtime" || flag == "--revision" {
            if i + 1 >= args.len() {
                eprintln!("{flag} needs a value");
                return ExitCode::from(2);
            }
            let v = args.remove(i + 1);
            if flag == "--mtime" {
                mtime = Some(v);
            } else {
                revision = Some(v);
            }
            args.remove(i);
        } else {
            i += 1;
        }
    }
    let Some(secs) = mtime.and_then(|m| m.trim().parse::<u64>().ok()) else {
        eprintln!("SITE_MTIME (or --mtime) must be the release time in seconds since 1970");
        return ExitCode::from(2);
    };
    let opt = BuildOptions::new(UNIX_EPOCH + Duration::from_secs(secs));
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["build", src, out] => FolderSource::open(PathBuf::from(src)).and_then(|s| {
            let s = match &revision {
                Some(r) => s.with_revision(r),
                None => s,
            };
            build(&s, &PathBuf::from(out), &opt).map(|r| vec![(String::new(), r)])
        }),
        ["build-all", src, out] => build_all(&PathBuf::from(src), &PathBuf::from(out), &opt),
        _ => {
            eprintln!("usage: fs-text-site build <source> <output> | build-all <sites-root> <output-root> [--mtime N] [--revision R]");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(list) => {
            for (id, r) in list {
                if !id.is_empty() {
                    println!("== {id}");
                }
                print!("{}", r.to_text());
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprint!("{e}");
            ExitCode::FAILURE
        }
    }
}
