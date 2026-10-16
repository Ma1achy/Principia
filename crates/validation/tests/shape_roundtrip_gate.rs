//! The `shape-roundtrip` gate (TASK-M2-08; REQ-CHART-020, REQ-CHART-053, PIT-3): its samples and their order, the miss
//! of the closed-form and the superseded inverse, the verdict at and beyond the threshold, the refusals, the report's
//! sections, and the runner on `fixtures/gates/shape-roundtrip/`, where the superseded inverse fails.

use std::path::{Path, PathBuf};

use validation::gate::{self, Gate, GateConfig, Verdict};
use validation::negative_control;
use validation::shape_roundtrip::{
    self, miss, samples, verdict, Input, InverseKind, Sample, ShapeRoundtripGate,
};
use validation::SHAPE_ROUNDTRIP_BOUND;

const PI: f64 = std::f64::consts::PI;

/// This workspace's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn input(samples: u32, near: u32, poles: u32, inverse: InverseKind) -> Input {
    Input {
        description: "a test input".to_owned(),
        seed: 3,
        samples,
        near_pole_samples: near,
        pole_samples: poles,
        inverse,
    }
}

// ── The samples ──────────────────────────────────────────────────────────────────────────────────────────────────

/// `list` is `input`'s samples: `samples` random points, then `near_pole_samples` about `n₀ = +1` and as many about
/// `−1` at polar radii in `[10⁻¹⁵, 10⁻²]`, then `pole_samples` at each of the eight poles in the order `n₀ = 1, −1`,
/// `n₁ = +0, −0`, `n₂ = +0, −0`; every point unit, every draw in its range; and the same seed draws the same list.
fn check_samples(input: &Input, list: &[Sample]) {
    let (k, near, poles) = (
        input.samples as usize,
        input.near_pole_samples as usize,
        input.pole_samples as usize,
    );
    assert_eq!(list.len(), k + 2 * near + 8 * poles, "the sample count");
    for s in list {
        let norm = s.n.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() <= 1e-14, "the point {:?} is not unit", s.n);
        assert!(
            s.z_mass.iter().all(|z| z.abs() <= 4.0)
                && (0.25..=4.0).contains(&s.inertia)
                && s.phase.abs() <= PI,
            "a draw out of its range: {s:?}"
        );
    }
    for (i, s) in list[k..k + 2 * near].iter().enumerate() {
        let sign = if i < near { 1.0 } else { -1.0 };
        let radius = s.n[1].hypot(s.n[2]);
        assert!(
            s.n[0] * sign > 0.0 && radius <= 1.01e-2 && radius >= 0.99e-15,
            "near-pole sample {i} at {:?}, polar radius {radius:e}",
            s.n
        );
    }
    let mut want = Vec::new();
    for n0 in [1.0f64, -1.0] {
        for n1 in [0.0f64, -0.0] {
            for n2 in [0.0f64, -0.0] {
                want.extend(std::iter::repeat_n([n0, n1, n2].map(f64::to_bits), poles));
            }
        }
    }
    let got: Vec<[u64; 3]> = list[k + 2 * near..]
        .iter()
        .map(|s| s.n.map(f64::to_bits))
        .collect();
    assert!(got == want, "the poles are not the eight (±1, ±0, ±0) in order");
    assert_eq!(samples(input), samples(input), "one seed, one list");
}

#[test]
fn shape_roundtrip_gate_samples() {
    let i = input(64, 8, 2, InverseKind::ClosedForm);
    check_samples(&i, &samples(&i));
    let lower = samples(&input(4096, 0, 0, InverseKind::ClosedForm))
        .iter()
        .filter(|s| s.n[2] < 0.0)
        .count();
    assert!((1700..2400).contains(&lower), "{lower} of 4096 on the lower hemisphere");
    assert_ne!(
        samples(&i),
        samples(&Input { seed: 4, ..i.clone() }),
        "another seed draws another list"
    );
}

negative_control!(
    shape_roundtrip_gate_samples,
    "a list missing its last pole must fail",
    expected = "the sample count",
    {
        let i = input(64, 8, 2, InverseKind::ClosedForm);
        let mut list = samples(&i);
        list.pop();
        check_samples(&i, &list)
    }
);

