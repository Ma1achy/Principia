//! `cargo xtask lint compute-pipelines` — every compute pipeline is created through the one entry point that takes
//! the fast-math setting, `engine::compute::pipeline`, and no vertex or fragment pipeline goes through wgpu's
//! passthrough (R-297; REQ-SYS-074). It reads every `.rs` file under `crates/` and `xtask/`, its comments and string
//! literals blanked, and fails, naming the file, line and identifier, on:
//! - wgpu's compute-pipeline creation ([`COMPUTE`]) outside the entry point's file, [`ENTRY_POINT`];
//! - wgpu's passthrough ([`PASSTHROUGH`]) outside it, so only the entry point's compute pipelines can use it;
//! - a vertex or fragment pipeline ([`DISPLAY`]) inside it, so the passthrough it holds never reaches one.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::lint_constants::strip;

/// The entry point's file, relative to the workspace root: the one file that creates compute pipelines and uses the
/// passthrough.
pub const ENTRY_POINT: &str = "crates/engine/src/compute.rs";

/// The directories the lint reads, relative to the workspace root.
pub const DIRS: [&str; 2] = ["crates", "xtask"];

/// wgpu's identifiers for creating a compute pipeline: only the entry point uses them.
pub const COMPUTE: [&str; 2] = ["create_compute_pipeline", "ComputePipelineDescriptor"];

/// wgpu's identifiers for a passthrough shader module: only the entry point uses them.
pub const PASSTHROUGH: [&str; 3] = [
    "create_shader_module_passthrough",
    "ShaderModuleDescriptorPassthrough",
    "PassthroughShaderEntryPoint",
];

/// wgpu's identifiers for a vertex or fragment pipeline: the entry point, which holds the passthrough, uses none.
pub const DISPLAY: [&str; 4] = [
    "create_render_pipeline",
    "RenderPipelineDescriptor",
    "VertexState",
    "FragmentState",
];

/// One identifier where the lint forbids it: its file, relative to the root, its line, and the identifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub path: PathBuf,
    pub line: usize,
    pub ident: &'static str,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rule = if DISPLAY.contains(&self.ident) {
            format!("a vertex or fragment pipeline beside the passthrough in {ENTRY_POINT}")
        } else if PASSTHROUGH.contains(&self.ident) {
            format!("wgpu's passthrough outside the compute entry point, {ENTRY_POINT}")
        } else {
            format!("a compute pipeline created outside the entry point, {ENTRY_POINT}")
        };
        write!(
            f,
            "{}:{}: `{}`: {rule} (R-297)",
            self.path.display(),
            self.line,
            self.ident
        )
    }
}

/// The identifiers forbidden in the file at `path`, relative to the root: [`DISPLAY`] in the entry point,
/// [`COMPUTE`] and [`PASSTHROUGH`] anywhere else.
pub fn forbidden(path: &Path) -> Vec<&'static str> {
    if path == Path::new(ENTRY_POINT) {
        DISPLAY.to_vec()
    } else {
        COMPUTE.iter().chain(&PASSTHROUGH).copied().collect()
    }
}

/// The line and identifier of each of `idents` in `source`, outside its comments and string literals, as a whole
/// word.
pub fn scan(source: &str, idents: &[&'static str]) -> Vec<(usize, &'static str)> {
    let code = strip(source);
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut found = Vec::new();
    for (n, line) in code.lines().enumerate() {
        for &ident in idents {
            let whole = line.match_indices(ident).any(|(i, _)| {
                let before = line[..i].chars().next_back();
                let after = line[i + ident.len()..].chars().next();
                !before.is_some_and(word) && !after.is_some_and(word)
            });
            if whole {
                found.push((n + 1, ident));
            }
        }
    }
    found
}

/// The `.rs` files under `root/dir`, relative to `root`, into `files`, skipping `target` directories; a missing
/// directory holds none.
fn collect(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let Ok(entries) = fs::read_dir(root.join(dir)) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = dir.join(entry.file_name());
        if entry.path().is_dir() {
            if entry.file_name() != "target" {
                collect(root, &path, files)?;
            }
        } else if path.extension().is_some_and(|x| x == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

/// Every finding under `root`, in file order.
pub fn check(root: &Path) -> Result<Vec<Finding>, String> {
    let mut files = Vec::new();
    for dir in DIRS {
        collect(root, Path::new(dir), &mut files)?;
    }
    files.sort();
    let mut found = Vec::new();
    for path in files {
        let source =
            fs::read_to_string(root.join(&path)).map_err(|e| format!("{}: {e}", path.display()))?;
        let idents = forbidden(&path);
        found.extend(
            scan(&source, &idents)
                .into_iter()
                .map(|(line, ident)| Finding {
                    path: path.clone(),
                    line,
                    ident,
                }),
        );
    }
    Ok(found)
}

/// Runs the lint over the workspace whose `Cargo.toml` is `manifest`.
pub fn run(manifest: &Path) -> Result<(), String> {
    let root = manifest
        .parent()
        .ok_or_else(|| format!("{}: no parent directory", manifest.display()))?;
    let found = check(root)?;
    for finding in &found {
        eprintln!("xtask lint compute-pipelines: {finding}");
    }
    if found.is_empty() {
        println!(
            "xtask lint compute-pipelines: every compute pipeline is created through {ENTRY_POINT}, and no vertex or \
             fragment pipeline beside its passthrough"
        );
        Ok(())
    } else {
        Err(format!(
            "{} compute pipeline or passthrough use(s) outside the entry point (R-297, REQ-SYS-074)",
            found.len()
        ))
    }
}
