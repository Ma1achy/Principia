//! `cargo xtask deps` — checks the workspace crate graph against the allowed-edge table of
//! systems_architecture §7.1, the crate map (R-170, confirmed by R-185). REQ-SYS-004.
//!
//! Edges *inside* one crate (decoder before kernel, canonicalise before the integrator) are not visible to
//! a crate-graph check; they stay a code-review item (systems_architecture §7.1).
//!
//! It also enforces R-187's condition on the `validation` dev-dependency: in `kernel` and `ledger`, no
//! source under `src/` uses `validation` (a test that does is an integration test in `tests/`), because
//! the dev-dependency cycle would give unit tests two copies of the crate (`Metadata::source_violations`).

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use proc_macro2::{TokenStream, TokenTree};
use serde::Deserialize;

/// The kind of a dependency, as `cargo metadata` reports it (`kind`: `null`, `"dev"`, `"build"`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepKind {
    Normal,
    Dev,
    Build,
}

impl fmt::Display for DepKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            DepKind::Normal => "normal dependency",
            DepKind::Dev => "dev-dependency",
            DepKind::Build => "build-dependency",
        })
    }
}

/// Which dependency kinds an allowed edge admits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kinds {
    /// Normal, build or dev.
    Any,
    /// Build-dependency only.
    BuildOnly,
    /// Dev-dependency only.
    DevOnly,
}

impl Kinds {
    fn admits(self, kind: DepKind) -> bool {
        match self {
            Kinds::Any => true,
            Kinds::BuildOnly => kind == DepKind::Build,
            Kinds::DevOnly => kind == DepKind::Dev,
        }
    }
}

/// One allowed workspace edge: `from` depends on `to`.
#[derive(Clone, Copy, Debug)]
pub struct AllowedEdge {
    pub from: &'static str,
    pub to: &'static str,
    pub kinds: Kinds,
}

/// `from` for §7.1's "any except `gui` (dev-dependency only) → `validation`" row (R-176, R-187): every
/// workspace crate but `gui` and `validation` itself, `kernel` and `ledger` included.
const ANY_EXCEPT_GUI: &str = "*";

/// The allowed workspace edges, transcribed from systems_architecture §7.1 ("Allowed workspace edges";
/// arrows read "depends on"). Every other workspace edge is forbidden.
pub const ALLOWED: &[AllowedEdge] = &[
    // §7: layout table → pack/unpack gen → kernel; link registry → decoder.
    // Build-dependency only (R-185): the ledger generates code into the kernel at build time.
    AllowedEdge { from: "kernel", to: "ledger", kinds: Kinds::BuildOnly },
    // §7: layout table → pack/unpack gen (WGSL), debug catalogue gen.
    AllowedEdge { from: "render", to: "ledger", kinds: Kinds::Any },
    // §7: kernel → dispatch; chart system → validation → resolve/lowering.
    AllowedEdge { from: "engine", to: "ledger", kinds: Kinds::Any },
    AllowedEdge { from: "engine", to: "kernel", kinds: Kinds::Any },
    // §7: payload → fragment assembly (the frame loop and dispatch drive the fragment side).
    AllowedEdge { from: "engine", to: "render", kinds: Kinds::Any },
    // GUI → state → engine (gui_state_contract §1).
    AllowedEdge { from: "gui", to: "engine", kinds: Kinds::Any },
    // The CLI depends on engine.
    AllowedEdge { from: "prin", to: "engine", kinds: Kinds::Any },
    // validation → any of the above except gui and prin: the harness exercises each seam; where it needs
    // the CLI it runs the built `prin` binary as a separate process (R-187). So no validation → prin.
    AllowedEdge { from: "validation", to: "kernel", kinds: Kinds::Any },
    AllowedEdge { from: "validation", to: "ledger", kinds: Kinds::Any },
    AllowedEdge { from: "validation", to: "render", kinds: Kinds::Any },
    AllowedEdge { from: "validation", to: "engine", kinds: Kinds::Any },
    // any except gui → validation, dev-dependency only (R-176, R-187). Never a normal or build dependency,
    // so the no_std kernel and rust-gpu builds never see it.
    AllowedEdge { from: ANY_EXCEPT_GUI, to: "validation", kinds: Kinds::DevOnly },
];

