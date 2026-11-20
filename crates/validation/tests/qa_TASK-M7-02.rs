//! qa's tests for TASK-M7-02's `oklab-roundtrip` gate (REQ-COL-049; PIT-3), written from the gate's stated
//! measurement: at every lattice point, the encoded colour `code/255` in f32 is decoded, taken to OKLab and back, and
//! encoded; the linear error is the largest `|lin' − lin|` on any channel, the encoded error the largest `|c' − c|`;
//! the gate passes when both are within the threshold, and reports, when one is not, how many points lie beyond it
//! and the first point (red slowest, blue fastest) at which the largest error is reached.
//!
//! The errors are recomputed here from that statement, not read from the gate's own per-point lists, and compared
//! with what the gate reports: the count of points beyond a threshold that some points equal exactly (so "beyond"
//! must mean strictly greater, the complement of "within"), and the first of tied maxima.

use std::path::{Path, PathBuf};

use render::colour::space::{self, Coefficients, OKLAB};
use validation::gate::{self, GateConfig, Verdict};
use validation::negative_control;
use validation::oklab::{measure, Input, OklabRoundtripGate};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The lattice of `stride` in the gate's order, red slowest, blue fastest.
fn lattice(stride: u8) -> Vec<[u8; 3]> {
    let codes: Vec<u8> = (0..=255u8).filter(|c| c % stride == 0).collect();
    let mut out = Vec::new();
    for &r in &codes {
        for &g in &codes {
            for &b in &codes {
                out.push([r, g, b]);
            }
        }
    }
    out
}

/// One point's (linear, encoded) error through `k`, from the gate's statement.
fn errors(k: &Coefficients, code: [u8; 3]) -> (f64, f64) {
    let c = code.map(|x| f32::from(x) / 255.0);
    let lin = c.map(space::srgb_to_linear::<f32>);
    let back = space::oklab_to_linear_with(k, space::linear_to_oklab_with(k, lin));
    let enc = back.map(space::linear_to_srgb::<f32>);
    let worst = |a: [f32; 3], b: [f32; 3]| {
        (0..3)
            .map(|i| (f64::from(a[i]) - f64::from(b[i])).abs())
            .fold(0.0, f64::max)
    };
    (worst(back, lin), worst(enc, c))
}

fn config(value: f64) -> GateConfig {
    let mut config = gate::config(&root(), "oklab-roundtrip").expect("the gate's config");
    config.threshold.value = value;
    config
}

const LINEAR: &str = "linear sRGB ↔ OKLab, |lin' − lin|";
const ENCODED: &str = "the whole chain, encoded, |c' − c|";

// ── The count of points beyond the threshold ───────────────────────────────────────────────────────────────────

/// `judge` at a threshold equal to an error some lattice points reach exactly, below the largest: the verdict is a
/// fail, and each error's line counts exactly the points strictly beyond the threshold.
fn check_over_count(judge: fn(&GateConfig, &str, &Input) -> Result<Vec<String>, String>) {
    let stride = 15;
    let points = lattice(stride);
    let all: Vec<(f64, f64)> = points.iter().map(|&p| errors(&OKLAB, p)).collect();
    let max_linear = all.iter().map(|e| e.0).fold(0.0, f64::max);
    // The most frequent nonzero linear error below the largest: several points sit exactly at the threshold.
    let mut values: Vec<f64> = all
        .iter()
        .map(|e| e.0)
        .filter(|&e| e > 0.0 && e < max_linear)
        .collect();
    values.sort_by(f64::total_cmp);
    let mut threshold = values[0];
    let mut best = 0;
    let mut i = 0;
    while i < values.len() {
        let j = values[i..].iter().take_while(|&&v| v == values[i]).count();
        if j > best {
            best = j;
            threshold = values[i];
        }
        i += j;
    }
    assert!(
        best >= 2,
        "the lattice should have a linear error several points share"
    );
    let over_linear = all.iter().filter(|e| e.0 > threshold).count();
    let over_encoded = all.iter().filter(|e| e.1 > threshold).count();
    let at_linear = all.iter().filter(|e| e.0 == threshold).count();
    assert!(
        over_linear > 0 && at_linear >= 2,
        "the threshold must lie strictly inside the linear errors"
    );
    let input = Input {
        description: "qa".to_owned(),
        stride,
        perturb: None,
        recorded: None,
    };
    let lines = judge(&config(threshold), "qa.json", &input).expect("the gate judges");
    let n = points.len();
    for (what, over) in [(LINEAR, over_linear), (ENCODED, over_encoded)] {
        let want = format!("{what}: {over} of {n} points beyond the threshold");
        assert!(
            lines.iter().any(|l| l.starts_with(&want)),
            "the gate's count of points beyond {threshold:e} is wrong: want `{want}`, report {lines:#?}"
        );
    }
}

