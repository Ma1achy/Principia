//! `cargo xtask deps` — checks the workspace crate graph against the allowed-edge table of
//! systems_architecture §7.1, the crate map (R-170, confirmed by R-185). REQ-SYS-004.
//!
//! Edges *inside* one crate (decoder before kernel, canonicalise before the integrator) are not visible to
//! a crate-graph check; they stay a code-review item (systems_architecture §7.1).
//!
//! It also enforces R-187's condition on the `validation` dev-dependency: in `kernel` and `ledger`, a test that
//! uses `validation` is an integration test (`tests/`), not a unit test in `src/`, because the dev-dependency
//! cycle would give unit tests two copies of the crate. The check compiles (R-191, `compile_check`): when either
//! crate takes `validation` as a dev-dependency, a copy of the workspace without that dev-dependency must pass
//! `cargo check -p kernel -p ledger --lib --tests` with `--no-default-features`, with default features and with
//! `--all-features`, each in the dev and the release profile (R-194), so a use by a unit test fails whatever its route
//! (an alias, a macro, `#[path]`, `include!`). Its known limits: other platforms (R-193) and other cfg combinations
//! (R-194); doctests are not compiled, and may use `validation` (R-194). Both crates must also keep their library and
//! binary targets under `src/`.

use std::fmt;
use std::path::{Component, Path, PathBuf};
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
    AllowedEdge {
        from: "kernel",
        to: "ledger",
        kinds: Kinds::BuildOnly,
    },
    // §7: layout table → pack/unpack gen (WGSL), debug catalogue gen.
    AllowedEdge {
        from: "render",
        to: "ledger",
        kinds: Kinds::Any,
    },
    // §7: kernel → dispatch; chart system → validation → resolve/lowering.
    AllowedEdge {
        from: "engine",
        to: "ledger",
        kinds: Kinds::Any,
    },
    AllowedEdge {
        from: "engine",
        to: "kernel",
        kinds: Kinds::Any,
    },
    // §7: payload → fragment assembly (the frame loop and dispatch drive the fragment side).
    AllowedEdge {
        from: "engine",
        to: "render",
        kinds: Kinds::Any,
    },
    // GUI → state → engine (gui_state_contract §1).
    AllowedEdge {
        from: "gui",
        to: "engine",
        kinds: Kinds::Any,
    },
    // The CLI depends on engine.
    AllowedEdge {
        from: "prin",
        to: "engine",
        kinds: Kinds::Any,
    },
    // validation → any of the above except gui and prin: the harness exercises each seam; where it needs
    // the CLI it runs the built `prin` binary as a separate process (R-187). So no validation → prin.
    AllowedEdge {
        from: "validation",
        to: "kernel",
        kinds: Kinds::Any,
    },
    AllowedEdge {
        from: "validation",
        to: "ledger",
        kinds: Kinds::Any,
    },
    AllowedEdge {
        from: "validation",
        to: "render",
        kinds: Kinds::Any,
    },
    AllowedEdge {
        from: "validation",
        to: "engine",
        kinds: Kinds::Any,
    },
    // any except gui → validation, dev-dependency only (R-176, R-187). Never a normal or build dependency,
    // so the no_std kernel and rust-gpu builds never see it.
    AllowedEdge {
        from: ANY_EXCEPT_GUI,
        to: "validation",
        kinds: Kinds::DevOnly,
    },
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
        .filter_map(|edge| {
            rule_broken(edge).map(|rule| Violation {
                edge: edge.clone(),
                rule,
            })
        })
        .collect()
}

