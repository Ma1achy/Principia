//! The gate `reference-convergence` (TASK-M3-25) and what [`super::brutus`] may not do itself: take the suite's starts,
//! and prove its own independence.
//!
//! **Adaptors** ([`from_published`], [`from_ic`]): a published start (velocities, its own `G`) and a kernel state
//! (`Ic<f64>`, momenta, `G = 1`) enter the reference with their own bits: a momentum is divided by its mass at the
//! rung's precision, never in f64, so a comparison with the kernel's march measures the march and not the start.
//!
//! **Independence** ([`independence`], [`kernel_closure`]). `validation` depends on `kernel`, so the crate graph cannot
//! show that `brutus.rs` does not; the check is on the file's source. Every path root in it must be `std`, `core`,
//! `alloc` or `dashu_float`, or a name the file itself declares, and it may not reach anything through `crate`,
//! `super`, an extern crate of `validation` (read from `crates/validation/Cargo.toml`, so a new dependency is forbidden
//! by default), a `mod` of its own, `#[path]`, `include!` or a macro not in the standard library's prelude. Comments
//! and string literals are stripped first, so a doc that names the kernel is not an import. Its limit: a macro's
//! expansion is not read, which is why every macro but the standard ones is refused. Beside it, [`kernel_closure`] holds
//! the arbitrary-precision crates out of `kernel`'s and `engine`'s normal and build dependencies, through
//! `cargo tree`: the reference shares no library with what it judges.
//!
//! **The gate's inputs** (`fixtures/gates/reference-convergence/`): `"kind": "converge"` runs a ladder on a named start
//! and asserts the stop rule, then a truth the start has independently of convergence (a convergent wrong force is still
//! wrong): the Lagrange triangle against its rigid rotation, the figure-eight's closure after its period. `"kind":
//! "independence"` runs the source scan and the closure check, optionally on the source with a line injected. The
//! threshold is the ladder's agreement, `10^−p`.

use std::path::Path;

use serde::Deserialize;

use kernel::decode::Ic;

use super::brutus::{self, Converged, Ladder, Motion, NotConverged, Start};
use crate::gate::{Gate, GateConfig, GateReport, RegionCount, Regions, Verdict};
use crate::ground_truth::tiers;
use crate::import;

/// The gate's name.
pub const NAME: &str = "reference-convergence";

/// The reference's source, relative to the workspace.
pub const SOURCE: &str = "crates/validation/src/reference/brutus.rs";

/// The crates the reference may name: the standard library's and its arbitrary-precision library.
pub const ALLOWED_ROOTS: &[&str] = &["std", "core", "alloc", "dashu_float"];

/// The arbitrary-precision library's crates, which `kernel` and `engine` may not depend on.
pub const PRECISION_CRATES: &[&str] = &["dashu-float", "dashu-int", "dashu-base"];

/// The macros the reference may invoke: the standard library's, which expand to nothing of this workspace.
pub const ALLOWED_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "format",
    "matches",
    "panic",
    "unreachable",
    "vec",
    "write",
    "writeln",
];

/// The default ladder: rungs of 12, 16, 20, … digits, up to ten, agreeing to `10⁻²⁰`, far below f64's rounding, so a
/// ladder held at f64's precision cannot pass it.
pub const LADDER: Ladder = Ladder {
    first: 12,
    step: 4,
    rungs: 10,
    agree: 20,
    max_steps: 200_000,
    fixed_bits: None,
};

/// A published start (its velocities and its `G`), as the reference takes it.
pub fn from_published(s: &import::Start) -> Start {
    Start {
        g: s.g,
        m: s.m,
        r: s.r,
        w: s.v,
        motion: Motion::Velocity,
    }
}

/// A kernel state (`G = 1`, momenta), as the reference takes it.
pub fn from_ic(x: &Ic<f64>) -> Start {
    Start {
        g: 1.0,
        m: x.m,
        r: x.r,
        w: x.p,
        motion: Motion::Momentum,
    }
}

