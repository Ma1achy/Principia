//! The `oklab-roundtrip` gate (TASK-M7-02; REQ-COL-049, PIT-3): the lattice's codes, one colour's round trip, the
//! measurement over a lattice and its order, the quantiles, a coefficient's perturbation, the verdict at and beyond the
//! threshold with its report's sections, the refusals, and the runner on `fixtures/gates/oklab-roundtrip/`, where the
//! perturbed coefficient fails.

use std::path::{Path, PathBuf};

use render::colour::space::{self, Coefficients, OKLAB};
use validation::gate::{self, Gate, GateConfig, Verdict};
use validation::negative_control;
use validation::oklab::{
    self, codes, measure, quantile, round_trip, Input, Matrix, Measurement, OklabRoundtripGate,
    Perturb, Recorded,
};

/// This workspace's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

// ── The lattice ──────────────────────────────────────────────────────────────────────────────────────────────────

/// `of` gives every multiple of a stride dividing 255, from 0 to 255, and refuses a stride of 0 or one not dividing
/// 255.
fn check_codes(of: fn(u8) -> Result<Vec<u8>, String>) {
    for stride in [1u8, 3, 5, 15, 17, 51, 85, 255] {
        let want: Vec<u8> = (0..=255u8).filter(|c| c % stride == 0).collect();
        let got = of(stride).unwrap_or_else(|e| panic!("stride {stride} refused: {e}"));
        assert!(
            got == want,
            "stride {stride} gives the codes {got:?}, not {want:?}"
        );
    }
    for stride in [0u8, 2, 4, 7, 254] {
        let got = of(stride);
        assert!(
            got.as_ref()
                .is_err_and(|e| e.contains("does not divide 255")),
            "stride {stride} is not refused: {got:?}"
        );
    }
}

#[test]
fn oklab_gate_codes() {
    check_codes(codes);
}

negative_control!(
    oklab_gate_codes,
    "codes stopping short of 255 must fail",
    expected = "gives the codes",
    check_codes(|stride| codes(stride).map(|c| c[..c.len() - 1].to_vec()))
);

// ── One colour's round trip, and the measurement ─────────────────────────────────────────────────────────────────

/// `of(k, code)` is the round trip computed here step by step in f32: black is exact; at white, a primary and a mid
/// colour, the linear error is the largest channel's `|lin' − lin|` and the encoded one the largest `|c' − c|`.
fn check_round_trip(of: fn(&Coefficients, [u8; 3]) -> (f64, f64)) {
    assert!(
        of(&OKLAB, [0; 3]) == (0.0, 0.0),
        "black does not go round exactly"
    );
    for code in [
        [255u8; 3],
        [255, 0, 0],
        [0, 0, 255],
        [10, 245, 145],
        [128, 64, 200],
    ] {
        let c = code.map(|x| f32::from(x) / 255.0);
        let lin = c.map(space::srgb_to_linear);
        let back = space::oklab_to_linear(space::linear_to_oklab(lin));
        let enc = back.map(space::linear_to_srgb);
        let worst = |a: [f32; 3], b: [f32; 3]| {
            (0..3)
                .map(|i| (f64::from(a[i]) - f64::from(b[i])).abs())
                .fold(0.0, f64::max)
        };
        let want = (worst(back, lin), worst(enc, c));
        let got = of(&OKLAB, code);
        assert!(
            got == want,
            "the round trip at {code:?} is {got:?}, not {want:?}"
        );
    }
}

#[test]
fn oklab_gate_round_trip() {
    check_round_trip(round_trip);
}

negative_control!(
    oklab_gate_round_trip,
    "errors swapped between linear and encoded must fail",
    expected = "the round trip at",
    check_round_trip(|k, code| {
        let (l, e) = round_trip(k, code);
        (e, l)
    })
);

/// `m` is the lattice of `stride` measured point by point, red slowest, blue fastest: each point's errors, the
/// largest of each and the first point reaching it.
fn check_measurement(m: &Measurement, stride: u8) {
    let codes = codes(stride).unwrap();
    let mut points = Vec::new();
    for &r in &codes {
        for &g in &codes {
            for &b in &codes {
                points.push([r, g, b]);
            }
        }
    }
    assert!(
        m.points as usize == points.len()
            && m.linear_all.len() == points.len()
            && m.srgb_all.len() == points.len(),
        "the measurement holds {} points ({} and {} errors), not the lattice's {}",
        m.points,
        m.linear_all.len(),
        m.srgb_all.len(),
        points.len()
    );
    for (k, code) in points.iter().enumerate() {
        let (l, e) = round_trip(&OKLAB, *code);
        assert!(
            m.linear_all[k] == l && m.srgb_all[k] == e,
            "point {k}'s errors are not the round trip at {code:?}"
        );
    }
    for (what, worst, all) in [
        ("linear", m.linear, &m.linear_all),
        ("encoded", m.srgb, &m.srgb_all),
    ] {
        let max = all.iter().copied().fold(0.0, f64::max);
        let first = all.iter().position(|&e| e == max).unwrap();
        assert!(
            worst.error == max && worst.at == points[first],
            "the {what} worst is {worst:?}, not {max:e} at {:?}, the first point reaching it",
            points[first]
        );
    }
}

