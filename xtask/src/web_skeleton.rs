//! `cargo xtask web-skeleton` (TASK-M8-86, R-627): builds the v1 web skeleton's site into one directory, what
//! `.github/workflows/pages.yml` deploys by hand. The gui crate is built as a cdylib for `wasm32-unknown-unknown` with its
//! `web` feature, through `cargo rustc`, so its native build stays as it is; `wasm-bindgen`, at the version pinned here,
//! which must be the `wasm-bindgen` crate's in `Cargo.lock`, generates the bindings; and `web/skeleton`'s files are copied
//! beside them.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The `wasm-bindgen` CLI's version: the crate's in `Cargo.lock`, which it must match exactly (`gui`'s manifest pins the
/// crate; `plan/HUMAN_SETUP.md` §6 and pages.yml install the CLI at it).
pub const WASM_BINDGEN_VERSION: &str = "0.2.129";

/// The files of `web/skeleton` the site is made of. The runner stays out: it drives the site.
pub const SITE_FILES: [&str; 5] = [
    "index.html",
    "coi-serviceworker.js",
    "loader.js",
    "worker.js",
    "probe-worker.js",
];

/// The wasm target.
pub const TARGET: &str = "wasm32-unknown-unknown";

fn root() -> PathBuf {
    crate::workspace_manifest()
        .parent()
        .expect("the workspace manifest has a directory")
        .to_owned()
}

/// Where the site goes by default.
pub fn default_out() -> PathBuf {
    root().join("target/web-skeleton")
}

/// The version `--version`'s `output` names, if it is `wasm-bindgen <version>`.
pub fn cli_version(output: &str) -> Option<&str> {
    output
        .trim()
        .strip_prefix("wasm-bindgen ")?
        .split(' ')
        .next()
}

/// The version the `wasm-bindgen` package has in the lock file's `text`.
pub fn locked_version(text: &str) -> Option<&str> {
    let (_, after) = text.split_once("name = \"wasm-bindgen\"\nversion = \"")?;
    after.split('"').next()
}

/// Refuses a CLI whose `--version` `output` is not [`WASM_BINDGEN_VERSION`]: its output would not match the crate.
pub fn check_cli(output: &str) -> Result<(), String> {
    match cli_version(output) {
        Some(WASM_BINDGEN_VERSION) => Ok(()),
        _ => Err(format!(
            "wasm-bindgen reports {:?}, not {WASM_BINDGEN_VERSION}; run `cargo install wasm-bindgen-cli --version \
             {WASM_BINDGEN_VERSION} --locked` (plan/HUMAN_SETUP.md §6)",
            output.trim()
        )),
    }
}

/// Runs `command`, failing with its name.
fn run(command: &mut Command) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|e| format!("{:?}: {e}", command.get_program()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{:?} failed: {status}", command.get_program()))
    }
}

/// The target directory cargo builds in.
fn target_dir(root: &Path) -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

/// Builds the site into `out`.
pub fn build(out: &Path) -> Result<(), String> {
    let root = root();
    let version = Command::new("wasm-bindgen")
        .arg("--version")
        .output()
        .map_err(|e| {
            format!(
                "wasm-bindgen: {e}; run `cargo install wasm-bindgen-cli --version \
                 {WASM_BINDGEN_VERSION} --locked` (plan/HUMAN_SETUP.md §6)"
            )
        })?;
    check_cli(&String::from_utf8_lossy(&version.stdout))?;
    run(Command::new(env!("CARGO"))
        .current_dir(&root)
        .args(["rustc", "-p", "gui", "--lib", "--release", "--locked"])
        .args(["--target", TARGET, "--features", "web"])
        .args(["--crate-type", "cdylib"]))?;
    let wasm = target_dir(&root).join(TARGET).join("release/gui.wasm");
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    run(Command::new("wasm-bindgen")
        .arg(&wasm)
        .args(["--target", "web", "--no-typescript", "--out-dir"])
        .arg(out))?;
    for file in SITE_FILES {
        std::fs::copy(root.join("web/skeleton").join(file), out.join(file))
            .map_err(|e| format!("{file}: {e}"))?;
    }
    println!("the site is in {}", out.display());
    Ok(())
}
