//! The numerical-gate runner (TASK-M0-05): `cargo xtask gate <gate>` runs a gate from `fixtures/gates/<gate>/` against
//! the threshold its requirement gives, or, where the corpus gives none, its calibration requirement's proposed value
//! (R-71), and writes a [`GateReport`] per input, as markdown and JSON, under `target/gates/` (philosophy §4.6).
//!
//! `fixtures/gates/<gate>/gate.json` names the gate, its inputs, its threshold and its region minimum:
//!
//! ```text
//! {
//!   "gate": "convergence",
//!   "inputs": ["converging.json"],
//!   "threshold": { "value": 0.1, "requirement": "REQ-VAL-135", "status": "provisional", "ruling": "R-171" },
//!   "min_regions": { "value": null, "requirement": "REQ-VAL-168" }
//! }
//! ```
//!
//! A threshold is refused unless it names a live requirement of `plan/requirements.yaml`; a provisional one, or a
//! region minimum, unless that requirement is a calibration requirement. Each input carries `"expected"`: `"pass"`,
//! `"fail"` or `"refused"`; the run fails naming each input whose outcome is not the expected one.

pub mod report;

use std::path::{Path, PathBuf};

use serde::Deserialize;

pub use report::{GateReport, Minimum, RegionCount, Regions, Verdict};

use crate::convergence::ConvergenceGate;
use crate::oklab::OklabRoundtripGate;

/// A numerical gate.
pub trait Gate: Sync {
    /// The gate's name: its directory under `fixtures/gates/`.
    fn name(&self) -> &'static str;

    /// Judges one input, `fixture` (its JSON, without `"expected"`), named `name`, against `config`; `Err` refuses it,
    /// giving the reason.
    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String>;
}

/// The registered gates, in the order `--all` runs them.
pub const REGISTRY: &[&dyn Gate] = &[&ConvergenceGate, &OklabRoundtripGate];

/// Whether a threshold is confirmed, or a calibration requirement's provisional value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Confirmed: given by the requirement, or confirmed by the human under a calibration requirement (R-71).
    Confirmed,
    /// Proposed, not yet confirmed; the requirement is a calibration requirement.
    Provisional,
}

impl Status {
    /// "confirmed" or "provisional".
    pub fn word(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Provisional => "provisional",
        }
    }
}

/// A gate's threshold and its source.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Threshold {
    /// The value.
    pub value: f64,
    /// The requirement, or calibration requirement, that gives it.
    #[serde(default)]
    pub requirement: String,
    /// Confirmed or provisional.
    pub status: Status,
    /// The ruling that set it, if any.
    #[serde(default)]
    pub ruling: Option<String>,
}

impl Threshold {
    /// "0.1, provisional (REQ-VAL-135, R-171)", or "0.1 (REQ-…)" when confirmed.
    pub fn describe(&self) -> String {
        let source = match &self.ruling {
            Some(ruling) => format!("{}, {ruling}", self.requirement),
            None => self.requirement.clone(),
        };
        match self.status {
            Status::Provisional => format!("{}, provisional ({source})", self.value),
            Status::Confirmed => format!("{} ({source})", self.value),
        }
    }
}

/// `gate.json`'s region minimum.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MinRegionsField {
    value: Option<u32>,
    #[serde(default)]
    requirement: String,
}

/// `gate.json` as written.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GateFile {
    gate: String,
    inputs: Vec<String>,
    threshold: Threshold,
    min_regions: MinRegionsField,
}

/// A gate's configuration, read from `fixtures/gates/<gate>/gate.json` and checked against the requirements.
#[derive(Debug, Clone)]
pub struct GateConfig {
    /// The gate's name.
    pub gate: String,
    /// The input files, relative to the gate's directory, in the order they run.
    pub inputs: Vec<String>,
    /// The threshold.
    pub threshold: Threshold,
    /// The region minimum.
    pub min_regions: Minimum,
}

/// A requirement's kind in `plan/requirements.yaml`: `None` for a plain requirement, else its `kind:`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// `calibration`, `definition`, or `None`.
    pub kind: Option<String>,
    /// Whether it is retired.
    pub retired: bool,
}

