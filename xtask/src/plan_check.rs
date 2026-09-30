//! `cargo xtask plan-check` — runs `plan/check_plan.py` from the repo root, which also runs `coverage.py`,
//! `milestones.py` and `reviewer_lists.py` with `--check` (REQ-SYS-007, REQ-SYS-008).

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

/// The checker, relative to the repo root.
pub const SCRIPT: &str = "plan/check_plan.py";

/// This workspace's repo root.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// `python3 plan/check_plan.py`, run in `root` (the checker finds the plan and the corpus from its own path).
pub fn command(root: &Path) -> Command {
    let mut command = Command::new("python3");
    command.arg(SCRIPT).current_dir(root);
    command
}

/// The error for a `python3` that could not be started.
fn not_started(e: std::io::Error) -> String {
    match e.kind() {
        ErrorKind::NotFound => format!(
            "python3 not found on PATH: plan-check runs {SCRIPT}, which needs Python 3 and PyYAML \
             (`pip install pyyaml`)"
        ),
        _ => format!("could not run python3: {e}"),
    }
}

/// Runs the checker in `root`, its output streamed; returns its exit status. A clear error first if `python3` or
/// PyYAML is missing.
pub fn status(root: &Path) -> Result<ExitStatus, String> {
    let probe = Command::new("python3")
        .args(["-c", "import yaml"])
        .current_dir(root)
        .output()
        .map_err(not_started)?;
    if !probe.status.success() {
        return Err(format!(
            "PyYAML is missing: plan-check runs {SCRIPT}, which imports yaml; install it with `pip install pyyaml`\n{}",
            String::from_utf8_lossy(&probe.stderr).trim_end()
        ));
    }
    command(root).status().map_err(not_started)
}

/// `cargo xtask plan-check` as a CI runner (R-177): `Err` if the checker fails.
pub fn run(root: &Path) -> Result<(), String> {
    let status = status(root)?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{SCRIPT} failed ({status})"))
    }
}