#[test]
fn oklab_gate_measure() {
    // Stride 15: 18 codes a channel, 5832 points, several of which tie.
    check_measurement(&measure(&OKLAB, 15).unwrap(), 15);
}

negative_control!(
    oklab_gate_measure,
    "a measurement naming a point off the lattice as its worst must fail",
    expected = "the first point reaching it",
    {
        let mut m = measure(&OKLAB, 15).unwrap();
        m.linear.at = [1, 2, 3];
        check_measurement(&m, 15)
    }
);

/// `of` is the nearest-rank quantile: the `⌈q·n⌉`-th smallest, the smallest at `q = 0`, and none of no values.
fn check_quantile(of: fn(&[f64], f64) -> Option<f64>) {
    let v = [3.0, 1.0, 4.0, 2.0];
    for (q, want) in [
        (0.0, 1.0),
        (0.25, 1.0),
        (0.26, 2.0),
        (0.5, 2.0),
        (0.75, 3.0),
        (0.99, 4.0),
        (1.0, 4.0),
    ] {
        let got = of(&v, q);
        assert!(
            got == Some(want),
            "the {q} quantile of {v:?} is {got:?}, not {want}"
        );
    }
    assert!(of(&[], 0.5).is_none(), "no values have a quantile");
}

#[test]
fn oklab_gate_quantile() {
    check_quantile(quantile);
}

negative_control!(
    oklab_gate_quantile,
    "a rank rounded down must fail",
    expected = "quantile of",
    check_quantile(|v, q| {
        let mut s = v.to_vec();
        s.sort_by(f64::total_cmp);
        s.get(((q * s.len() as f64).floor() as usize).min(s.len().max(1) - 1))
            .copied()
    })
);

// ── The perturbation ─────────────────────────────────────────────────────────────────────────────────────────────

/// Each of `apply`'s perturbations changes its matrix's entry by `by` and nothing else; an entry off the matrix, and
/// a zero or non-finite change, are refused.
fn check_perturb(apply: fn(&Perturb, &Coefficients) -> Result<Coefficients, String>) {
    let pick = |k: &Coefficients, m: Matrix| match m {
        Matrix::M1 => k.m1,
        Matrix::M2 => k.m2,
        Matrix::M2Inv => k.m2_inv,
        Matrix::M1Inv => k.m1_inv,
    };
    let all = [Matrix::M1, Matrix::M2, Matrix::M2Inv, Matrix::M1Inv];
    for matrix in all {
        let p = Perturb {
            matrix,
            row: 2,
            col: 1,
            by: 0.5,
        };
        let k = apply(&p, &OKLAB).unwrap_or_else(|e| panic!("{p:?} refused: {e}"));
        for other in all {
            let mut want = pick(&OKLAB, other);
            if other == matrix {
                want[2][1] += 0.5;
            }
            assert!(
                pick(&k, other) == want,
                "{p:?} leaves {other:?} as {:?}, not {want:?}",
                pick(&k, other)
            );
        }
    }
    for (row, col, by) in [
        (3, 0, 1e-4),
        (0, 3, 1e-4),
        (0, 0, 0.0),
        (0, 0, f64::NAN),
        (0, 0, f64::INFINITY),
    ] {
        let p = Perturb {
            matrix: Matrix::M1,
            row,
            col,
            by,
        };
        assert!(apply(&p, &OKLAB).is_err(), "{p:?} is not refused");
    }
}

#[test]
fn oklab_gate_perturb() {
    check_perturb(Perturb::apply);
}

negative_control!(
    oklab_gate_perturb,
    "a perturbation landing in M₂ whatever its matrix must fail",
    expected = "leaves",
    check_perturb(|p, k| {
        Perturb {
            matrix: Matrix::M2,
            ..*p
        }
        .apply(k)
    })
);