/// Looks `id` up in `requirements` (the text of `plan/requirements.yaml`).
pub fn find_requirement(requirements: &str, id: &str) -> Option<Requirement> {
    let mut lines = requirements.lines();
    lines.find(|line| line.strip_prefix("- id: ").map(str::trim) == Some(id))?;
    let mut found = Requirement {
        kind: None,
        retired: false,
    };
    for line in lines.take_while(|line| !line.starts_with("- ")) {
        if let Some(kind) = line.strip_prefix("  kind: ") {
            found.kind = Some(kind.trim().to_owned());
        }
        if line.starts_with("  retired:") {
            found.retired = true;
        }
    }
    Some(found)
}

/// Refuses `id` unless it is a live requirement, and, when `calibration`, a calibration requirement.
fn check_requirement(
    requirements: &str,
    what: &str,
    id: &str,
    calibration: bool,
) -> Result<(), String> {
    if id.trim().is_empty() {
        return Err(format!(
            "{what} names no requirement or calibration requirement; a numeric value needs one (R-71)"
        ));
    }
    let found = find_requirement(requirements, id)
        .ok_or_else(|| format!("{what} names {id}, which is not in plan/requirements.yaml"))?;
    if found.retired {
        return Err(format!(
            "{what} names {id}, which is retired and in no gate"
        ));
    }
    if calibration && found.kind.as_deref() != Some("calibration") {
        return Err(format!(
            "{what} names {id}, which is not a calibration requirement (R-71)"
        ));
    }
    Ok(())
}

impl GateConfig {
    /// Parses `text` (a `gate.json`) and checks its threshold and region minimum against `requirements` (the text of
    /// `plan/requirements.yaml`).
    pub fn parse(text: &str, requirements: &str) -> Result<Self, String> {
        let file: GateFile = serde_json::from_str(text).map_err(|e| format!("gate.json: {e}"))?;
        let t = &file.threshold;
        if !t.value.is_finite() {
            return Err(format!(
                "gate.json: the threshold {} is not finite",
                t.value
            ));
        }
        check_requirement(
            requirements,
            "gate.json: the threshold",
            &t.requirement,
            t.status == Status::Provisional,
        )?;
        if t.status == Status::Provisional && t.ruling.is_none() {
            return Err(format!(
                "gate.json: the provisional threshold names no ruling setting it, beside {}",
                t.requirement
            ));
        }
        let m = &file.min_regions;
        check_requirement(
            requirements,
            "gate.json: the region minimum",
            &m.requirement,
            true,
        )?;
        let min_regions = match m.value {
            Some(value) => Minimum::Declared {
                value,
                requirement: m.requirement.clone(),
            },
            None => Minimum::NotCalibrated {
                requirement: m.requirement.clone(),
            },
        };
        if file.inputs.is_empty() {
            return Err("gate.json: no inputs".to_owned());
        }
        Ok(Self {
            gate: file.gate,
            inputs: file.inputs,
            threshold: file.threshold,
            min_regions,
        })
    }
}

/// What an input is expected to give.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Expected {
    /// A passing report.
    Pass,
    /// A failing report.
    Fail,
    /// A refusal.
    Refused,
}

/// Where a run's reports go: `$CARGO_TARGET_DIR/gates`, or `<root>/target/gates`.
pub fn out_dir(root: &Path) -> PathBuf {
    match std::env::var_os("CARGO_TARGET_DIR") {
        Some(dir) => std::env::current_dir()
            .unwrap_or_default()
            .join(dir)
            .join("gates"),
        None => root.join("target/gates"),
    }
}

/// The registered gate named `name`.
pub fn gate(name: &str) -> Result<&'static dyn Gate, String> {
    REGISTRY
        .iter()
        .copied()
        .find(|g| g.name() == name)
        .ok_or_else(|| {
            let names: Vec<&str> = REGISTRY.iter().map(|g| g.name()).collect();
            format!(
                "no gate `{name}`; the registered gates: {}",
                names.join(", ")
            )
        })
}

