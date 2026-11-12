//! The registry's scan (gui_state_contract §3; REQ-GEN-020): which `.wgsl` files are occupants, and the id each
//! registers under. It reads only the filesystem and uses only `std`, because the build runs it too: `build.rs`
//! compiles this file, scans the render crate with [`occupant_files`] and embeds what it finds as the table
//! [`shipped_table`] writes, so the crate's own registry is in the binary and reads no file at run time (R-815,
//! RQ-677 (e)). The library runs the same scan on any render crate ([`super::scan()`]).
//!
//! **What is scanned.** [`ROOTS`]: the hand-written occupants under `shaders/wgsl/frag/` ([`OCCUPANT_DIR`]), one
//! directory per slot and `debug/`; the debug catalogue's generated views under `frag/debug/generated/`
//! ([`GENERATED_DIR`]; REQ-GEN-009) and its generated reductions under `frag/debug/reductions/` ([`REDUCTIONS_DIR`]).
//! `shaders/wgsl/compositor/`, the fixed passes, is under none of them, so it is not scanned.

use std::path::{Path, PathBuf};

/// The hand-written occupants, relative to the render crate: one directory per slot (gui_state_contract §3).
pub const OCCUPANT_DIR: &str = "shaders/wgsl/frag";

/// The debug catalogue's generated views, relative to the render crate (RQ-219; `ledger::gen::catalogue::DIR`).
pub const GENERATED_DIR: &str = "frag/debug/generated";

/// The debug catalogue's generated reductions, relative to the render crate (`ledger::gen::catalogue::REDUCTIONS_DIR`).
pub const REDUCTIONS_DIR: &str = "frag/debug/reductions";

/// The scanned directories, relative to the render crate, each with what leads the id of a file under it.
pub const ROOTS: [(&str, &str); 3] = [
    (OCCUPANT_DIR, ""),
    (GENERATED_DIR, "debug/generated/"),
    (REDUCTIONS_DIR, "debug/reductions/"),
];

/// One `.wgsl` file the scan found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    /// The file's path under its scan root, `/`-separated, without `.wgsl`, led by the root's prefix:
    /// `combiner/multiply`, `debug/generated/state`.
    pub id: String,
    /// The file's path relative to the render crate, `/`-separated: `shaders/wgsl/frag/combiner/multiply.wgsl`.
    pub rel: String,
    /// The file, under the render crate as the scan was given it.
    pub path: PathBuf,
}

/// A directory the scan could not read, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanError {
    pub path: PathBuf,
    pub why: String,
}

/// Every `.wgsl` file under the [`ROOTS`] of the render crate at `render`, sorted by id. A missing directory holds
/// no file.
pub fn occupant_files(render: &Path) -> Result<Vec<Found>, ScanError> {
    let mut out = Vec::new();
    for (root, prefix) in ROOTS {
        let dir = render.join(root);
        let mut files = Vec::new();
        wgsl_files(&dir, &mut files)?;
        for path in files {
            let under = path.strip_prefix(&dir).unwrap_or(&path);
            let parts: Vec<String> = under
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            let stem: Vec<String> = under
                .with_extension("")
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            out.push(Found {
                id: format!("{prefix}{}", stem.join("/")),
                rel: format!("{root}/{}", parts.join("/")),
                path,
            });
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// Each `.wgsl` file under `dir`, recursively; none if `dir` does not exist.
fn wgsl_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), ScanError> {
    if !dir.is_dir() {
        return Ok(());
    }
    let io = |e: std::io::Error| ScanError {
        path: dir.to_owned(),
        why: e.to_string(),
    };
    for item in std::fs::read_dir(dir).map_err(io)? {
        let path = item.map_err(io)?.path();
        if path.is_dir() {
            wgsl_files(&path, out)?;
        } else if path.extension().is_some_and(|x| x == "wgsl") {
            out.push(path);
        }
    }
    Ok(())
}

/// The Rust source of the embedded table, `SHIPPED`: one `(id, rel, source)` per file of `found`, in its order, the
/// source embedded from the file with `include_str!`. `build.rs` writes it to `$OUT_DIR/frag_registry.rs` from a scan
/// of an absolute path, since `include_str!` reads a relative one against the file it is written in.
pub fn shipped_table(found: &[Found]) -> String {
    let mut out = String::from(
        "/// The render crate's occupants as the build scanned them: each file's registry id, its path relative to \
         the crate, and its WGSL.\npub const SHIPPED: &[(&str, &str, &str)] = &[\n",
    );
    for f in found {
        out.push_str(&format!(
            "    ({:?}, {:?}, include_str!({:?})),\n",
            f.id,
            f.rel,
            f.path.to_string_lossy()
        ));
    }
    out.push_str("];\n");
    out
}
