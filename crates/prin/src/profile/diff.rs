//! `prin profile diff BASE NEW --threshold P%` (render_gui_spec § "Profiler", "What `prin profile diff` compares",
//! REQ-TOOL-119): for each scope, the p95 of its per-frame ms in BASE and in NEW; a regression where NEW's p95 is more
//! than P% above BASE's, decided exactly (R-323). It exits 1 when any scope regresses, 0 when none does, and 2 when a
//! file cannot be read or either file has no frame records (R-323, R-328). A trace that is an incomplete session, its
//! last line perhaps cut off, is compared as usual, and the diff says so first: "session incomplete", with the bytes
//! the reader dropped (R-323, R-298, R-299).
//!
//! The scopes: the frame (`frame_ms`), each stage by its key (`stage_ms`, in the frames where it is not null), each CPU
//! scope by its stage and the names from the stage down to it, and each GPU pass by its stage and name. A scope that
//! occurs more than once in a frame gives that frame the sum of its ms; a frame where it does not occur gives no
//! sample. A scope in only one file is listed, not compared, and is not a regression.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::path::Path;
use std::process::ExitCode;

use engine::contract::profile::{self, percentile, Scope, Session, Stage, Trace};

/// A stage, by its place in `stage_ms`'s order, so the report lists the stages in that order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct At(usize);

impl At {
    fn key(self) -> &'static str {
        Stage::ALL[self.0].key()
    }
}

/// One scope of the diff.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    /// The frame: `frame_ms`.
    Frame,
    /// A stage: its `stage_ms`.
    Stage(At),
    /// A CPU scope: its stage, then the names from the stage down to it.
    Scope(At, Vec<String>),
    /// A GPU pass: its stage and name.
    GpuPass(At, String),
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Frame => write!(f, "frame"),
            Key::Stage(stage) => write!(f, "stage {}", stage.key()),
            Key::Scope(stage, path) => write!(f, "scope {}/{}", stage.key(), path.join("/")),
            Key::GpuPass(stage, name) => write!(f, "gpu pass {}/{name}", stage.key()),
        }
    }
}

/// `--threshold P%`: P as the decimal written, `numerator / 10^scale`, so the regression test is exact (R-323).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Threshold {
    /// P's digits, the point dropped.
    numerator: u128,
    /// How many of them follow the point.
    scale: u32,
    /// P as written, without its `%`.
    text: String,
}

impl fmt::Display for Threshold {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", self.text)
    }
}

/// The most digits P may have, so every product [`regresses`] forms fits in a u128: (100 · 10^scale + numerator) is
/// below 2^67, and a double's significand below 2^53.
const MAX_DIGITS: usize = 18;

/// Parses `--threshold`: a percentage ≥ 0 written as a plain decimal, `5%`, `5`, `7.5%` or `0.25`, of at most 18
/// digits.
pub(crate) fn parse_threshold(text: &str) -> Result<Threshold, String> {
    let number = text.strip_suffix('%').unwrap_or(text);
    let (int, frac) = number.split_once('.').unwrap_or((number, ""));
    let digits = format!("{int}{frac}");
    if int.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) || digits.len() > MAX_DIGITS {
        return Err(format!(
            "{text:?} is not a percentage ≥ 0 such as 5% or 7.5%, of at most {MAX_DIGITS} digits"
        ));
    }
    Ok(Threshold {
        numerator: digits
            .parse()
            .map_err(|_| format!("{text:?} is not a percentage such as 5%"))?,
        scale: u32::try_from(frac.len()).map_err(|e| e.to_string())?,
        text: number.to_owned(),
    })
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
        for (i, (stage, value)) in Stage::ALL.into_iter().zip(stage_ms).enumerate() {
            let at = At(i);
            if let Some(value) = value {
                this.insert(Key::Stage(at), value);
            }
            let Some(sections) = frame.stages.get(stage) else {
                continue;
            };
            let mut path = Vec::new();
            add_scopes(at, &sections.scopes, &mut path, &mut this);
            for pass in &sections.gpu_passes {
                *this
                    .entry(Key::GpuPass(at, pass.name.clone()))
                    .or_insert(0.0) += pass.ms;
            }
        }
        for (key, value) in this {
            all.entry(key).or_default().push(value);
        }
    }
    all
}