/// Reads and checks `fixtures/gates/<gate>/gate.json` under `root`.
pub fn config(root: &Path, gate: &str) -> Result<GateConfig, String> {
    let path = root.join("fixtures/gates").join(gate).join("gate.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let requirements_path = root.join("plan/requirements.yaml");
    let requirements = std::fs::read_to_string(&requirements_path)
        .map_err(|e| format!("cannot read {}: {e}", requirements_path.display()))?;
    let config =
        GateConfig::parse(&text, &requirements).map_err(|e| format!("{}: {e}", path.display()))?;
    if config.gate != gate {
        return Err(format!(
            "{} names gate `{}`, not `{gate}`",
            path.display(),
            config.gate
        ));
    }
    Ok(config)
}

/// Runs the gate `name` on each of its inputs under `root`, writing reports under `out`, and printing each outcome.
/// Fails naming each input whose outcome is not its expected one.
pub fn run(root: &Path, name: &str, out: &Path) -> Result<(), String> {
    let gate = gate(name)?;
    let config = config(root, name)?;
    let dir = root.join("fixtures/gates").join(name);
    println!("gate {name}: threshold {}", config.threshold.describe());
    let mut wrong = Vec::new();
    for input in &config.inputs {
        let path = dir.join(input);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let mut value: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let expected = value
            .as_object_mut()
            .and_then(|o| o.remove("expected"))
            .ok_or_else(|| {
                format!(
                    "{}: no \"expected\" (pass, fail or refused)",
                    path.display()
                )
            })?;
        let expected: Expected = serde_json::from_value(expected)
            .map_err(|e| format!("{}: \"expected\": {e}", path.display()))?;
        let outcome = match gate.run(&config, input, &value) {
            Ok(report) => {
                let (md, js) = report.write(out)?;
                println!(
                    "gate {name}: {input}: {} (expected {})",
                    report.verdict.word().to_uppercase(),
                    word(expected)
                );
                for line in report.negative_results.iter().chain(&report.scatter) {
                    println!("  {line}");
                }
                if let Some(regions) = &report.regions {
                    for line in regions.lines() {
                        println!("  {line}");
                    }
                    for flag in regions.flags() {
                        println!("  flagged: {flag}");
                    }
                }
                println!("  report: {} and {}", md.display(), js.display());
                match report.verdict {
                    Verdict::Pass => Expected::Pass,
                    Verdict::Fail => Expected::Fail,
                }
            }
            Err(reason) => {
                println!(
                    "gate {name}: {input}: REFUSED (expected {}): {reason}",
                    word(expected)
                );
                Expected::Refused
            }
        };
        if outcome != expected {
            wrong.push(format!(
                "{input} gave {}, expected {}",
                word(outcome),
                word(expected)
            ));
        }
    }
    if wrong.is_empty() {
        println!("gate {name}: every input gave its expected outcome");
        Ok(())
    } else {
        Err(format!("gate {name}: {}", wrong.join("; ")))
    }
}

fn word(expected: Expected) -> &'static str {
    match expected {
        Expected::Pass => "pass",
        Expected::Fail => "fail",
        Expected::Refused => "refused",
    }
}

/// Runs every registered gate, in order; fails naming each gate that failed.
pub fn run_all(root: &Path, out: &Path) -> Result<(), String> {
    let failed: Vec<String> = REGISTRY
        .iter()
        .filter_map(|g| run(root, g.name(), out).err())
        .collect();
    if failed.is_empty() {
        Ok(())
    } else {
        Err(failed.join("\n"))
    }
}

/// Lists every registered gate with its threshold and inputs, running none; fails on a gate whose `gate.json` is
/// refused.
pub fn list(root: &Path) -> Result<(), String> {
    for g in REGISTRY {
        let config = config(root, g.name())?;
        println!(
            "gate {}: threshold {}; inputs: {}",
            g.name(),
            config.threshold.describe(),
            config.inputs.join(", ")
        );
    }
    Ok(())
}
