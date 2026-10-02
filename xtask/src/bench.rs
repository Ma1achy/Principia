//! `cargo xtask bench <bench>` and `cargo xtask bench --all` (TASK-M0-19): run a fixed benchmark headless (telemetry
//! §1.1), write its profiler schema v1 trace, the file `prin profile` writes, to `target/bench/<bench>.jsonl`, and
//! compare it with its baseline, `fixtures/bench/<bench>/baseline.json`, through `prin profile diff`.
//!
//! Benchmarks run on the human's own Mac, at milestone gates and on demand; no hosted workflow runs one (R-186). The
//! benches live in `validation` (`validation::bench`), which xtask reaches only as a dev-dependency (systems_architecture
//! §7.1; R-176, R-187), so xtask runs validation's `bench` binary, release-built, as a separate process, as it does
//! `gate`. The kernel is built first, since `trivial-kernel` dispatches the WGSL `cargo xtask build-kernel` writes.
//!
//! The diff runs at a threshold of 0%, so every rise in a scope's p95 is listed; a rise is reported, not failed, since
//! no requirement yet gates a bench (the first is M3's REQ-PERF-004). Decided per R-369 (RQ-201).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::deps::cargo;

/// Which benches to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which<'a> {
    /// The bench so named.
    One(&'a str),
    /// Every registered bench.
    All,
}

/// The threshold `prin profile diff` is given: every rise is listed (RQ-201, decided per R-369).
pub const DIFF_THRESHOLD: &str = "0%";

/// Where a bench's trace is written, under the workspace root.
pub fn result_path(root: &Path, bench: &str) -> PathBuf {
    root.join("target/bench").join(format!("{bench}.jsonl"))
}

/// A bench's checked-in baseline, under the workspace root.
pub fn baseline_path(root: &Path, bench: &str) -> PathBuf {
    root.join("fixtures/bench")
        .join(bench)
        .join("baseline.json")
}

/// What `prin profile diff`'s exit code means for the bench: 0, no rise; 1, a rise, reported and not failed; anything
/// else (2, a usage or read error; none, killed) fails the bench.
pub fn diff_outcome(code: Option<i32>) -> Result<&'static str, String> {
    match code {
        Some(0) => Ok("no scope's p95 rose on the baseline"),
        Some(1) => Ok("a scope's p95 rose on the baseline (reported, not gated: no requirement gates this bench)"),
        Some(code) => Err(format!("prin profile diff failed (exit {code})")),
        None => Err("prin profile diff was killed".to_owned()),
    }
}

/// `cargo run --release --quiet` of `bin` in `package` on the workspace of `manifest`, with `args`.
fn cargo_run(manifest: &Path, package: &str, bin: &str) -> Command {
    let mut command = Command::new(cargo());
    command
        .args(["run", "--release", "--quiet", "--manifest-path"])
        .arg(manifest)
        .args(["-p", package, "--bin", bin, "--"]);
    command
}

/// The registered benches, as validation's `bench --list` names them.
fn registered(manifest: &Path) -> Result<Vec<String>, String> {
    let out = cargo_run(manifest, "validation", "bench")
        .arg("--list")
        .output()
        .map_err(|e| format!("cannot run validation's bench binary: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "bench --list failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect())
}

/// Runs `which` on the workspace of `manifest`; with `bless`, writes each trace as its bench's baseline first.
pub fn run(manifest: &Path, which: Which<'_>, bless: bool) -> Result<(), String> {
    let root = manifest.parent().unwrap_or(Path::new("."));
    crate::build_kernel::run(manifest)?;
    let names = match which {
        Which::One(name) => vec![name.to_owned()],
        Which::All => registered(manifest)?,
    };
    std::fs::create_dir_all(root.join("target/bench")).map_err(|e| e.to_string())?;
    for name in &names {
        let result = result_path(root, name);
        let status = cargo_run(manifest, "validation", "bench")
            .arg("--root")
            .arg(root)
            .arg(name)
            .arg("--out")
            .arg(&result)
            .status()
            .map_err(|e| format!("cannot run validation's bench binary: {e}"))?;
        if !status.success() {
            return Err(format!("bench {name} failed ({status})"));
        }
        println!("bench {name}: wrote {}", result.display());
        let baseline = baseline_path(root, name);
        if bless {
            std::fs::create_dir_all(baseline.parent().unwrap_or(root))
                .and_then(|()| std::fs::copy(&result, &baseline))
                .map_err(|e| format!("cannot write {}: {e}", baseline.display()))?;
            println!("bench {name}: baseline written to {}", baseline.display());
        }
        if !baseline.is_file() {
            return Err(format!(
                "bench {name}: no baseline at {}; `cargo xtask bench {name} --bless` writes it",
                baseline.display()
            ));
        }
        let status = cargo_run(manifest, "prin", "prin")
            .args(["profile", "diff"])
            .arg(&baseline)
            .arg(&result)
            .args(["--threshold", DIFF_THRESHOLD])
            .status()
            .map_err(|e| format!("cannot run prin profile diff: {e}"))?;
        println!("bench {name}: {}", diff_outcome(status.code())?);
    }
    Ok(())
}