fn add_scopes(stage: At, scopes: &[Scope], path: &mut Vec<String>, this: &mut BTreeMap<Key, f64>) {
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

/// A finite double's magnitude as `significand · 2^exponent`, exactly.
fn parts(x: f64) -> (u128, i32) {
    let bits = x.to_bits();
    let biased = ((bits >> 52) & 0x7ff) as i32;
    let fraction = u128::from(bits & ((1 << 52) - 1));
    if biased == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1 << 52), biased - 1075)
    }
}

/// Compares `x · 2^k` with `y`, for x and y below 2^128, exactly.
fn compare_scaled(x: u128, k: i32, y: u128) -> Ordering {
    if x == 0 || y == 0 {
        return x.cmp(&y);
    }
    let (lx, ly) = (
        128 - x.leading_zeros() as i32,
        128 - y.leading_zeros() as i32,
    );
    // x · 2^k lies in [2^(lx+k−1), 2^(lx+k)), y in [2^(ly−1), 2^ly).
    if lx + k > ly {
        Ordering::Greater
    } else if lx + k < ly {
        Ordering::Less
    } else if k >= 0 {
        // lx + k = ly ≤ 128, so the shift keeps every bit.
        (x << k).cmp(&y)
    } else {
        // ly − k = lx ≤ 128, likewise.
        x.cmp(&(y << -k))
    }
}

/// Whether NEW's p95 `new` regresses on BASE's `base` by more than the threshold P: `(new − base) / base × 100 > P`,
/// or, from a BASE of 0, `new > 0` (render_gui_spec § "Profiler"). Decided exactly, not by rounded floating-point
/// arithmetic (R-323): as `100 · 10^s · new > (100 · 10^s + p) · base`, P being p / 10^s and each p95 the double it
/// is, so 100 → 107 at 7% is a rise of exactly 7%, and not a regression. Each p95 is a duration, never negative: the
/// reader refuses a negative ms (telemetry §5).
fn regresses(base: f64, new: f64, threshold: &Threshold) -> bool {
    let unit = 100 * 10u128.pow(threshold.scale);
    let (new_m, new_e) = parts(new);
    let (base_m, base_e) = parts(base);
    compare_scaled(
        unit * new_m,
        new_e - base_e,
        (unit + threshold.numerator) * base_m,
    ) == Ordering::Greater
}

