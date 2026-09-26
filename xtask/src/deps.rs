//! `cargo xtask deps` — checks the workspace crate graph against the allowed-edge table of
//! systems_architecture §7.1, the crate map (R-170, confirmed by R-185). REQ-SYS-004.
//!
//! Edges *inside* one crate (decoder before kernel, canonicalise before the integrator) are not visible to
//! a crate-graph check; they stay a code-review item (systems_architecture §7.1).

use std::fmt;
use std::path::Path;
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

/// `from` for the "any (dev-dependency only) → validation" row: every workspace crate but `validation`.
const ANY: &str = "*";

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
    // validation → any of the above except gui: the harness exercises each seam.
    AllowedEdge { from: "validation", to: "kernel", kinds: Kinds::Any },
    AllowedEdge { from: "validation", to: "ledger", kinds: Kinds::Any },
    AllowedEdge { from: "validation", to: "render", kinds: Kinds::Any },
    AllowedEdge { from: "validation", to: "engine", kinds: Kinds::Any },
    // Whether `prin` is among "any of the above" is open (RQ-129); allowed until it is ruled.
    AllowedEdge { from: "validation", to: "prin", kinds: Kinds::Any },
    // any → validation, dev-dependency only (R-176).
    AllowedEdge { from: ANY, to: "validation", kinds: Kinds::DevOnly },
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
    // Reading B of RQ-129 (pending a ruling): §7.1's "ledger depends on nothing, kernel on nothing but
    // ledger" is applied ahead of its "any (dev-dependency only) → validation" row.
    if (from == "ledger" || from == "kernel") && to == "validation" {
        return Some(
            "ledger and kernel take no workspace dependency on validation, not even a dev-dependency \
             (systems_architecture §7.1; the reading applied until RQ-129 is ruled)",
        );
    }
    if from == "ledger" {
        return Some("ledger is a root: it has no workspace dependency (systems_architecture §7.1)");
    }
    if from == "kernel" && to != "ledger" {
        return Some("kernel depends on no workspace crate but ledger (systems_architecture §7.1)");
    }
    if from == "kernel" && to == "ledger" && edge.kind != DepKind::Build {
        return Some("kernel → ledger is a build-dependency only (R-185)");
    }
    let allowed = ALLOWED.iter().any(|a| {
        let from_matches = a.from == from || (a.from == ANY && from != a.to);
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
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub kind: Option<String>,
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

    /// The workspace edges: each dependency of a workspace member on another workspace member.
    pub fn edges(&self) -> Result<Vec<Edge>, String> {
        let members: Vec<&Package> = self
            .packages
            .iter()
            .filter(|p| self.workspace_members.contains(&p.id))
            .collect();
        let is_member = |name: &str| members.iter().any(|p| p.name == name);
        let mut edges = Vec::new();
        for package in &members {
            for dep in package.dependencies.iter().filter(|d| is_member(&d.name)) {
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
