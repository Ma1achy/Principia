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
    /// depends on `validation`, no `.rs` file under its `src/` uses `validation` (under the name it gives the
    /// dependency). A crate without that dependency cannot use the crate, so it is not scanned.
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
            let mut names: Vec<String> = package
                .dependencies
                .iter()
                .filter(|d| d.is_on(validation))
                .map(|d| d.rename.as_deref().unwrap_or(&d.name).replace('-', "_"))
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
                for name in &names {
                    for line in crate_uses(&text, name) {
                        violations.push(SourceViolation {
                            krate: package.name.clone(),
                            file: file.clone(),
                            line,
                        });
                    }
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
             R-187)",
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

#[derive(Debug, PartialEq)]
enum Token {
    Ident(String),
    PathSep,
    /// A multi-character operator with `<` or `>` in it (`->`, `=>`, `>=`, `<=`, `>>=`, `<<=`, `>>`, `<<`), kept
    /// whole so that its `>` is not read as the `>` that closes a generic or a qualified path.
    Op(&'static str),
    Punct(char),
}

/// The multi-character operators lexed as one `Token::Op`, longest first.
const ANGLE_OPS: [&str; 8] = [">>=", "<<=", "->", "=>", ">=", "<=", ">>", "<<"];

/// Splits Rust source into identifiers, `::`, the operators in `ANGLE_OPS` and other punctuation, each with its
/// 1-based line. Comments, string and character literals are skipped, so a mention there is not a use.
fn tokens(text: &str) -> Vec<(Token, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let (mut i, mut line) = (0, 1);
    let at = |i: usize| chars.get(i).copied().unwrap_or('\0');
    // Skips to just past the closing `"` followed by `hashes` `#`s, counting lines.
    let skip_string = |mut i: usize, line: &mut usize, hashes: usize, escapes: bool| -> usize {
        while i < chars.len() {
            match chars[i] {
                '\\' if escapes => i += 1,
                '\n' => *line += 1,
                '"' if (1..=hashes).all(|k| at(i + k) == '#') => return i + 1 + hashes,
                _ => {}
            }
            i += 1;
        }
        i
    };
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == '/' && at(i + 1) == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && at(i + 1) == '*' {
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '/' && at(i + 1) == '*' {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && at(i + 1) == '/' {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    if chars[i] == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
        } else if c == '"' {
            i = skip_string(i + 1, &mut line, 0, true);
        } else if c == '\'' {
            // A character literal ('x', '\n', '\u{..}'), or a lifetime ('a), which is skipped as punctuation.
            if at(i + 1) == '\\' {
                i += 2;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                i += 1;
            } else if at(i + 2) == '\'' {
                i += 3;
            } else {
                i += 1;
            }
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            // Raw and byte string prefixes: r"…", r#"…"#, b"…", br"…".
            if matches!(word.as_str(), "r" | "br" | "b") && (at(i) == '"' || (word != "b" && at(i) == '#')) {
                let mut hashes = 0;
                while at(i + hashes) == '#' {
                    hashes += 1;
                }
                if at(i + hashes) == '"' {
                    i = skip_string(i + hashes + 1, &mut line, hashes, word == "b");
                    continue;
                }
            }
            out.push((Token::Ident(word), line));
        } else if c == ':' && at(i + 1) == ':' {
            out.push((Token::PathSep, line));
            i += 2;
        } else if let Some(op) = ANGLE_OPS.iter().find(|op| op.chars().enumerate().all(|(k, o)| at(i + k) == o)) {
            out.push((Token::Op(op), line));
            i += op.len();
        } else {
            out.push((Token::Punct(c), line));
            i += 1;
        }
    }
    out
}

/// The lines on which `text` uses the external crate `name`: as a path root (`name::…` or `::name::…`), in
/// `use name` or in `extern crate name`. `a::name::…` names a module of `a`, not the crate, and is not a use.
fn crate_uses(text: &str, name: &str) -> Vec<usize> {
    let toks = tokens(text);
    let ident = |k: Option<usize>, w: &str| {
        k.and_then(|k| toks.get(k)).is_some_and(|(t, _)| *t == Token::Ident(w.to_owned()))
    };
    let is_ident = |k: Option<usize>| k.and_then(|k| toks.get(k)).is_some_and(|(t, _)| matches!(t, Token::Ident(_)));
    let is = |k: Option<usize>, want: &Token| k.and_then(|k| toks.get(k)).is_some_and(|(t, _)| t == want);
    let mut lines = Vec::new();
    for (k, (tok, line)) in toks.iter().enumerate() {
        if *tok != Token::Ident(name.to_owned()) {
            continue;
        }
        let prev = k.checked_sub(1);
        let prev2 = k.checked_sub(2);
        let path_root = is(Some(k + 1), &Token::PathSep)
            && (!is(prev, &Token::PathSep) || !(is_ident(prev2) || prev2.is_some_and(|j| closes_angle(&toks, j))));
        let used = path_root || ident(prev, "use") || (ident(prev, "crate") && ident(prev2, "extern"));
        if used && lines.last() != Some(line) {
            lines.push(*line);
        }
    }
    lines
}

/// Whether the token at `j` (`>` or `>>`) closes a `<` opened earlier in the same statement, as in
/// `<T as Trait>::name` or `Vec::<Vec<u8>>::name`, so that the `::name` after it is an associated item, not the
/// crate. A `>` or `>>` with no such `<` is a comparison or a shift, and `::name` after it is the crate. `->`,
/// `=>`, `>=` and `>>=` are `Token::Op`s that close nothing. The walk back skips balanced `()` and `[]` and stops
/// at `;`, `{`, `}` or an unmatched `(` or `[`. It cannot tell a comparison `<` from a generic one, so in
/// `a < b && c > ::name::X` the `>` is read as closing the `<`.
fn closes_angle(toks: &[(Token, usize)], j: usize) -> bool {
    let angles = |t: &Token| match t {
        Token::Punct('>') => 1,
        Token::Op(">>") => 2,
        Token::Punct('<') => -1,
        Token::Op("<<") => -2,
        _ => 0,
    };
    let mut depth = angles(&toks[j].0);
    if depth <= 0 {
        return false;
    }
    let mut groups = 0usize;
    for (tok, _) in toks[..j].iter().rev() {
        match tok {
            Token::Punct(')' | ']') => groups += 1,
            Token::Punct('(' | '[') if groups > 0 => groups -= 1,
            Token::Punct('(' | '[' | ';' | '{' | '}') => return false,
            _ if groups > 0 => {}
            t => {
                depth += angles(t);
                if depth <= 0 {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::crate_uses;

    #[test]
    fn crate_uses_finds_path_roots_use_and_extern_crate() {
        let src = "use validation::Harness;\n\
                   fn f() { validation::run(); }\n\
                   extern crate validation;\n\
                   use validation as v;\n\
                   fn g() { ::validation::run(); }\n";
        assert_eq!(crate_uses(src, "validation"), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn crate_uses_ignores_comments_literals_and_other_paths() {
        // Control for the test above: the same name, in places that are not a use of the crate.
        let src = "// validation::run();\n\
                   /* use validation; */\n\
                   const S: &str = \"validation::run\";\n\
                   const R: &str = r#\"use validation;\"#;\n\
                   fn f() { crate::validation::run(); engine::validation::check(); }\n\
                   fn validation() {}\n\
                   fn g<'a>(x: &'a u8) -> char { let _ = validation(); 'v' }\n";
        assert_eq!(crate_uses(src, "validation"), Vec::<usize>::new());
        assert_eq!(crate_uses("fn f() { validation::run(); }", "validation"), vec![1]);
    }
}