fn rule_broken(edge: &Edge) -> Option<&'static str> {
    let (from, to) = (edge.from.as_str(), edge.to.as_str());
    if to == "gui" {
        return Some("nothing depends on gui (systems_architecture §7.1; gui_state_contract §1)");
    }
    if from == "gui" && to == "validation" {
        return Some(
            "gui never depends on validation, in any kind (systems_architecture §7.1; R-187)",
        );
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
    /// `cargo metadata` always writes it; the minimal test fixtures may not.
    #[serde(default)]
    pub workspace_root: Option<String>,
    /// `cargo metadata` always writes it; the minimal test fixtures may not.
    #[serde(default)]
    pub target_directory: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Package {
    pub name: String,
    pub id: String,
    /// The package's `Cargo.toml`. `cargo metadata` always writes it; the minimal test fixtures may not.
    #[serde(default)]
    pub manifest_path: Option<String>,
    pub dependencies: Vec<Dependency>,
    /// The package's targets. `cargo metadata` always writes them; the minimal test fixtures may not.
    #[serde(default)]
    pub targets: Vec<Target>,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub kind: Vec<String>,
    pub src_path: String,
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
        Self::from_cargo_at(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml"))
    }

    /// Runs `cargo metadata --format-version 1` on the workspace of `manifest`.
    pub fn from_cargo_at(manifest: &Path) -> Result<Self, String> {
        let output = Command::new(cargo())
            .args(["metadata", "--format-version", "1", "--manifest-path"])
            .arg(manifest)
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
        self.packages
            .iter()
            .filter(|p| self.workspace_members.contains(&p.id))
            .collect()
    }

    /// The members of `NO_VALIDATION_IN_SRC` (kernel, ledger), in the order `cargo metadata` lists them.
    fn no_validation_in_src(&self) -> Vec<&Package> {
        self.members()
            .into_iter()
            .filter(|m| NO_VALIDATION_IN_SRC.contains(&m.name.as_str()))
            .collect()
    }

    /// Fails when a library or binary target of kernel or ledger has its root outside the crate's `src/`
    /// (systems_architecture §7.1; R-187, R-191). `require_sources`: whether each crate must have its manifest path
    /// and targets in the metadata, as the live workspace does; a fixture may describe a graph without them, and then
    /// the crate is skipped.
    pub fn check_targets(&self, require_sources: bool) -> Result<(), String> {
        for package in self.no_validation_in_src() {
            let Some(dir) = package
                .manifest_path
                .as_deref()
                .and_then(|m| Path::new(m).parent())
            else {
                if require_sources {
                    return Err(format!(
                        "{}: cargo metadata gives no manifest_path",
                        package.name
                    ));
                }
                continue;
            };
            package.targets_under(&dir.join("src"), require_sources)?;
        }
        Ok(())
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
                edges.push(Edge {
                    from: package.name.clone(),
                    to: dep.name.clone(),
                    kind,
                });
            }
        }
        Ok(edges)
    }

    /// Each dependency of kernel or ledger on the workspace's `validation` that is a dev-dependency, with its crate.
    fn validation_dev_dependencies(&self) -> Vec<(&Package, &Dependency)> {
        let members = self.members();
        let Some(validation) = members.iter().find(|m| m.name == "validation") else {
            return Vec::new();
        };
        self.no_validation_in_src()
            .into_iter()
            .flat_map(|p| p.dependencies.iter().map(move |d| (p, d)))
            .filter(|(_, d)| d.kind.as_deref() == Some("dev") && d.is_on(validation))
            .collect()
    }
}

impl Package {
    /// Fails when a library or binary target of the package (the targets with unit tests; `tests/`, examples,
    /// benches and the build script are not ones) has its root outside `src`.
    fn targets_under(&self, src: &Path, require_sources: bool) -> Result<(), String> {
        let with_unit_tests = |t: &&Target| {
            t.kind
                .iter()
                .any(|k| !matches!(k.as_str(), "test" | "example" | "bench" | "custom-build"))
        };
        let mut found = false;
        for target in self.targets.iter().filter(with_unit_tests) {
            found = true;
            if !normalize(Path::new(&target.src_path)).starts_with(src) {
                return Err(format!(
                    "{}: its {} target is at {}, outside {}: kernel's and ledger's library and binary targets, \
                     whose unit tests may not use validation, sit under src/ (systems_architecture §7.1; R-187, \
                     R-191)",
                    self.name,
                    target.kind.join(", "),
                    target.src_path,
                    src.display()
                ));
            }
        }
        if !found && require_sources {
            return Err(format!(
                "{}: cargo metadata gives no library or binary target to check",
                self.name
            ));
        }
        Ok(())
    }
}

/// The crates in which no unit test may use `validation` (systems_architecture §7.1; R-187).
pub const NO_VALIDATION_IN_SRC: &[&str] = &["kernel", "ledger"];

/// Where, under the workspace's target directory, the compile check keeps its own target directory: stable, so that
/// the check is incremental, and separate, so that it never contends with the build that runs `xtask`.
pub const CHECK_TARGET_DIR: &str = "xtask-deps-check";