fn judge_lines(config: &GateConfig, name: &str, input: &Input) -> Result<Vec<String>, String> {
    let report = OklabRoundtripGate::judge(config, name, input)?;
    assert!(
        report.verdict == Verdict::Fail,
        "a threshold below the largest error must fail"
    );
    Ok(report.negative_results)
}

#[test]
fn qa_m7_02_gate_counts_points_beyond() {
    check_over_count(judge_lines);
}

negative_control!(
    qa_m7_02_gate_counts_points_beyond,
    "a report counting the points at or beyond the threshold must fail",
    expected = "the gate's count of points beyond",
    check_over_count(|config, _name, input| {
        // The report the `>=` count would give: every line's count raised by the points exactly at the threshold.
        let t = config.threshold.value;
        let all: Vec<(f64, f64)> = lattice(input.stride)
            .iter()
            .map(|&p| errors(&OKLAB, p))
            .collect();
        let n = all.len();
        let at = |f: fn(&(f64, f64)) -> f64| all.iter().filter(|e| f(e) >= t).count();
        Ok(vec![
            format!(
                "{LINEAR}: {} of {n} points beyond the threshold",
                at(|e| e.0)
            ),
            format!(
                "{ENCODED}: {} of {n} points beyond the threshold",
                at(|e| e.1)
            ),
        ])
    })
);

/// The points a measurement reports its largest linear and encoded errors at, for coefficients and a stride.
type WorstPoints = fn(&Coefficients, u8) -> ([u8; 3], [u8; 3]);

// ── The first of tied maxima ───────────────────────────────────────────────────────────────────────────────────

/// With every matrix zero, OKLab is 0 and the colour comes back black, so each point's error is its largest linear
/// channel: on the lattice of stride 255 the largest error, 1, is tied at seven points. `measure` must report the
/// first in lattice order, for the linear error and the encoded one, as recomputed here.
fn check_first_of_ties(at: WorstPoints) {
    let zero = Coefficients {
        m1: [[0.0; 3]; 3],
        m2: [[0.0; 3]; 3],
        m2_inv: [[0.0; 3]; 3],
        m1_inv: [[0.0; 3]; 3],
    };
    let stride = 255;
    let points = lattice(stride);
    let all: Vec<(f64, f64)> = points.iter().map(|&p| errors(&zero, p)).collect();
    let first = |f: fn(&(f64, f64)) -> f64| {
        let max = all.iter().map(f).fold(0.0, f64::max);
        let ties = all.iter().filter(|e| f(e) == max).count();
        assert!(
            ties >= 2,
            "the largest error must be tied for the order to matter"
        );
        points[all.iter().position(|e| f(e) == max).unwrap()]
    };
    let want = (first(|e| e.0), first(|e| e.1));
    let got = at(&zero, stride);
    assert!(
        got == want,
        "the worst point is not the first of the tied maxima: got {got:?}, want {want:?}"
    );
}

#[test]
fn qa_m7_02_measure_first_of_ties() {
    check_first_of_ties(|k, stride| {
        let m = measure(k, stride).expect("a lattice");
        (m.linear.at, m.srgb.at)
    });
}

negative_control!(
    qa_m7_02_measure_first_of_ties,
    "reporting the last of the tied maxima must fail",
    expected = "not the first of the tied maxima",
    check_first_of_ties(|k, stride| {
        let points = lattice(stride);
        let all: Vec<(f64, f64)> = points.iter().map(|&p| errors(k, p)).collect();
        let last = |f: fn(&(f64, f64)) -> f64| {
            let max = all.iter().map(f).fold(0.0, f64::max);
            points[all.iter().rposition(|e| f(e) == max).unwrap()]
        };
        (last(|e| e.0), last(|e| e.1))
    })
);
