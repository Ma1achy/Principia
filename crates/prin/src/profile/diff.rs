//! `prin profile diff BASE NEW --threshold P%` (render_gui_spec § "Profiler", "What `prin profile diff` compares",
//! REQ-TOOL-119): for each scope, the p95 of its per-frame ms in BASE and in NEW; a regression where NEW's p95 is more
//! than P% above BASE's. It exits 1 when any scope regresses, 0 when none does, and 2 when a file cannot be read.
//!
//! The scopes: the frame (`frame_ms`), each stage by its key (`stage_ms`, in the frames where it is not null), each CPU
//! scope by its stage and the names from the stage down to it, and each GPU pass by its stage and name. A scope that
//! occurs more than once in a frame gives that frame the sum of its ms; a frame where it does not occur gives no
//! sample. A scope in only one file is listed, not compared, and is not a regression.

use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::path::Path;
use std::process::ExitCode;

use engine::contract::profile::{self, percentile, Scope, Stage, Trace};

/// One scope of the diff.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    /// The frame: `frame_ms`.
    Frame,
    /// A stage: its `stage_ms`.
    Stage(&'static str),
    /// A CPU scope: its stage, then the names from the stage down to it.
    Scope(&'static str, Vec<String>),
    /// A GPU pass: its stage and name.
    GpuPass(&'static str, String),
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Frame => write!(f, "frame"),
            Key::Stage(stage) => write!(f, "stage {stage}"),
            Key::Scope(stage, path) => write!(f, "scope {stage}/{}", path.join("/")),
            Key::GpuPass(stage, name) => write!(f, "gpu pass {stage}/{name}"),
        }
    }
}

/// Parses `--threshold`: a percentage, `5%` or `5`, finite and ≥ 0.
pub(crate) fn parse_threshold(text: &str) -> Result<f64, String> {
    let number = text.strip_suffix('%').unwrap_or(text);
    let p: f64 = number
        .parse()
        .map_err(|_| format!("{text:?} is not a percentage such as 5%"))?;
    if p.is_finite() && p >= 0.0 {
        Ok(p)
    } else {
        Err(format!("{text:?} is not a percentage ≥ 0"))
    }
}

/// Each scope's samples: one per frame it occurs in.
fn samples(trace: &Trace) -> BTreeMap<Key, Vec<f64>> {
    let mut all: BTreeMap<Key, Vec<f64>> = BTreeMap::new();
    for frame in &trace.frames {
        let mut this: BTreeMap<Key, f64> = BTreeMap::new();
        this.insert(Key::Frame, frame.frame_ms);
        let ms = &frame.stage_ms;
        let stage_ms = [
            Some(ms.integrate),
            Some(ms.reduce),
            Some(ms.colour),
            Some(ms.upload),
            ms.present,
        ];
        for (stage, value) in Stage::ALL.into_iter().zip(stage_ms) {
            if let Some(value) = value {
                this.insert(Key::Stage(stage.key()), value);
            }
            let Some(sections) = frame.stages.get(stage) else {
                continue;
            };
            let mut path = Vec::new();
            add_scopes(stage.key(), &sections.scopes, &mut path, &mut this);
            for pass in &sections.gpu_passes {
                *this
                    .entry(Key::GpuPass(stage.key(), pass.name.clone()))
                    .or_insert(0.0) += pass.ms;
            }
        }
        for (key, value) in this {
            all.entry(key).or_default().push(value);
        }
    }
    all
}

fn add_scopes(
    stage: &'static str,
    scopes: &[Scope],
    path: &mut Vec<String>,
    this: &mut BTreeMap<Key, f64>,
) {
    for scope in scopes {
        path.push(scope.name.clone());
        *this.entry(Key::Scope(stage, path.clone())).or_insert(0.0) += scope.ms;
        add_scopes(stage, &scope.children, path, this);
        path.pop();
    }
}

/// Each scope's p95 (nearest rank, [`percentile`]).
fn p95s(trace: &Trace) -> BTreeMap<Key, f64> {
    samples(trace)
        .into_iter()
        .filter_map(|(key, s)| Some((key, percentile(&s, 95)?)))
        .collect()
}

/// What the diff found.
pub(crate) struct Report {
    /// The text printed.
    pub(crate) text: String,
    /// Whether any scope regressed.
    pub(crate) regressed: bool,
}

/// Whether NEW's p95 `new` regresses on BASE's `base` by more than `threshold` percent. From a BASE p95 of 0, any
/// rise is one.
fn regresses(base: f64, new: f64, threshold: f64) -> bool {
    if base > 0.0 {
        (new - base) / base * 100.0 > threshold
    } else {
        new > base
    }
}

/// Compares NEW's per-scope p95 with BASE's at `threshold` percent.
pub(crate) fn compare(base: &Trace, new: &Trace, threshold: f64) -> Report {
    let base = p95s(base);
    let new = p95s(new);
    let mut text = format!("p95 per scope, ms; a regression is a rise of more than {threshold}%\n");
    let mut regressed = false;
    for (key, b) in &base {
        let Some(n) = new.get(key) else {
            text.push_str(&format!("{key}: only in BASE, not compared\n"));
            continue;
        };
        let change = if *b > 0.0 {
            format!("{:+.2}%", (n - b) / b * 100.0)
        } else {
            "from 0".to_owned()
        };
        let flag = if regresses(*b, *n, threshold) {
            regressed = true;
            "  REGRESSION"
        } else {
            ""
        };
        text.push_str(&format!("{key}: {b} -> {n} ({change}){flag}\n"));
    }
    for key in new.keys().filter(|k| !base.contains_key(k)) {
        text.push_str(&format!("{key}: only in NEW, not compared\n"));
    }
    text.push_str(if regressed {
        "regression\n"
    } else {
        "no regression\n"
    });
    Report { text, regressed }
}

fn read(path: &Path) -> Result<Trace, String> {
    let file = File::open(path)
        .map_err(|e| format!("prin profile diff: cannot open {}: {e}", path.display()))?;
    profile::read(file).map_err(|e| {
        format!(
            "prin profile diff: {} is not profiler schema v1: {e}",
            path.display()
        )
    })
}

/// `prin profile diff BASE NEW --threshold P%`.
pub(crate) fn main(base: &Path, new: &Path, threshold: f64) -> Result<ExitCode, String> {
    let report = compare(&read(base)?, &read(new)?, threshold);
    print!("{}", report.text);
    Ok(if report.regressed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}