/// One workspace dependency edge: `from` depends on `to` with `kind`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: DepKind,
}

/// A forbidden edge, with the rule it breaks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub edge: Edge,
    pub rule: &'static str,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "forbidden edge {} → {} ({}): {}",
            self.edge.from, self.edge.to, self.edge.kind, self.rule
        )
    }
}

/// Checks each edge against the invariants of systems_architecture §7.1 and the allowed-edge table.
/// Returns every violation (empty when the graph is allowed).
pub fn check(edges: &[Edge]) -> Vec<Violation> {
    edges
        .iter()
        .filter_map(|edge| rule_broken(edge).map(|rule| Violation { edge: edge.clone(), rule }))
        .collect()
}

fn rule_broken(edge: &Edge) -> Option<&'static str> {
    let (from, to) = (edge.from.as_str(), edge.to.as_str());
    if to == "gui" {
        return Some("nothing depends on gui (systems_architecture §7.1; gui_state_contract §1)");
    }
    if from == "gui" && to == "validation" {
        return Some("gui never depends on validation, in any kind (systems_architecture §7.1; R-187)");
    }
    if from == "validation" && to == "prin" {
        return Some(
            "validation never depends on prin; it runs the built binary as a separate process \
             (systems_architecture §7.1; R-187)",
        );
    }
    if to == "validation" && edge.kind != DepKind::Dev {
        return Some(
            "validation is reached only as a dev-dependency, never a normal or build dependency \
             (systems_architecture §7.1; R-176, R-187)",
        );
    }
    if from == "ledger" && to != "validation" {
        return Some(
            "ledger is a root: it has no workspace dependency but validation as a dev-dependency \
             (systems_architecture §7.1; R-187)",
        );
    }
    if from == "kernel" && to != "ledger" && to != "validation" {
        return Some(
            "kernel depends on no workspace crate but ledger, and validation as a dev-dependency \
             (systems_architecture §7.1; R-187)",
        );
    }
    if from == "kernel" && to == "ledger" && edge.kind != DepKind::Build {
        return Some("kernel → ledger is a build-dependency only (R-185)");
    }
    let allowed = ALLOWED.iter().any(|a| {
        let from_matches =
            a.from == from || (a.from == ANY_EXCEPT_GUI && from != "gui" && from != a.to);
        from_matches && a.to == to && a.kinds.admits(edge.kind)
    });
    if allowed {
        None
    } else {
        Some("not in the allowed-edge table (systems_architecture §7.1)")
    }
}

