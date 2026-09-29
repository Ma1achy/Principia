//! The fixture copy and the `cargo xtask controls` run that `qa_TASK-M0-21.rs` and `qa_TASK-M0-21_r2.rs` share with
//! their controls in `qa_TASK-M0-21_controls.rs` (REQ-VAL-157; R-215). A `tests/*.rs` file is a crate of its own, so
//! each includes this file with `#[path]`; every item here is used by each of the three.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use validation::spawn::Spawn;

// A target directory of its own for each nested cargo run (REQ-VAL-164; R-227).
#[path = "own_target.rs"]
mod own_target;
use own_target::{Lease, FIXTURES};

/// The workspace root (this crate is `crates/validation`).
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

pub fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

/// The `xtask` binary, built once, in a target directory this process holds while it runs: a build into the
/// workspace's would replace the `xtask` other tests spawn (REQ-VAL-164).
fn xtask() -> &'static Path {
    static BIN: OnceLock<(Lease, PathBuf)> = OnceLock::new();
    let (_, bin) = BIN.get_or_init(|| {
        let target = Lease::take("qa_m0_21-xtask");
        let status = Command::new(cargo())
            .args(["build", "-p", "xtask", "--manifest-path"])
            .arg(root().join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", target.dir())
            .timed_output()
            .expect("run cargo build")
            .status;
        assert!(status.success(), "cargo build -p xtask failed");
        let bin = target
            .dir()
            .join("debug")
            .join(format!("xtask{}", std::env::consts::EXE_SUFFIX));
        (target, bin)
    });
    bin
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let dest = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &dest);
        } else {
            std::fs::copy(&path, &dest).unwrap();
        }
    }
}

/// Makes each manifest's relative `crates/validation` path absolute, so the copy builds outside the workspace.
fn absolutise(dir: &Path, validation: &str) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            absolutise(&path, validation);
        } else if path.file_name().unwrap() == "Cargo.toml" {
            let text = std::fs::read_to_string(&path).unwrap();
            let fixed: String = text
                .lines()
                .map(|line| match line.find("path = \"") {
                    Some(at) if line.contains("crates/validation\"") => {
                        format!("{}path = \"{validation}\" }}", &line[..at])
                    }
                    _ => line.to_owned(),
                })
                .collect::<Vec<_>>()
                .join("\n");
            std::fs::write(&path, fixed + "\n").unwrap();
        }
    }
}

/// A copy of the fixture `name` outside this workspace, with the files `remove` deleted, and the target directory it
/// builds in, its own while the copy lives (REQ-VAL-164); the copy is removed on drop.
pub struct Copy(PathBuf, Lease);

impl Copy {
    pub fn new(name: &str, remove: &[&str]) -> Copy {
        static RUN: AtomicUsize = AtomicUsize::new(0);
        let run = RUN.fetch_add(1, Ordering::Relaxed);
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("qa_m0_21")
            .join(format!("{name}-{}-{run}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        copy_dir(&root().join("fixtures/controls_qa_m0_21").join(name), &dir);
        for file in remove {
            std::fs::remove_file(dir.join(file)).unwrap();
        }
        absolutise(&dir, root().join("crates/validation").to_str().unwrap());
        std::fs::copy(root().join("Cargo.lock"), dir.join("Cargo.lock")).unwrap();
        Copy(dir, Lease::take(FIXTURES))
    }

    pub fn manifest(&self) -> PathBuf {
        self.0.join("Cargo.toml")
    }

    /// The copy's target directory, for `CARGO_TARGET_DIR`.
    pub fn target(&self) -> &Path {
        self.1.dir()
    }
}

impl Drop for Copy {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub struct Verdict {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
}

impl Verdict {
    pub fn from(output: Output) -> Verdict {
        Verdict {
            ok: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    pub fn all(&self) -> String {
        format!("stdout:\n{}\nstderr:\n{}", self.stdout, self.stderr)
    }
}

/// `cargo xtask controls --manifest-path` on a copy of fixture `name` with `remove` deleted.
pub fn controls(name: &str, remove: &[&str]) -> Verdict {
    let copy = Copy::new(name, remove);
    Verdict::from(
        Command::new(xtask())
            .args(["controls", "--manifest-path"])
            .arg(copy.manifest())
            .env("CARGO_TARGET_DIR", copy.target())
            .timed_output()
            .expect("run xtask controls"),
    )
}