/// What `compile_check` did when it did not fail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompileCheck {
    /// Neither kernel nor ledger takes `validation` as a dev-dependency, so no unit test of theirs can use it.
    NotNeeded,
    /// The copy without the dev-dependency compiled; these crates took it.
    Passed(Vec<String>),
}

/// R-187's condition, checked by compiling (R-191). When kernel or ledger takes `validation` as a dev-dependency, the
/// workspace is copied to a temporary directory (`copy_workspace`), `validation` is removed from their
/// dev-dependencies there, and `cargo check -p kernel -p ledger --lib --tests --offline` must pass in each feature set and profile, with
/// `CARGO_TARGET_DIR` at `CHECK_TARGET_DIR` under the workspace's target directory. The copy also leaves out their
/// integration-test, example and bench targets: those are not unit tests, and an integration test may use
/// `validation` (R-187), so only the library and binary targets remain for `--tests` to compile in test mode; each has
/// `test = true` there, so a `test = false` one is compiled in test mode too (`strip_manifest`). A use
/// of `validation` by a unit test, by any route, then fails to compile, and the error carries the compiler's output.
/// The check runs six times, stopping at the first failure (R-194, amending R-192): `--no-default-features`, default
/// features and `--all-features` (`FEATURE_SETS`), each in the dev and the release profile (`PROFILES`). A unit test
/// behind a feature, behind a feature's absence, or behind the release profile is compiled too.
/// Known limits: the check builds for the host target, so a unit test gated on another platform is not seen (R-193);
/// any other cfg combination (for example a test gated on feature `a` on and `b` off) is not seen either (R-194).
/// Doctests are not compiled: a kernel or ledger doctest may use `validation`, since rustdoc builds each one as a
/// separate crate that links the library from outside (R-194).
///
/// `--offline`: the check needs no package that the workspace's own build has not already fetched.
pub fn compile_check(metadata: &Metadata) -> Result<CompileCheck, String> {
    let deps = metadata.validation_dev_dependencies();
    if deps.is_empty() {
        return Ok(CompileCheck::NotNeeded);
    }
    let root = metadata
        .workspace_root
        .as_deref()
        .map(PathBuf::from)
        .ok_or("cargo metadata gives no workspace_root")?;
    let target = metadata
        .target_directory
        .as_deref()
        .map(PathBuf::from)
        .ok_or("cargo metadata gives no target_directory")?;
    let copy = std::env::temp_dir().join(format!(
        "xtask-deps-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    let result = check_copy(metadata, &deps, &root, &target, &copy);
    let _ = std::fs::remove_dir_all(&copy);
    result
}

fn check_copy(
    metadata: &Metadata,
    deps: &[(&Package, &Dependency)],
    root: &Path,
    target: &Path,
    copy: &Path,
) -> Result<CompileCheck, String> {
    let member_dirs: Vec<PathBuf> = metadata
        .members()
        .iter()
        .filter_map(|m| {
            Some(
                Path::new(m.manifest_path.as_deref()?)
                    .parent()?
                    .to_path_buf(),
            )
        })
        .collect();
    copy_workspace(
        root,
        copy,
        &member_dirs,
        &[target.to_path_buf(), root.join(".git")],
    )?;
    let crates = metadata.no_validation_in_src();
    for package in &crates {
        let manifest = package
            .manifest_path
            .as_deref()
            .ok_or_else(|| format!("{}: no manifest_path", package.name))?;
        let relative = Path::new(manifest).strip_prefix(root).map_err(|_| {
            format!(
                "{}: its manifest {manifest} is outside the workspace root",
                package.name
            )
        })?;
        // The name each dev-dependency on validation has in this manifest: its rename, or `validation`.
        let keys: Vec<&str> = deps
            .iter()
            .filter(|(p, _)| p.id == package.id)
            .map(|(_, d)| d.rename.as_deref().unwrap_or(&d.name))
            .collect();
        strip_manifest(&copy.join(relative), &keys)?;
    }
    let packages: Vec<String> = crates.iter().map(|p| format!("-p {}", p.name)).collect();
    for (profile, profile_args) in PROFILES {
        for (features, feature_args) in FEATURE_SETS {
            let mut command = Command::new(cargo());
            command
                .arg("check")
                .arg("--manifest-path")
                .arg(copy.join("Cargo.toml"));
            for package in &crates {
                command.args(["-p", &package.name]);
            }
            let output = command
                .args(["--lib", "--tests", "--offline"])
                .args(*feature_args)
                .args(*profile_args)
                .env("CARGO_TARGET_DIR", target.join(CHECK_TARGET_DIR))
                .output()
                .map_err(|e| format!("cannot run cargo check: {e}"))?;
            if output.status.success() {
                continue;
            }
            // The compiler names files in the copy; name them in the workspace.
            let copy_prefix = format!("{}{}", copy.display(), std::path::MAIN_SEPARATOR);
            let root_prefix = format!("{}{}", root.display(), std::path::MAIN_SEPARATOR);
            let stderr =
                String::from_utf8_lossy(&output.stderr).replace(&copy_prefix, &root_prefix);
            let flags: Vec<&str> = feature_args
                .iter()
                .chain(profile_args.iter())
                .copied()
                .collect();
            return Err(format!(
                "kernel and ledger do not compile without their validation dev-dependency with {features} in the \
                 {profile} profile, so a unit test in src/ uses validation, or they do not compile at all: in kernel \
                 and ledger a test that uses validation is an integration test (tests/), not a unit test in src/ \
                 (systems_architecture §7.1; R-187, R-191, R-194). `cargo check {} --lib --tests --offline{}`, on a \
                 copy of the workspace without that dev-dependency, says:\n{stderr}",
                packages.join(" "),
                flags.iter().map(|f| format!(" {f}")).collect::<String>()
            ));
        }
    }
    let mut names: Vec<String> = deps.iter().map(|(p, _)| p.name.clone()).collect();
    names.dedup();
    Ok(CompileCheck::Passed(names))
}

/// The feature sets the compile check builds (R-194): no features, the default ones, and all of them. A unit test
/// behind a feature, or behind a feature's absence (`not(feature = "…")`, whether that feature is default or not), is
/// compiled by one of them.
const FEATURE_SETS: &[(&str, &[&str])] = &[
    ("--no-default-features", &["--no-default-features"]),
    ("default features", &[]),
    ("--all-features", &["--all-features"]),
];

/// The profiles the compile check builds each feature set in (R-194): dev, and release (`--release`), so a unit test
/// behind `debug_assertions` or its absence is compiled. One `CARGO_TARGET_DIR` serves both; cargo keeps each
/// profile's output apart.
const PROFILES: &[(&str, &[&str])] = &[("dev", &[]), ("release", &["--release"])];

/// In the manifest at `path` (a copy): removes each of `keys` from every dev-dependency table (`[dev-dependencies]`
/// and each `[target.'…'.dev-dependencies]`), leaves out every integration-test, example and bench target
/// (`autotests`, `autoexamples` and `autobenches` off; `[[test]]`, `[[example]]` and `[[bench]]` removed), and sets
/// `test = true` on `[lib]` and on every `[[bin]]`. `--tests` selects only targets with `test = true`, and `--lib`
/// checks the library outside test mode, so a `test = false` target would keep its `#[cfg(test)]` code out of the
/// check, while `cargo test --lib` (or `--bin`) still builds that code with the dev-dependency. An auto-discovered
/// binary with no `[[bin]]` entry already has `test = true`. No other manifest key keeps a library's or binary's
/// `#[cfg(test)]` code out of `cargo check --lib --tests` (`harness = false`, `doctest = false`, `proc-macro` and
/// `crate-type` leave it in).
fn strip_manifest(path: &Path, keys: &[&str]) -> Result<(), String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let mut doc: toml_edit::DocumentMut = text
        .parse()
        .map_err(|e| format!("cannot parse {}: {e}", path.display()))?;
    let strip = |table: &mut toml_edit::Table| {
        for name in ["dev-dependencies", "dev_dependencies"] {
            if let Some(deps) = table
                .get_mut(name)
                .and_then(toml_edit::Item::as_table_like_mut)
            {
                for key in keys {
                    deps.remove(key);
                }
            }
        }
    };
    strip(doc.as_table_mut());
    if let Some(targets) = doc
        .get_mut("target")
        .and_then(toml_edit::Item::as_table_like_mut)
    {
        for (_, platform) in targets.iter_mut() {
            if let Some(platform) = platform.as_table_mut() {
                strip(platform);
            }
        }
    }
    for kind in ["test", "example", "bench"] {
        doc.remove(kind);
    }
    if let Some(lib) = doc
        .get_mut("lib")
        .and_then(toml_edit::Item::as_table_like_mut)
    {
        lib.insert("test", toml_edit::value(true));
    }
    match doc.get_mut("bin") {
        Some(toml_edit::Item::ArrayOfTables(bins)) => {
            for bin in bins.iter_mut() {
                bin.insert("test", toml_edit::value(true));
            }
        }
        Some(toml_edit::Item::Value(toml_edit::Value::Array(bins))) => {
            for bin in bins.iter_mut() {
                let bin = bin
                    .as_inline_table_mut()
                    .ok_or_else(|| format!("{}: a `bin` entry is not a table", path.display()))?;
                bin.insert("test", toml_edit::Value::from(true));
            }
        }
        Some(_) => {
            return Err(format!(
                "{}: `bin` is not an array of tables",
                path.display()
            ))
        }
        None => {}
    }
    let package = doc
        .get_mut("package")
        .and_then(toml_edit::Item::as_table_like_mut)
        .ok_or_else(|| format!("{}: no [package] table", path.display()))?;
    for auto in ["autotests", "autoexamples", "autobenches"] {
        package.insert(auto, toml_edit::value(false));
    }
    std::fs::write(path, doc.to_string())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// Copies the workspace at `root` to `to`, leaving out `skip` (the target directory, `.git`). Each member's directory
/// is copied whole, and so is each `Cargo.toml` and `Cargo.lock`; everything else, which cargo does not edit or read
/// as a manifest (the docs, a large archive), is linked, so that a file a member reaches outside its directory
/// (`include!("../../x.rs")`) is still there and the copy stays cheap.
fn copy_workspace(
    root: &Path,
    to: &Path,
    members: &[PathBuf],
    skip: &[PathBuf],
) -> Result<(), String> {
    let err = |p: &Path, e: std::io::Error| {
        format!("cannot copy {} to the check's workspace: {e}", p.display())
    };
    std::fs::create_dir_all(to).map_err(|e| err(to, e))?;
    for entry in std::fs::read_dir(root).map_err(|e| err(root, e))? {
        let entry = entry.map_err(|e| err(root, e))?;
        let (path, dest) = (entry.path(), to.join(entry.file_name()));
        let kind = entry.file_type().map_err(|e| err(&path, e))?;
        if skip.contains(&path) {
            continue;
        }
        if kind.is_dir() && members.contains(&path) {
            copy_tree(&path, &dest, skip)?;
        } else if kind.is_dir() && members.iter().any(|m| m.starts_with(&path)) {
            copy_workspace(&path, &dest, members, skip)?;
        } else if kind.is_file()
            && matches!(
                entry.file_name().to_str(),
                Some("Cargo.toml" | "Cargo.lock")
            )
        {
            std::fs::copy(&path, &dest).map_err(|e| err(&path, e))?;
        } else {
            link(&path, &dest).map_err(|e| err(&path, e))?;
        }
    }
    Ok(())
}

/// Copies the tree at `from` to `to`, leaving out `skip`; a symbolic link is copied as a link.
fn copy_tree(from: &Path, to: &Path, skip: &[PathBuf]) -> Result<(), String> {
    let err = |p: &Path, e: std::io::Error| {
        format!("cannot copy {} to the check's workspace: {e}", p.display())
    };
    std::fs::create_dir_all(to).map_err(|e| err(to, e))?;
    for entry in std::fs::read_dir(from).map_err(|e| err(from, e))? {
        let entry = entry.map_err(|e| err(from, e))?;
        let (path, dest) = (entry.path(), to.join(entry.file_name()));
        let kind = entry.file_type().map_err(|e| err(&path, e))?;
        if skip.contains(&path) {
            continue;
        }
        if kind.is_symlink() {
            let target = std::fs::read_link(&path).map_err(|e| err(&path, e))?;
            link(&target, &dest).map_err(|e| err(&path, e))?;
        } else if kind.is_dir() {
            copy_tree(&path, &dest, skip)?;
        } else {
            std::fs::copy(&path, &dest).map_err(|e| err(&path, e))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn link(target: &Path, dest: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, dest)
}

#[cfg(not(unix))]
fn link(target: &Path, dest: &Path) -> std::io::Result<()> {
    if target.is_dir() {
        copy_tree(target, dest, &[]).map_err(std::io::Error::other)
    } else {
        std::fs::copy(target, dest).map(|_| ())
    }
}

/// The cargo that runs `xtask`, or `cargo`.
fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

/// `path` with `.` and `..` components resolved lexically.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}