/// The subset of `cargo metadata --format-version 1` the check reads.
#[derive(Debug, Deserialize)]
pub struct Metadata {
    pub packages: Vec<Package>,
    pub workspace_members: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Package {
    pub name: String,
    pub id: String,
    /// The package's `Cargo.toml`. `cargo metadata` always writes it; the minimal test fixtures may not.
    #[serde(default)]
    pub manifest_path: Option<String>,
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub kind: Option<String>,
    /// `null` for a path dependency; `registry+…` or `git+…` otherwise.
    #[serde(default)]
    pub source: Option<String>,
    /// The directory of a path dependency.
    #[serde(default)]
    pub path: Option<String>,
    /// The name the dependent uses for it, when renamed in its `Cargo.toml`.
    #[serde(default)]
    pub rename: Option<String>,
}

impl Dependency {
    /// Whether this dependency resolves to the workspace member `member`, as opposed to a package that only
    /// shares its name. A registry or git dependency (`source` set) is never a workspace member. A path
    /// dependency is one only if its `path` is the member's directory, when both paths are known.
    fn is_on(&self, member: &Package) -> bool {
        if self.source.is_some() || self.name != member.name {
            return false;
        }
        match (&self.path, &member.manifest_path) {
            (Some(path), Some(manifest)) => {
                Path::new(manifest).parent() == Some(Path::new(path.as_str()))
            }
            _ => true,
        }
    }
}

impl Metadata {
    /// Runs `cargo metadata --format-version 1` on this workspace.
    pub fn from_cargo() -> Result<Self, String> {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml");
        let output = Command::new(cargo)
            .args(["metadata", "--format-version", "1", "--manifest-path"])
            .arg(&manifest)
            .output()
            .map_err(|e| format!("cannot run cargo metadata: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "cargo metadata failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Self::from_json(&output.stdout)
    }

    /// Reads a `cargo metadata` JSON document from a file (the test fixtures).
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let bytes =
            std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        Self::from_json(&bytes)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(bytes).map_err(|e| format!("cannot parse cargo metadata: {e}"))
    }

    /// The workspace members, in the order `cargo metadata` lists them.
    fn members(&self) -> Vec<&Package> {
        self.packages.iter().filter(|p| self.workspace_members.contains(&p.id)).collect()
    }

    /// R-187's condition on the `validation` dev-dependency: in each crate of `NO_VALIDATION_IN_SRC` that
    /// depends on `validation`, no `.rs` file under its `src/` has an identifier named `validation` or the name
    /// the crate gives the dependency (`identifier_lines`: a deliberate over-approximation that fails local items
    /// with that name too). A crate without that dependency cannot use the crate, so it is not scanned.
    ///
    /// `require_sources`: whether a crate to scan must have its `src/` on disk. It is set for the live
    /// workspace; a fixture may describe a graph with no sources behind it, and then the scan is skipped.
    pub fn source_violations(&self, require_sources: bool) -> Result<Vec<SourceViolation>, String> {
        let members = self.members();
        let Some(validation) = members.iter().find(|m| m.name == "validation") else {
            return Ok(Vec::new());
        };
        let mut violations = Vec::new();
        for package in members.iter().filter(|m| NO_VALIDATION_IN_SRC.contains(&m.name.as_str())) {
            let deps: Vec<&Dependency> = package.dependencies.iter().filter(|d| d.is_on(validation)).collect();
            // The crate's own name and each name the dependency is given (a rename).
            let mut names: Vec<String> = deps
                .iter()
                .flat_map(|d| [Some(&d.name), d.rename.as_ref()])
                .flatten()
                .map(|n| n.replace('-', "_"))
                .collect();
            names.sort();
            names.dedup();
            if names.is_empty() {
                continue;
            }
            let src = package
                .manifest_path
                .as_deref()
                .and_then(|m| Path::new(m).parent())
                .map(|dir| dir.join("src"));
            let src = match src {
                Some(src) if src.is_dir() => src,
                _ if !require_sources => continue,
                _ => return Err(format!("{}: cannot find its src/ directory to scan", package.name)),
            };
            for file in rust_files(&src)? {
                let text = std::fs::read_to_string(&file)
                    .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
                let lines = identifier_lines(&text, &names).map_err(|e| {
                    format!("{}: {e}; it cannot be checked for a use of validation (R-187)", file.display())
                })?;
                for line in lines {
                    violations.push(SourceViolation { krate: package.name.clone(), file: file.clone(), line });
                }
            }
        }
        Ok(violations)
    }

    /// The workspace edges: each dependency of a workspace member that resolves to another workspace member
    /// (not merely one with a member's name; see `Dependency::is_on`).
    pub fn edges(&self) -> Result<Vec<Edge>, String> {
        let members = self.members();
        let is_member = |dep: &Dependency| members.iter().any(|m| dep.is_on(m));
        let mut edges = Vec::new();
        for package in &members {
            for dep in package.dependencies.iter().filter(|d| is_member(d)) {
                let kind = match dep.kind.as_deref() {
                    None => DepKind::Normal,
                    Some("dev") => DepKind::Dev,
                    Some("build") => DepKind::Build,
                    Some(other) => {
                        return Err(format!(
                            "{} → {}: unknown dependency kind {other:?}",
                            package.name, dep.name
                        ))
                    }
                };
                edges.push(Edge { from: package.name.clone(), to: dep.name.clone(), kind });
            }
        }
        Ok(edges)
    }
}

/// The crates in which no source under `src/` may use `validation` (systems_architecture §7.1; R-187).
pub const NO_VALIDATION_IN_SRC: &[&str] = &["kernel", "ledger"];

/// A use of `validation` in a source under `src/` of a crate in `NO_VALIDATION_IN_SRC`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceViolation {
    pub krate: String,
    pub file: PathBuf,
    /// 1-based.
    pub line: usize,
}

impl fmt::Display for SourceViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "forbidden use of validation in {} src/ at {}:{}: in kernel and ledger a test that uses \
             validation is an integration test (tests/), not a unit test in src/ (systems_architecture §7.1; \
             R-187). Any identifier named validation (or the dependency's rename) outside comments and literals \
             counts, a local item with that name included: without name resolution `validation::x` cannot be told \
             apart from the crate, so rename the local item",
            self.krate,
            self.file.display(),
            self.line
        )
    }
}

