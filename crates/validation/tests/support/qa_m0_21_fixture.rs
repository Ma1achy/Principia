//! The fixture copy and the `cargo xtask controls` run that `qa_TASK-M0-21.rs` and `qa_TASK-M0-21_r2.rs` share with
//! their controls in `qa_TASK-M0-21_controls.rs` (REQ-VAL-157; R-215). A `tests/*.rs` file is a crate of its own, so
//! each includes this file with `#[path]`; every item here is used by each of the three.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;
use validation::spawn::Spawn;

// A target directory of its own for each nested cargo run (REQ-VAL-164; R-227).
#[path = "own_target.rs"]
mod own_target;
use own_target::{fixture_files, Lease, FIXTURES};

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

/// The `xtask` binary, built once, in the pool's `xtask` directory, which this process holds while it runs: a build
/// into the workspace's would replace the `xtask` other tests spawn, and so would another process's build into that
/// directory (REQ-VAL-164). One directory, kept across runs, so a warm run rebuilds nothing (R-270).
fn xtask() -> &'static Path {
    static BIN: OnceLock<(Lease, PathBuf)> = OnceLock::new();
    let (_, bin) = BIN.get_or_init(|| {
        let target = Lease::take(FIXTURES, Some("xtask"));
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

/// A manifest's text with its relative `crates/validation` path made absolute, so the copy builds outside the
/// workspace.
fn absolutise(text: &str, validation: &str) -> String {
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
    fixed + "\n"
}

/// A copy of the fixture `name` outside this workspace, with the files `remove` left out, and the target directory it
/// builds in, its fixture type's, held while the copy is (REQ-VAL-164, R-270). The copy is one per fixture and
/// `remove`, kept across runs (R-231).
pub struct Copy(PathBuf, Lease);

impl Copy {
    pub fn new(name: &str, remove: &[&str]) -> Copy {
        // The `xtask` directory first, then the fixture type's, in every process, so no two wait on each other.
        xtask();
        let validation = root().join("crates/validation");
        let mut files: Vec<(PathBuf, Vec<u8>)> =
            fixture_files(&root().join("fixtures/controls_qa_m0_21").join(name))
                .into_iter()
                .filter(|(path, _)| !remove.iter().any(|file| path == Path::new(file)))
                .map(|(path, bytes)| match path.file_name() {
                    Some(file) if file == "Cargo.toml" => {
                        let text = String::from_utf8(bytes).unwrap();
                        let text = absolutise(&text, validation.to_str().unwrap());
                        (path, text.into_bytes())
                    }
                    _ => (path, bytes),
                })
                .collect();
        files.push((
            PathBuf::from("Cargo.lock"),
            std::fs::read(root().join("Cargo.lock")).unwrap(),
        ));
        let removed: String = remove
            .iter()
            .map(|file| format!("-{}", file.replace(['/', '.'], "_")))
            .collect();
        let copy = format!("{name}{removed}");
        let target = Lease::take(FIXTURES, Some("controls_qa_m0_21"));
        let copy = target.copy(&copy, &files);
        Copy(copy, target)
    }

    pub fn manifest(&self) -> PathBuf {
        self.0.join("Cargo.toml")
    }

    /// The copy's target directory, for `CARGO_TARGET_DIR`.
    pub fn target(&self) -> &Path {
        self.1.dir()
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