// ── The miss ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// Over `input`'s samples, the closed form misses by at most `1e-14` and the superseded inverse by more than `|n₂|`
/// wherever `|n₂| > 1e-3`; on the equator, `n₂ = 0`, both are within `1e-14`.
fn check_miss(of: fn(&Sample, InverseKind) -> (f64, f64)) {
    for s in samples(&input(2048, 64, 2, InverseKind::ClosedForm)) {
        let (a, b) = of(&s, InverseKind::ClosedForm);
        assert!(a.max(b) <= 1e-14, "the closed form misses {:?} by {a:e}, {b:e}", s.n);
        let (c, d) = of(&s, InverseKind::NegativeCross);
        if s.n[2].abs() > 1e-3 {
            assert!(
                c.min(d) > s.n[2].abs(),
                "the superseded inverse misses {:?} by only {c:e}, {d:e}",
                s.n
            );
        }
        let chi = s.n[1].atan2(s.n[0]);
        let equator = Sample {
            n: [chi.cos(), chi.sin(), 0.0],
            ..s
        };
        let (e, f) = of(&equator, InverseKind::NegativeCross);
        assert!(
            e.max(f) <= 1e-14,
            "on the equator the superseded inverse misses {:?} by {e:e}, {f:e}",
            equator.n
        );
    }
}

#[test]
fn shape_roundtrip_gate_miss() {
    check_miss(miss);
}

negative_control!(
    shape_roundtrip_gate_miss,
    "a measure that builds the superseded inverse for both must fail",
    expected = "the closed form misses",
    check_miss(|s, _| miss(s, InverseKind::NegativeCross))
);

// ── The verdict, the refusals and the report ─────────────────────────────────────────────────────────────────────

/// The gate's configuration as `gate.json` gives it.
fn config() -> GateConfig {
    gate::config(&root(), shape_roundtrip::NAME).unwrap()
}

/// `of` passes a worst at or below the threshold and fails one beyond it.
fn check_verdict(of: fn(f64, f64) -> Verdict) {
    let t = 1e-13;
    assert_eq!(of(t, t), Verdict::Pass, "a worst at the threshold");
    assert_eq!(of(t * 0.5, t), Verdict::Pass, "a worst below the threshold");
    assert_eq!(of(f64::from_bits(t.to_bits() + 1), t), Verdict::Fail, "a worst one ulp beyond the threshold");
}

#[test]
fn shape_roundtrip_gate_verdict() {
    check_verdict(verdict);
}

negative_control!(
    shape_roundtrip_gate_verdict,
    "a strict comparison fails a worst at the threshold, so it must fail",
    expected = "a worst at the threshold",
    check_verdict(|w, t| if w < t { Verdict::Pass } else { Verdict::Fail })
);

/// `run` refuses a threshold that is not the register's, either side of it, and an input with no samples, and accepts
/// the register's threshold with one sample.
fn check_refusals(run: fn(&GateConfig, &Input) -> Result<(), String>) {
    let register = SHAPE_ROUNDTRIP_BOUND.number();
    let one = input(1, 0, 0, InverseKind::ClosedForm);
    for value in [
        f64::from_bits(register.to_bits() + 1),
        f64::from_bits(register.to_bits() - 1),
    ] {
        let mut c = config();
        c.threshold.value = value;
        let got = run(&c, &one);
        assert!(
            got.as_ref().is_err_and(|e| e.contains("is not the register's")),
            "the threshold {value:e} is not refused: {got:?}"
        );
    }
    let got = run(&config(), &input(0, 4, 4, InverseKind::ClosedForm));
    assert!(
        got.as_ref().is_err_and(|e| e.contains("no samples")),
        "no samples is not refused: {got:?}"
    );
    assert_eq!(run(&config(), &one), Ok(()), "one sample at the register's threshold");
}

#[test]
fn shape_roundtrip_gate_refusals() {
    check_refusals(|c, i| ShapeRoundtripGate::judge(c, "t", i).map(|_| ()));
    let value = serde_json::json!({ "description": "x", "seed": 1 });
    let got = ShapeRoundtripGate.run(&config(), "bad.json", &value);
    assert!(
        got.as_ref().is_err_and(|e| e.contains("is not a shape-roundtrip input")),
        "an input missing its fields is not refused: {got:?}"
    );
}