// ── Independence ───────────────────────────────────────────────────────────────────────────────────────────────────

/// `source` with its comments, string and character literals blanked, so only code remains.
fn code_only(source: &str) -> String {
    let b: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let next = b.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            while i < b.len() {
                if b[i] == '/' && b.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if b[i] == '*' && b.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            out.push(' ');
        } else if c == 'r'
            && (next == Some('"') || next == Some('#'))
            && !ident_char(b.get(i.wrapping_sub(1)).copied())
        {
            // A raw string: r"…" or r#"…"#.
            let mut j = i + 1;
            let mut hashes = 0;
            while b.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if b.get(j) != Some(&'"') {
                out.push(c);
                i += 1;
                continue;
            }
            j += 1;
            'raw: while j < b.len() {
                if b[j] == '"' && (0..hashes).all(|h| b.get(j + 1 + h) == Some(&'#')) {
                    j += 1 + hashes;
                    break 'raw;
                }
                j += 1;
            }
            out.push_str("\"\"");
            i = j;
        } else if c == '"' {
            i += 1;
            while i < b.len() && b[i] != '"' {
                if b[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            out.push_str("\"\"");
        } else if c == '\'' {
            // A character literal ('x', '\n', '\u{…}'), or a lifetime ('a), which is kept.
            let close = if next == Some('\\') {
                (i + 2..b.len().min(i + 12)).find(|&j| b[j] == '\'')
            } else if b.get(i + 2) == Some(&'\'') {
                Some(i + 2)
            } else {
                None
            };
            match close {
                Some(j) => {
                    out.push_str("' '");
                    i = j + 1;
                }
                None => {
                    out.push(c);
                    i += 1;
                }
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

fn ident_char(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric() || c == '_')
}

/// The extern crates of `validation`, from its manifest's dependency tables (normal, dev, build, per target), as Rust
/// names (`-` read as `_`).
pub fn extern_crates(manifest: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_deps = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_deps = line.trim_end_matches(']').ends_with("dependencies");
            continue;
        }
        if !in_deps || line.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = line.split_once('=') {
            let name = name.trim();
            if !name.is_empty() {
                names.push(name.replace('-', "_"));
            }
        }
    }
    names
}

/// The source scan (the module's note): every way `source` reaches beyond `std`, `core`, `alloc` and `dashu_float`,
/// with `externs` the extern crates of the crate it compiles in. Empty when it reaches nothing else.
pub fn independence(source: &str, externs: &[String]) -> Vec<String> {
    let code = code_only(source);
    let chars: Vec<char> = code.chars().collect();
    let mut tokens: Vec<(String, usize)> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            tokens.push((chars[start..i].iter().collect(), start));
        } else {
            i += 1;
        }
    }
    let line_of = |at: usize| chars[..at].iter().filter(|&&c| c == '\n').count() + 1;
    let after = |at: usize, len: usize| -> String {
        chars[at + len..]
            .iter()
            .skip_while(|c| c.is_whitespace())
            .take(2)
            .collect()
    };
    let mut found = Vec::new();
    for (k, (word, at)) in tokens.iter().enumerate() {
        let next = after(*at, word.chars().count());
        let line = line_of(*at);
        let prev = k.checked_sub(1).map(|p| tokens[p].0.as_str());
        // A path's root: followed by `::`, and not a later segment (`a::kernel`), though a global path (`::kernel`) is one.
        let before: Vec<char> = chars[..*at]
            .iter()
            .rev()
            .filter(|c| !c.is_whitespace())
            .take(3)
            .copied()
            .collect();
        let later_segment = before.len() >= 2
            && before[0] == ':'
            && before[1] == ':'
            && before
                .get(2)
                .is_some_and(|&c| c.is_alphanumeric() || c == '_' || c == '>');
        let path_root = next == "::" && !later_segment;
        match word.as_str() {
            "crate" | "super" if next == "::" || prev == Some("use") || prev == Some("pub") => {
                found.push(format!(
                    "line {line}: `{word}`, which reaches the rest of the crate"
                ))
            }
            "mod"
                if next
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_alphabetic() || c == '_') =>
            {
                found.push(format!(
                    "line {line}: a `mod` declaration, whose file the scan does not read"
                ))
            }
            "include" | "include_str" | "include_bytes" if next.starts_with('!') => found.push(
                format!("line {line}: `{word}!`, which pastes in another file"),
            ),
            "path"
                if chars[..*at]
                    .iter()
                    .rev()
                    .take_while(|c| **c != '\n')
                    .any(|&c| c == '#') =>
            {
                found.push(format!("line {line}: a `#[path]` attribute"))
            }
            "extern" if after(*at, 6).starts_with("cr") => {
                found.push(format!("line {line}: `extern crate`"))
            }
            // A macro invocation: the name, `!` at once, then a delimiter (`if !x` and `a != b` are not).
            w if chars.get(at + w.chars().count()) == Some(&'!')
                && chars[at + w.chars().count() + 1..]
                    .iter()
                    .find(|c| !c.is_whitespace())
                    .is_some_and(|c| matches!(c, '(' | '[' | '{'))
                && !ALLOWED_MACROS.contains(&w) =>
            {
                found.push(format!(
                    "line {line}: the macro `{w}!`, whose expansion the scan does not read"
                ))
            }
            w if path_root && externs.iter().any(|e| e == w) && !ALLOWED_ROOTS.contains(&w) => {
                found.push(format!("line {line}: the extern crate `{w}`"))
            }
            w if prev == Some("use") && !ALLOWED_ROOTS.contains(&w) && w != "self" => found.push(
                format!("line {line}: `use {w}`, a root that is not std or dashu_float"),
            ),
            _ => {}
        }
    }
    found
}

