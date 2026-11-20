//! A stand-in `cargo` for xtask's tests, in place of a shell script run as an executable, which Windows refuses
//! (REQ-TOOL-238). A test copies it into a directory of its own beside `stand_in.json`, its configuration, and points
//! xtask at the copy. Each key is optional, and the copy, as the keys set, in this order:
//!
//! - `log`: appends its arguments, joined by spaces, to that file; with `log_cwd` true, its working directory first,
//!   resolved (`canonicalize`) and without Windows' verbatim prefix, as `pwd -P` prints it;
//! - `metadata`: answers a `metadata` call with that text and exits 0;
//! - `real`: hands a call naming `golden_harness` to that cargo and exits with its status;
//! - `listed`: answers a call with `--list` with that text and exits 0;
//! - `build`: exits with that status on a call with `build-kernel`;
//! - `ran`: on a call with `--exact`, appends each argument after it to that file and reports it as a control that
//!   made its test fail;
//! - `filtered`: true, on a call with the filter `negative_control`, reports each control `listed` lists as having
//!   made its test fail;
//! - `output`: on any other call, prints that text;
//! - `library`: writes a stand-in library at `target/release/<library>` in its working directory;
//! - `status`: exits with it (0 if absent).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::{exit, Command};

use serde_json::Value;

// Beside the binary in a directory of its own: as `src/bin/plain.rs` cargo would take it for a binary.
#[path = "stand_in/plain.rs"]
mod plain;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exe = std::env::current_exe().expect("the stand-in's own path");
    let config: Value = serde_json::from_str(
        &fs::read_to_string(exe.with_file_name("stand_in.json")).expect("stand_in.json"),
    )
    .expect("stand_in.json is JSON");
    let text = |key: &str| config.get(key).and_then(Value::as_str);
    let set = |key: &str| config.get(key) == Some(&Value::Bool(true));
    let has = |arg: &str| args.iter().any(|a| a == arg);
    let joined = args.join(" ");

    if let Some(log) = text("log") {
        let line = if set("log_cwd") {
            // Resolved, as `pwd -P` resolves it, and without the verbatim `\\?\` prefix `canonicalize` adds on
            // Windows, which `pwd -P` doesn't print: a test compares it with `validation::paths::plain` of its
            // directory's `canonicalize`.
            let cwd = std::env::current_dir()
                .and_then(|dir| dir.canonicalize())
                .expect("the working directory");
            format!("{} {joined}", plain::plain(&cwd).display())
        } else {
            joined.clone()
        };
        append(log, &line);
    }
    if let Some(metadata) = text("metadata") {
        if args.first().map(String::as_str) == Some("metadata") {
            println!("{metadata}");
            exit(0);
        }
    }
    if let Some(real) = text("real") {
        if joined.contains("golden_harness") {
            let status = Command::new(real)
                .args(&args)
                .status()
                .expect("run the real cargo");
            exit(status.code().unwrap_or(1));
        }
    }
    let listed = text("listed");
    if let Some(listed) = listed {
        if has("--list") {
            print!("{listed}");
            exit(0);
        }
    }
    if let Some(build) = config.get("build").and_then(Value::as_i64) {
        if has("build-kernel") {
            exit(build as i32);
        }
    }
    let exact = args.iter().position(|a| a == "--exact");
    if let (Some(ran), Some(at)) = (text("ran"), exact) {
        for name in &args[at + 1..] {
            append(ran, name);
            println!("test {name} - should panic ... ok");
        }
    } else if set("filtered") && has("negative_control") {
        for line in listed.unwrap_or_default().lines() {
            if let Some(control) = line
                .strip_suffix(": test")
                .filter(|name| name.ends_with("::negative_control"))
            {
                println!("test {control} - should panic ... ok");
            }
        }
    } else if let Some(output) = text("output") {
        print!("{output}");
    }
    if let Some(library) = text("library") {
        let dir = PathBuf::from("target").join("release");
        fs::create_dir_all(&dir).expect("target/release");
        fs::write(dir.join(library), "a stand-in backend\n").expect("the stand-in library");
    }
    exit(config.get("status").and_then(Value::as_i64).unwrap_or(0) as i32);
}

/// Appends `line` and a newline to the file at `path`, creating it.
fn append(path: &str, line: &str) {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("the stand-in's log");
    writeln!(file, "{line}").expect("write the stand-in's log");
}