// ── The verdict and the report ───────────────────────────────────────────────────────────────────────────────────

/// The gate's configuration, its threshold set to `value`.
fn config(value: f64) -> GateConfig {
    let mut config = gate::config(&root(), oklab::NAME).unwrap();
    config.threshold.value = value;
    config
}

fn input(stride: u8, perturb: Option<Perturb>, recorded: Option<Recorded>) -> Input {
    Input {
        description: "a test input".to_owned(),
        stride,
        perturb,
        recorded,
    }
}

/// The gate passes the lattice of stride 15 at a threshold equal to its larger error and fails it just below, the
/// report giving each error's max, quantiles and margin, the points beyond, the region count and the perturbation;
/// and a recorded measurement differing from the one made is said to.
fn check_verdict(judge: fn(&GateConfig, &str, &Input) -> Result<oklab_report::Report, String>) {
    let m = measure(&OKLAB, 15).unwrap();
    let top = m.linear.error.max(m.srgb.error);
    let pass = judge(&config(top), "x.json", &input(15, None, None)).unwrap();
    assert!(
        pass.verdict == Verdict::Pass,
        "the lattice fails at a threshold equal to its error"
    );
    for (what, w, all) in [
        ("linear sRGB ↔ OKLab", m.linear, &m.linear_all),
        ("the whole chain", m.srgb, &m.srgb_all),
    ] {
        let line = format!(
            "max {:.3e} at sRGB code {:?}; median {:.3e}, p99 {:.3e}, over {} points",
            w.error,
            w.at,
            quantile(all, 0.5).unwrap(),
            quantile(all, 0.99).unwrap(),
            m.points
        );
        let margin = format!(
            "margin {:.1}× (threshold {top:.3e} over max {:.3e})",
            top / w.error,
            w.error
        );
        let none = format!("0 of {} points beyond the threshold {top:.3e}", m.points);
        for want in [line, margin] {
            assert!(
                pass.scatter
                    .iter()
                    .any(|l| l.starts_with(what) && l.ends_with(&want)),
                "the scatter {:?} lacks `{what}: … {want}`",
                pass.scatter
            );
        }
        assert!(
            pass.negative_results
                .iter()
                .any(|l| l.starts_with(what) && l.ends_with(&none)),
            "the negative results {:?} lack `{none}`",
            pass.negative_results
        );
    }
    assert!(
        pass.regions == Some(m.points),
        "the region count is {:?}, not {}",
        pass.regions,
        m.points
    );
    let below = top - top * f64::EPSILON;
    let fail = judge(&config(below), "x.json", &input(15, None, None)).unwrap();
    let beyond = m.srgb_all.iter().filter(|&&e| e > below).count();
    assert!(
        fail.verdict == Verdict::Fail,
        "the lattice passes below its error"
    );
    let want = format!(
        "{beyond} of {} points beyond the threshold {below:.3e}; the worst, {:.3e} at sRGB code {:?}, fails",
        m.points, m.srgb.error, m.srgb.at
    );
    assert!(
        fail.negative_results.iter().any(|l| l.ends_with(&want)),
        "the negative results {:?} lack `{want}`",
        fail.negative_results
    );
    let p = Perturb {
        matrix: Matrix::M1Inv,
        row: 1,
        col: 2,
        by: 1e-4,
    };
    let perturbed = judge(&config(1e-4), "x.json", &input(15, Some(p), None)).unwrap();
    assert!(
        perturbed.verdict == Verdict::Fail,
        "M₁⁻¹ perturbed by 1e-4 passes"
    );
    assert!(
        perturbed.negative_results.iter().any(|l| l == "measured with M1Inv[1][2] perturbed by 1e-4 (PIT-3: a wrong coefficient must fail the gate)"),
        "the negative results {:?} do not name the perturbation",
        perturbed.negative_results
    );
    let differs = "the measurement here differs from the one recorded on here";
    for (linear, srgb, said) in [
        (m.linear.error, m.srgb.error, false),
        (m.linear.error, 1.0, true),
        (1.0, m.srgb.error, true),
    ] {
        let recorded = Recorded {
            platform: "here".to_owned(),
            linear,
            srgb,
        };
        let r = judge(&config(top), "x.json", &input(15, None, Some(recorded))).unwrap();
        let line = format!(
            "recorded on here: linear {linear:e}, encoded {srgb:e}; measured here: linear {:e}, encoded {:e}",
            m.linear.error, m.srgb.error
        );
        assert!(
            r.scatter.contains(&line),
            "the scatter {:?} lacks `{line}`",
            r.scatter
        );
        assert!(
            r.negative_results.iter().any(|l| l.starts_with(differs)) == said,
            "recorded ({linear:e}, {srgb:e}): the report {} it differs",
            if said { "does not say" } else { "says" }
        );
    }
}