/// The normal and build dependencies of `package`, by `cargo tree` on the workspace at `root`: each package name.
pub fn normal_closure(root: &Path, package: &str) -> Result<Vec<String>, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let out = std::process::Command::new(cargo)
        .current_dir(root)
        .args([
            "tree",
            "--quiet",
            "-p",
            package,
            "-e",
            "normal,build",
            "--prefix",
            "none",
            "--format",
            "{p}",
        ])
        .output()
        .map_err(|e| format!("cannot run cargo tree: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo tree -p {package} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_owned))
        .collect())
}

/// Which of `forbidden` are in `kernel`'s or `engine`'s normal and build dependencies: empty when none is.
pub fn kernel_closure(root: &Path, forbidden: &[&str]) -> Result<Vec<String>, String> {
    let mut found = Vec::new();
    for package in ["kernel", "engine"] {
        let closure = normal_closure(root, package)?;
        for f in forbidden {
            if closure.iter().any(|c| c == f) {
                found.push(format!("`{package}` depends on `{f}`"));
            }
        }
    }
    Ok(found)
}

// ── The gate ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// A gate input.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "lowercase")]
enum Input {
    Converge(ConvergeInput),
    Independence(IndependenceInput),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConvergeInput {
    description: String,
    /// `figure_eight` or `lagrange`.
    start: String,
    /// The ladder's overrides of [`LADDER`].
    #[serde(default)]
    rungs: Option<u32>,
    #[serde(default)]
    fixed_bits: Option<usize>,
    /// The truth's bound (the start's own: closure for the figure-eight, the rigid rotation for Lagrange).
    truth_bound: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IndependenceInput {
    description: String,
    /// A line appended to the source before the scan, a control's.
    #[serde(default)]
    inject: Option<String>,
    /// The crates `kernel` and `engine` may not depend on; [`PRECISION_CRATES`] when absent.
    #[serde(default)]
    forbidden: Option<Vec<String>>,
}

/// A start the gate runs, its output times, and the truth at the last: `(start, times, truth(end) → distance)`.
pub struct Case {
    /// The start.
    pub start: Start,
    /// The output times.
    pub times: Vec<f64>,
    /// What the truth is.
    pub truth: &'static str,
    /// The distance of the accepted run's last state from the truth, scaled as the agreement is.
    pub distance: fn(&brutus::Output, &Start) -> f64,
}

/// The scaled distance of `(r, v)` from `(r₀, v₀)`: the largest `|Δrᵢ|/R` and `|Δvᵢ|/V`, `R` and `V` the truth's
/// largest radius and speed.
fn distance(r: &[[f64; 2]; 3], v: &[[f64; 2]; 3], r0: &[[f64; 2]; 3], v0: &[[f64; 2]; 3]) -> f64 {
    let norm = |a: [f64; 2]| a[0].hypot(a[1]);
    let size = r0.iter().map(|&a| norm(a)).fold(0.0f64, f64::max);
    let speed = v0.iter().map(|&a| norm(a)).fold(0.0f64, f64::max);
    (0..3)
        .map(|i| {
            let dr = norm([r[i][0] - r0[i][0], r[i][1] - r0[i][1]]) / size;
            let dv = norm([v[i][0] - v0[i][0], v[i][1] - v0[i][1]]) / speed;
            dr.max(dv)
        })
        .fold(0.0f64, f64::max)
}

/// The figure-eight (`tiers::figure_eight`, Simó's digits) over one period: it returns to its start.
pub fn figure_eight_case() -> Case {
    let s = tiers::figure_eight();
    Case {
        start: from_published(&s),
        times: vec![tiers::figure_eight_period()],
        truth: "closure after the period",
        distance: |o, s| distance(&o.r, &o.v, &s.r, &s.w),
    }
}

/// Lagrange's triangle at masses (0.2, 0.3, 0.5) (`tiers::lagrange`) over half a turn, against its rigid rotation.
pub fn lagrange_case() -> Case {
    let rigid = tiers::lagrange([0.2, 0.3, 0.5]);
    let half = rigid.period() / 2.0;
    Case {
        start: from_ic(&rigid.start),
        times: vec![half / 2.0, half],
        truth: "the rigid rotation at half a turn",
        distance: |o, s| {
            let rigid = tiers::lagrange(s.m);
            let x = rigid.at(o.t);
            let v0 = [0, 1, 2].map(|i| [x.p[i][0] / x.m[i], x.p[i][1] / x.m[i]]);
            distance(&o.r, &o.v, &x.r, &v0)
        },
    }
}

/// The ladder's table, a line per rung.
pub fn table(runs: &[brutus::Run], disagreement: &[f64]) -> Vec<String> {
    runs.iter()
        .enumerate()
        .map(|(j, r)| {
            let last = r.outputs.last();
            format!(
                "rung {j}: {} digits, {} bits, {} columns: {} steps ({} rejected), {} evaluations, |ΔE/E| {:.2e}, ΔL_z {:.2e}{}",
                r.rung.digits,
                r.rung.bits,
                r.rung.columns,
                r.accepted,
                r.rejected,
                r.evaluations,
                last.map_or(f64::NAN, |o| o.energy_error.abs()),
                last.map_or(f64::NAN, |o| o.lz_error.abs()),
                match j.checked_sub(1).and_then(|k| disagreement.get(k)) {
                    Some(d) => format!(", disagreement with the rung before {d:.3e}"),
                    None => String::new(),
                }
            )
        })
        .collect()
}

/// A ladder's verdict against the threshold and the truth: `(pass, lines)`.
pub fn judge(
    result: &Result<Converged, NotConverged>,
    case: &Case,
    threshold: f64,
    truth_bound: f64,
) -> (bool, Vec<String>, Vec<String>) {
    match result {
        Ok(c) => {
            let accepted = c.accepted();
            let last = accepted.outputs.last().expect("an output");
            let d = (case.distance)(last, &case.start);
            let agreed = *c.disagreement.last().expect("two rungs") <= threshold;
            let truth = d <= truth_bound;
            let mut negative = vec![format!(
                "converged: the {}-digit rung agrees with the {}-digit rung to {:.3e} (threshold {threshold:e}){}",
                accepted.rung.digits,
                c.runs[c.runs.len() - 2].rung.digits,
                c.disagreement.last().unwrap(),
                if agreed { "" } else { ": NOT within it" }
            )];
            negative.push(format!(
                "truth: {} at {}: distance {d:.3e}, bound {truth_bound:e}{}",
                case.truth,
                last.t,
                if truth { "" } else { ": BEYOND it" }
            ));
            (agreed && truth, table(&c.runs, &c.disagreement), negative)
        }
        Err(e) => {
            let lines = match e {
                NotConverged::Spent { runs, disagreement }
                | NotConverged::Failed {
                    runs, disagreement, ..
                } => table(runs, disagreement),
                NotConverged::Vacuous { .. } => vec!["no table: the ladder was refused".to_owned()],
            };
            (false, lines, vec![e.to_string()])
        }
    }
}

/// The gate `reference-convergence`.
pub struct ReferenceConvergenceGate;

impl Gate for ReferenceConvergenceGate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let input = Input::deserialize(fixture).map_err(|e| format!("refused: {name}: {e}"))?;
        let threshold = config.threshold.value;
        let (description, pass, scatter, negative, regions) = match input {
            Input::Converge(c) => {
                let case = match c.start.as_str() {
                    "figure_eight" => figure_eight_case(),
                    "lagrange" => lagrange_case(),
                    other => {
                        return Err(format!(
                            "refused: {name}: no start `{other}` (figure_eight or lagrange)"
                        ))
                    }
                };
                let agree = -threshold.log10();
                if agree.fract() != 0.0 || agree <= 0.0 {
                    return Err(format!(
                        "refused: {name}: the threshold {threshold:e} is not 10^−p"
                    ));
                }
                let ladder = Ladder {
                    agree: agree as u32,
                    rungs: c.rungs.unwrap_or(LADDER.rungs),
                    fixed_bits: c.fixed_bits,
                    ..LADDER
                };
                let result = brutus::converge(&case.start, &case.times, &ladder);
                let (pass, scatter, negative) = judge(&result, &case, threshold, c.truth_bound);
                (c.description, pass, scatter, negative, case.times.len())
            }
            Input::Independence(i) => {
                let root = import::root();
                let mut source = std::fs::read_to_string(root.join(SOURCE))
                    .map_err(|e| format!("{SOURCE}: {e}"))?;
                if let Some(line) = &i.inject {
                    source.push('\n');
                    source.push_str(line);
                    source.push('\n');
                }
                let manifest = std::fs::read_to_string(root.join("crates/validation/Cargo.toml"))
                    .map_err(|e| format!("crates/validation/Cargo.toml: {e}"))?;
                let externs = extern_crates(&manifest);
                let scan = independence(&source, &externs);
                let forbidden: Vec<String> = i
                    .forbidden
                    .unwrap_or_else(|| PRECISION_CRATES.iter().map(|s| s.to_string()).collect());
                let forbidden: Vec<&str> = forbidden.iter().map(String::as_str).collect();
                let closure = kernel_closure(&root, &forbidden)?;
                let pass = scan.is_empty() && closure.is_empty();
                let scatter = vec![
                    format!("{SOURCE}: {} line(s) scanned against {} extern crate(s)", source.lines().count(), externs.len()),
                    format!("kernel and engine's normal and build dependencies checked for {forbidden:?}"),
                ];
                let mut negative: Vec<String> = scan.into_iter().chain(closure).collect();
                if negative.is_empty() {
                    negative.push("nothing reached: the reference names only std and dashu_float, and neither kernel nor engine depends on the precision crates".to_owned());
                }
                (i.description, pass, scatter, negative, 1)
            }
        };
        Ok(GateReport {
            gate: NAME.to_owned(),
            input: name.to_owned(),
            description,
            verdict: if pass { Verdict::Pass } else { Verdict::Fail },
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded(regions as u32),
                minimum: config.min_regions.clone(),
            }),
            negative_results: negative,
        })
    }
}