/// Compares NEW's per-scope p95 with BASE's at `threshold` percent.
pub(crate) fn compare(base: &Trace, new: &Trace, threshold: &Threshold) -> Report {
    let mut text = String::new();
    for (name, trace) in [("BASE", base), ("NEW", new)] {
        if trace.session == Session::Incomplete {
            text.push_str(&incomplete(name, trace));
        }
    }
    let base = p95s(base);
    let new = p95s(new);
    text.push_str(&format!(
        "p95 per scope, ms; a regression is a rise of more than {threshold}\n"
    ));
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

/// The notice for a trace that is an incomplete session (R-298): "session incomplete", and the bytes of a cut-off last
/// line the reader dropped (R-299), 0 when none was. Its frames are still compared (R-323).
fn incomplete(name: &str, trace: &Trace) -> String {
    format!(
        "{name}: session incomplete; {} bytes of a cut-off last line dropped\n",
        trace.dropped_bytes
    )
}

/// The refusal for a trace with no frame records, which has no p95 to compare: exit 2 (R-323, R-328). An
/// incomplete session's notice comes first, as it does before a comparison.
fn no_frames(name: &str, path: &Path, trace: &Trace) -> Option<String> {
    if !trace.frames.is_empty() {
        return None;
    }
    let notice = if trace.session == Session::Incomplete {
        incomplete(name, trace)
    } else {
        String::new()
    };
    Some(format!(
        "{notice}prin profile diff: {name} has no frame records ({}), so nothing to compare",
        path.display()
    ))
}

/// `prin profile diff BASE NEW --threshold P%`. A BASE or NEW with no frame records has nothing to compare, and is an
/// error, exit 2 (R-323, R-328).
pub(crate) fn main(base: &Path, new: &Path, threshold: &Threshold) -> Result<ExitCode, String> {
    let (base_trace, new_trace) = (read(base)?, read(new)?);
    if let Some(why) = no_frames("BASE", base, &base_trace) {
        return Err(why);
    }
    if let Some(why) = no_frames("NEW", new, &new_trace) {
        return Err(why);
    }
    let report = compare(&base_trace, &new_trace, threshold);
    print!("{}", report.text);
    Ok(if report.regressed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

/// The tests' scratch guard (R-342), shared with the crate's integration tests.
#[cfg(test)]
#[path = "../../../validation/tests/support/scratch.rs"]
mod scratch;

#[cfg(test)]
mod tests {
    use super::scratch::Scratch;
    use super::*;

    /// `x · 2^k` against `y`, exactly: the zero short-circuit, each length branch, and each tie at equal lengths.
    fn check_compare_scaled(compare: fn(u128, i32, u128) -> Ordering) {
        let cases: [(u128, i32, u128, Ordering); 13] = [
            (0, 0, 0, Ordering::Equal),
            (0, 10, 1, Ordering::Less),
            (1, -10, 0, Ordering::Greater),
            (1, 10, 2, Ordering::Greater),
            (1 << 9, 3, 1 << 20, Ordering::Less),
            (1, -127, 1 << 127, Ordering::Less),
            (3, 1, 6, Ordering::Equal),
            (3, 1, 5, Ordering::Greater),
            (3, 1, 7, Ordering::Less),
            (5, -1, 3, Ordering::Less),
            (7, -1, 3, Ordering::Greater),
            (4, -1, 2, Ordering::Equal),
            (u128::MAX, 0, u128::MAX, Ordering::Equal),
        ];
        for (x, k, y, want) in cases {
            assert_eq!(
                compare(x, k, y),
                want,
                "{x} · 2^{k} against {y} is misordered"
            );
        }
    }

    #[test]
    fn profile_diff_compare_scaled_is_exact() {
        check_compare_scaled(compare_scaled);
    }

    validation::negative_control!(
        profile_diff_compare_scaled_is_exact,
        "a comparison that ignores the power of two must fail the check",
        expected = "is misordered",
        check_compare_scaled(|x, _, y| x.cmp(&y))
    );

    /// A double's magnitude as significand and power of two: normal, subnormal and zero.
    fn check_parts(parts: fn(f64) -> (u128, i32)) {
        let cases: [(f64, (u128, i32)); 5] = [
            (0.0, (0, -1074)),
            (f64::from_bits(1), (1, -1074)),
            (f64::from_bits((1 << 52) - 1), ((1 << 52) - 1, -1074)),
            (1.0, (1 << 52, -52)),
            (f64::MAX, ((1 << 53) - 1, 971)),
        ];
        for (x, want) in cases {
            assert_eq!(parts(x), want, "{x:e} is split wrongly");
        }
    }

    #[test]
    fn profile_diff_parts_are_exact() {
        check_parts(parts);
    }

    validation::negative_control!(
        profile_diff_parts_are_exact,
        "a split that drops the exponent must fail the check",
        expected = "is split wrongly",
        check_parts(|x| (u128::from(x.to_bits()), 0))
    );

    /// R-323's exact test, at ties, from zero and between subnormal p95s.
    fn check_regresses(regresses: fn(f64, f64, &Threshold) -> bool) {
        let cases: [(f64, f64, &str, bool); 9] = [
            (100.0, 107.0, "7%", false),
            (100.0, f64::from_bits(107.0f64.to_bits() + 1), "7%", true),
            (0.0, 0.0, "5%", false),
            (0.0, f64::from_bits(1), "5%", true),
            (f64::from_bits(1), f64::from_bits(1), "0%", false),
            (f64::from_bits(1), f64::from_bits(2024), "5%", true),
            (1e-300, f64::from_bits(1), "5%", false),
            (f64::from_bits(1), 1e-300, "5%", true),
            (f64::from_bits(100), f64::from_bits(105), "5%", false),
        ];
        for (base, new, threshold, want) in cases {
            let p = parse_threshold(threshold).expect("a threshold");
            assert_eq!(
                regresses(base, new, &p),
                want,
                "{base:e} -> {new:e} at {threshold} is misjudged"
            );
        }
    }

    #[test]
    fn profile_diff_regresses_is_exact() {
        check_regresses(regresses);
    }

    validation::negative_control!(
        profile_diff_regresses_is_exact,
        "a test that never finds a regression must fail the check",
        expected = "is misjudged",
        check_regresses(|_, _, _| false)
    );

    /// `--threshold` prints as written, with its `%`.
    fn check_threshold_shown(show: fn(&Threshold) -> String) {
        for (text, want) in [("7.5%", "7.5%"), ("5", "5%"), ("0.25", "0.25%")] {
            let p = parse_threshold(text).expect("a threshold");
            assert_eq!(show(&p), want, "{text:?} is shown wrongly");
        }
    }

    #[test]
    fn profile_diff_threshold_shown_as_written() {
        check_threshold_shown(|p| p.to_string());
    }

    validation::negative_control!(
        profile_diff_threshold_shown_as_written,
        "a threshold shown without its % must fail the check",
        expected = "is shown wrongly",
        check_threshold_shown(|p| p.text.clone())
    );

    /// A complete trace of 20 frames.
    const BASE: &str = include_str!("../../tests/fixtures/profile/base.jsonl");

    /// A fresh scratch file holding `text`, under the system temp folder; deleted when the test passes, kept when it
    /// fails (R-342).
    fn scratch(name: &str, text: &str) -> Scratch {
        let path = Scratch::new(&format!("prin-diff-unit-{name}"));
        std::fs::write(&path, text).expect("the scratch file is written");
        path
    }

    type Diff = fn(&Path, &Path, &Threshold) -> Result<ExitCode, String>;

    /// A NEW with no frame records is refused; the refusal says "session incomplete" when NEW is an incomplete
    /// session, and not when it is complete (R-323).
    fn check_no_frames_notice(diff: Diff, tag: &str) {
        let lines: Vec<&str> = BASE.lines().collect();
        let (header, summary) = (lines[0], lines[lines.len() - 1]);
        let base = scratch(&format!("{tag}-base"), BASE);
        let threshold = parse_threshold("5%").expect("a threshold");
        for (name, text, incomplete) in [
            ("complete", format!("{header}\n{summary}\n"), false),
            ("incomplete", format!("{header}\n"), true),
        ] {
            let new = scratch(&format!("{tag}-{name}"), &text);
            let refused = diff(&base, &new, &threshold);
            let Err(why) = refused else {
                panic!("a {name} NEW with no frames is not refused");
            };
            assert!(
                why.contains("no frame records"),
                "the refusal does not say why: {why}"
            );
            assert_eq!(
                why.contains("session incomplete"),
                incomplete,
                "a {name} NEW with no frames has the wrong notice: {why}"
            );
        }
    }

    #[test]
    fn profile_diff_no_frames_notice() {
        check_no_frames_notice(main, "test");
    }

    validation::negative_control!(
        profile_diff_no_frames_notice,
        "a refusal that never says session incomplete must fail the check",
        expected = "has the wrong notice",
        check_no_frames_notice(
            |base, new, threshold| main(base, new, threshold)
                .map_err(|e| e.replace("session incomplete", "")),
            "control"
        )
    );

    /// A BASE with no frame records is refused, exit 2, as an empty NEW is: with nothing in BASE the gate could never
    /// fail (R-328, physics's finding 2). Both the header-only (incomplete) and header-plus-summary (complete) cases;
    /// an incomplete BASE's notice comes first.
    fn check_no_frames_base(diff: Diff, tag: &str) {
        let lines: Vec<&str> = BASE.lines().collect();
        let (header, summary) = (lines[0], lines[lines.len() - 1]);
        let new = scratch(&format!("{tag}-new"), BASE);
        let threshold = parse_threshold("5%").expect("a threshold");
        for (name, text, incomplete) in [
            ("complete", format!("{header}\n{summary}\n"), false),
            ("incomplete", format!("{header}\n"), true),
        ] {
            let base = scratch(&format!("{tag}-base-{name}"), &text);
            let refused = diff(&base, &new, &threshold);
            let Err(why) = refused else {
                panic!("a {name} BASE with no frames is not refused");
            };
            assert!(
                why.contains("BASE has no frame records"),
                "the refusal does not say BASE has no frame records: {why}"
            );
            assert_eq!(
                why.starts_with("BASE: session incomplete; 0 bytes"),
                incomplete,
                "a {name} BASE with no frames has the wrong notice: {why}"
            );
        }
    }

    #[test]
    fn profile_diff_no_frames_base_exits_2() {
        check_no_frames_base(main, "test-base");
    }

    validation::negative_control!(
        profile_diff_no_frames_base_exits_2,
        "a diff that compares an empty BASE must fail the check",
        expected = "BASE with no frames is not refused",
        check_no_frames_base(
            |_, new, threshold| main(new, new, threshold),
            "control-base"
        )
    );
}
