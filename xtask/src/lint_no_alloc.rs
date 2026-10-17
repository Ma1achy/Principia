//! `cargo xtask lint no-alloc` — no allocation in the kernel's hot path (integrator_contract Part 2c, "Rules the new
//! kernel must hold by construction", rule 5; REQ-PERF-002). It reads every `.rs` file of the kernel's physics and
//! driver layers ([`LAYERS`]: each one's directory, and its module file beside it should it have one), its comments
//! and string literals blanked, and fails, naming the file, line and token, on any of [`TOKENS`]: `Vec`, `Box` or
//! `dyn` as a whole word, or `std` or `alloc` as a whole word followed by `::`.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::lint_constants::strip;

/// The layers the lint reads, relative to the workspace root: each is a directory, and may have a module file of the
/// same name with `.rs` beside it.
pub const LAYERS: [&str; 2] = ["crates/kernel/src/physics", "crates/kernel/src/driver"];

/// The forbidden tokens: the first three as whole words, the last two as whole words followed by `::`.
pub const TOKENS: [&str; 5] = ["Vec", "Box", "dyn", "std::", "alloc::"];

/// One forbidden token: its file, relative to the root, its line, and the token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub path: PathBuf,
    pub line: usize,
    pub token: &'static str,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: `{}` in the kernel's physics or driver layer, which allocates nothing and uses no `std` (Part 2c \
             rule 5)",
            self.path.display(),
            self.line,
            self.token
        )
    }
}

/// Whether `c` may be part of an identifier.
fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether `line` holds `token` at a word start: for a token ending in `::`, its name as a whole word followed by
/// `::` (whitespace allowed between); otherwise the token as a whole word.
fn holds(line: &str, token: &str) -> bool {
    let (name, path) = match token.strip_suffix("::") {
        Some(name) => (name, true),
        None => (token, false),
    };
    line.match_indices(name).any(|(i, _)| {
        let before = line[..i].chars().next_back();
        let rest = &line[i + name.len()..];
        let after = rest.chars().next();
        if before.is_some_and(word) || after.is_some_and(word) {
            return false;
        }
        !path || rest.trim_start().starts_with("::")
    })
}

/// The line and token of each of [`TOKENS`] in `source`, outside its comments and string literals.
pub fn scan(source: &str) -> Vec<(usize, &'static str)> {
    let code = strip(source);
    let mut found = Vec::new();
    for (n, line) in code.lines().enumerate() {
        for token in TOKENS {
            if holds(line, token) {
                found.push((n + 1, token));
            }
        }
    }
    found
}

/// The `.rs` files under `root/dir`, relative to `root`, into `files`; a missing directory holds none.
fn collect(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let Ok(entries) = fs::read_dir(root.join(dir)) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = dir.join(entry.file_name());
        if entry.path().is_dir() {
            collect(root, &path, files)?;
        } else if path.extension().is_some_and(|x| x == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

/// Every finding under `root`, in file order.
pub fn check(root: &Path) -> Result<Vec<Finding>, String> {
    let mut files = Vec::new();
    for layer in LAYERS {
        collect(root, Path::new(layer), &mut files)?;
        let module = PathBuf::from(format!("{layer}.rs"));
        if root.join(&module).is_file() {
            files.push(module);
        }
    }
    files.sort();
    let mut found = Vec::new();
    for path in files {
        let source =
            fs::read_to_string(root.join(&path)).map_err(|e| format!("{}: {e}", path.display()))?;
        found.extend(scan(&source).into_iter().map(|(line, token)| Finding {
            path: path.clone(),
            line,
            token,
        }));
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
        eprintln!("xtask lint no-alloc: {finding}");
    }
    if found.is_empty() {
        println!(
            "xtask lint no-alloc: no `Vec`, `Box`, `dyn`, `std::` or `alloc::` in the kernel's physics and driver layers"
        );
        Ok(())
    } else {
        Err(format!(
            "{} `Vec`, `Box`, `dyn`, `std::` or `alloc::` use(s) in the kernel's physics and driver layers \
             (REQ-PERF-002)",
            found.len()
        ))
    }
}