/// The parts of a [`gate::GateReport`] the verdict check reads.
mod oklab_report {
    use validation::gate::{GateReport, RegionCount, Verdict};

    pub struct Report {
        pub verdict: Verdict,
        pub scatter: Vec<String>,
        pub negative_results: Vec<String>,
        pub regions: Option<u32>,
    }

    impl From<GateReport> for Report {
        fn from(r: GateReport) -> Self {
            let regions = r.regions.as_ref().and_then(|g| match g.count {
                RegionCount::Recorded(n) => Some(n),
                RegionCount::NotRecorded { .. } => None,
            });
            Self {
                verdict: r.verdict,
                scatter: r.scatter,
                negative_results: r.negative_results,
                regions,
            }
        }
    }
}

#[test]
fn oklab_gate_verdict() {
    check_verdict(|c, n, i| OklabRoundtripGate::judge(c, n, i).map(Into::into));
}

negative_control!(
    oklab_gate_verdict,
    "a gate passing only strictly below the threshold must fail",
    expected = "fails at a threshold equal to its error",
    check_verdict(|c, n, i| {
        let mut c = c.clone();
        c.threshold.value -= c.threshold.value * f64::EPSILON;
        OklabRoundtripGate::judge(&c, n, i).map(Into::into)
    })
);

// ── Refusals ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// `run` refuses a stride not dividing 255, a perturbation off the matrix, and JSON that is not an input (an unknown
/// field), each naming why.
fn check_refusals(run: fn(&GateConfig, &str, &serde_json::Value) -> Result<(), String>) {
    let c = config(1e-4);
    for (json, why) in [
        (
            serde_json::json!({"description": "d", "stride": 7}),
            "does not divide 255",
        ),
        (
            serde_json::json!({"description": "d", "stride": 85,
                "perturb": {"matrix": "m2", "row": 3, "col": 0, "by": 1e-4}}),
            "outside a 3 × 3 matrix",
        ),
        (
            serde_json::json!({"description": "d", "stride": 85, "lattice": 5}),
            "is not an oklab-roundtrip input",
        ),
    ] {
        let got = run(&c, "x.json", &json);
        assert!(
            got.as_ref().is_err_and(|e| e.contains(why)),
            "{json} is not refused with `{why}`: {got:?}"
        );
    }
}

#[test]
fn oklab_gate_refusals() {
    check_refusals(|c, n, v| OklabRoundtripGate.run(c, n, v).map(|_| ()));
}

negative_control!(
    oklab_gate_refusals,
    "a gate refusing nothing must fail",
    expected = "is not refused",
    check_refusals(|_, _, _| Ok(()))
);

// ── The fixtures ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The runner on `root`'s `fixtures/gates/oklab-roundtrip/` gives every input its expected outcome: the whole 8-bit
/// gamut and the stride-5 lattice pass, and the perturbed `M₁` fails (PIT-3).
fn check_fixtures(root: &Path) {
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("oklab_roundtrip_gate");
    let result = gate::run(root, oklab::NAME, &out);
    assert!(
        result.is_ok(),
        "the gate's fixtures do not give their expected outcomes: {result:?}"
    );
}

#[test]
fn oklab_gate_fixtures() {
    check_fixtures(&root());
}

negative_control!(
    oklab_gate_fixtures,
    "the perturbed fixture expected to pass must fail the run",
    expected = "do not give their expected outcomes",
    {
        let copy = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("oklab_roundtrip_copy");
        let dir = copy.join("fixtures/gates").join(oklab::NAME);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(copy.join("plan")).unwrap();
        std::fs::copy(
            root().join("plan/requirements.yaml"),
            copy.join("plan/requirements.yaml"),
        )
        .unwrap();
        for entry in std::fs::read_dir(root().join("fixtures/gates").join(oklab::NAME)).unwrap() {
            let path = entry.unwrap().path();
            std::fs::copy(&path, dir.join(path.file_name().unwrap())).unwrap();
        }
        let perturbed = dir.join("perturbed_m1.json");
        let text = std::fs::read_to_string(&perturbed)
            .unwrap()
            .replace(r#""expected": "fail""#, r#""expected": "pass""#);
        std::fs::write(&perturbed, text).unwrap();
        check_fixtures(&copy)
    }
);