negative_control!(
    shape_roundtrip_gate_refusals,
    "a gate refusing nothing must fail",
    expected = "is not refused",
    check_refusals(|_, _| Ok(()))
);

/// The report on a small input: the verdict, the threshold, the scatter's worst, paths, hemispheres and poles, the
/// margin or the count beyond, and the region count, for the closed form (pass) and the superseded inverse (fail).
fn check_report(judge: fn(&GateConfig, &Input) -> Result<validation::gate::GateReport, String>) {
    let pass = judge(&config(), &input(256, 16, 2, InverseKind::ClosedForm)).unwrap();
    assert_eq!(pass.verdict, Verdict::Pass, "the closed form's verdict");
    assert_eq!(pass.gate, shape_roundtrip::NAME);
    let text = pass.scatter.join("\n");
    for want in ["max by path", "on the lower hemisphere", "near n₀ = ±1", "at the eight poles", "margin"] {
        assert!(text.contains(want), "the scatter lacks `{want}`: {text}");
    }
    assert!(
        pass.negative_results.iter().any(|l| l.starts_with("0 of 304 samples beyond")),
        "the pass's negative results: {:?}",
        pass.negative_results
    );
    let fail = judge(&config(), &input(256, 16, 2, InverseKind::NegativeCross)).unwrap();
    assert_eq!(fail.verdict, Verdict::Fail, "the superseded inverse's verdict");
    assert!(
        fail.negative_results.iter().any(|l| l.contains("negative-cross inverse")),
        "the fail's negative results: {:?}",
        fail.negative_results
    );
    assert!(pass.check().is_ok() && fail.check().is_ok(), "a report section is empty");
}

#[test]
fn shape_roundtrip_gate_report() {
    check_report(|c, i| ShapeRoundtripGate::judge(c, "t", i));
}

negative_control!(
    shape_roundtrip_gate_report,
    "a judge that always measures the closed form passes the superseded inverse, so it must fail",
    expected = "the superseded inverse's verdict",
    check_report(|c, i| ShapeRoundtripGate::judge(
        c,
        "t",
        &Input {
            inverse: InverseKind::ClosedForm,
            ..i.clone()
        }
    ))
);

// ── The fixtures ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The runner on `root`'s `fixtures/gates/shape-roundtrip/` gives every input its expected outcome: `random.json`
/// passes and `negative_cross.json` fails.
fn check_fixtures(root: &Path) {
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("shape_roundtrip_gate");
    let result = gate::run(root, shape_roundtrip::NAME, &out);
    assert!(
        result.is_ok(),
        "the gate's fixtures do not give their expected outcomes: {result:?}"
    );
    let random = shape_roundtrip::input(root, "random.json").unwrap();
    assert_eq!(random.inverse, InverseKind::ClosedForm, "random.json's inverse");
}

#[test]
fn shape_roundtrip_gate_fixtures() {
    check_fixtures(&root());
}

negative_control!(
    shape_roundtrip_gate_fixtures,
    "the superseded inverse's fixture expected to pass must fail the run",
    expected = "do not give their expected outcomes",
    {
        let copy = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("shape_roundtrip_copy");
        let dir = copy.join("fixtures/gates").join(shape_roundtrip::NAME);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(copy.join("plan")).unwrap();
        std::fs::copy(
            root().join("plan/requirements.yaml"),
            copy.join("plan/requirements.yaml"),
        )
        .unwrap();
        for entry in
            std::fs::read_dir(root().join("fixtures/gates").join(shape_roundtrip::NAME)).unwrap()
        {
            let path = entry.unwrap().path();
            std::fs::copy(&path, dir.join(path.file_name().unwrap())).unwrap();
        }
        let negative = dir.join("negative_cross.json");
        let text = std::fs::read_to_string(&negative)
            .unwrap()
            .replace(r#""expected": "fail""#, r#""expected": "pass""#);
        std::fs::write(&negative, text).unwrap();
        check_fixtures(&copy)
    }
);