/// Every `.rs` file under `dir`, recursively, sorted.
fn rust_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries =
            std::fs::read_dir(&dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
        for entry in entries {
            let path = entry.map_err(|e| format!("cannot read {}: {e}", dir.display()))?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

/// The 1-based lines on which an identifier in `names` appears in `text`, outside comments and string, char and
/// byte literals, inside macro bodies and attributes too (a raw identifier `r#name` counts). `text` is lexed with
/// `proc-macro2`; a file that does not lex is an error, so it is never passed unscanned.
///
/// Every such identifier counts as a use of the crate, a local item with the same name included (`mod validation`,
/// `fn validation`, an associated item `<T as Tr>::validation`). This over-approximates R-187 on purpose: from
/// edition 2018 `validation::x` may name a local module or the extern crate, and only name resolution can tell
/// them apart, so a check without it that must never pass a real use has to fail both. Rename the local item.
fn identifier_lines(text: &str, names: &[String]) -> Result<Vec<usize>, String> {
    fn walk(stream: TokenStream, names: &[String], lines: &mut Vec<usize>) {
        for tree in stream {
            match tree {
                TokenTree::Ident(ident) => {
                    let word = ident.to_string();
                    if names.iter().any(|n| *n == word.strip_prefix("r#").unwrap_or(&word)) {
                        lines.push(ident.span().start().line);
                    }
                }
                TokenTree::Group(group) => walk(group.stream(), names, lines),
                TokenTree::Punct(_) | TokenTree::Literal(_) => {}
            }
        }
    }
    let stream = TokenStream::from_str(text).map_err(|e| format!("cannot lex it: {e}"))?;
    let mut lines = Vec::new();
    walk(stream, names, &mut lines);
    lines.sort_unstable();
    lines.dedup();
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::identifier_lines;

    fn lines(src: &str) -> Vec<usize> {
        identifier_lines(src, &["validation".to_owned()]).unwrap()
    }

    #[test]
    fn identifier_lines_finds_every_identifier_with_the_name() {
        let src = "use validation::Harness;\n\
                   extern crate validation;\n\
                   fn g() -> u8 { <u8 as Tr>::validation::X }\n\
                   mod validation {}\n\
                   fn f() { vec![r#validation::X]; }\n\
                   #[validation::attr] fn h() {}\n\
                   fn t(c: char, d: u8) -> bool { 'a' < c && d > ::validation::LIMIT }\n";
        assert_eq!(lines(src), vec![1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn identifier_lines_ignores_comments_and_literals() {
        // Control for the test above: the name in places that are not identifiers.
        let src = "// validation::run();\n\
                   /* use validation; */\n\
                   /// validation in a doc comment\n\
                   const S: &str = \"validation::run\";\n\
                   const R: &str = r#\"use validation;\"#;\n\
                   const C: &core::ffi::CStr = cr#\"a\"validation\"#;\n\
                   const B: &[u8] = b\"validation\";\n\
                   fn f() -> char { 'v' }\n";
        assert_eq!(lines(src), Vec::<usize>::new());
        assert_eq!(lines("\n\nfn f() { validation(); }"), vec![3]);
    }

    #[test]
    fn identifier_lines_fails_on_a_file_that_does_not_lex() {
        assert!(identifier_lines("fn f() { \"unterminated }", &["validation".to_owned()]).is_err());
        // Control: the same file, terminated.
        assert!(identifier_lines("fn f() { \"terminated\" }", &["validation".to_owned()]).is_ok());
    }
}
