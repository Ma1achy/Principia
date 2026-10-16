//! The `shape-roundtrip` gate (TASK-M2-08; REQ-CHART-020, REQ-CHART-053, PIT-3): its samples and their order, the miss
//! of the closed-form and the superseded inverse, the verdict at and beyond the threshold, the refusals, the report's
//! sections, and the runner on `fixtures/gates/shape-roundtrip/`, where the superseded inverse fails.

use std::path::{Path, PathBuf};

use kernel::chart::shape_sphere::{exponential, spherical};
use validation::decode_f32_eps::SplitMix;
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
        assert!(
            (norm - 1.0).abs() <= 1e-14,
            "the point {:?} is not unit",
            s.n
        );
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
            s.n[0] * sign > 0.0 && (0.99e-15..=1.01e-2).contains(&radius),
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
    assert!(
        got == want,
        "the poles are not the eight (±1, ±0, ±0) in order"
    );
    assert_eq!(samples(input), samples(input), "one seed, one list");
}

/// A draw on `[lo, hi]` from `rng`, as the gate draws: 53 random bits.
fn on(rng: &mut SplitMix, lo: f64, hi: f64) -> f64 {
    lo + (hi - lo) * ((rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64)
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let k = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    a.map(|x| x / k)
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The masses' and phase's draws take both signs, as do the near-pole points' `n₂`.
fn check_signs(input: &Input, list: &[Sample]) {
    let (k, near) = (input.samples as usize, input.near_pole_samples as usize);
    let both = |xs: Vec<f64>| xs.iter().any(|&x| x < 0.0) && xs.iter().any(|&x| x > 0.0);
    assert!(
        both(list.iter().map(|s| s.z_mass[0]).collect())
            && both(list.iter().map(|s| s.z_mass[1]).collect())
            && both(list.iter().map(|s| s.phase).collect()),
        "the mass and phase draws do not take both signs"
    );
    assert!(
        both(list[k..k + 2 * near].iter().map(|s| s.n[2]).collect()),
        "the near-pole points' n₂ does not take both signs"
    );
}

/// The first three samples are, from the seed's stream, a spherical point, an exponential one and a spherical one,
/// each followed by its four gauge draws.
fn check_order(input: &Input, list: &[Sample]) {
    let mut rng = SplitMix::new(input.seed);
    let mut want = Vec::new();
    for j in 0..3 {
        let n = if j == 1 {
            let n0 = unit([0, 1, 2].map(|_| on(&mut rng, -1.0, 1.0)));
            let e1 = unit(cross(n0, [0, 1, 2].map(|_| on(&mut rng, -1.0, 1.0))));
            let (scale, u, v) = (
                on(&mut rng, 0.0, PI),
                on(&mut rng, 0.0, 1.0),
                on(&mut rng, 0.0, 1.0),
            );
            exponential(n0, e1, cross(n0, e1), scale, u, v)
        } else {
            spherical(on(&mut rng, 0.0, 1.0), on(&mut rng, 0.0, 1.0), PI)
        };
        let z_mass = [on(&mut rng, -4.0, 4.0), on(&mut rng, -4.0, 4.0)];
        let (inertia, phase) = (on(&mut rng, 0.25, 4.0), on(&mut rng, -PI, PI));
        want.push(Sample {
            n,
            z_mass,
            inertia,
            phase,
        });
    }
    assert_eq!(
        list[..3],
        want[..],
        "the first three samples are not drawn in order"
    );
}

#[test]
fn shape_roundtrip_gate_samples() {
    let i = input(64, 8, 2, InverseKind::ClosedForm);
    check_samples(&i, &samples(&i));
    let lower = samples(&input(4096, 0, 0, InverseKind::ClosedForm))
        .iter()
        .filter(|s| s.n[2] < 0.0)
        .count();
    assert!(
        (1700..2400).contains(&lower),
        "{lower} of 4096 on the lower hemisphere"
    );
    assert_ne!(
        samples(&i),
        samples(&Input {
            seed: 4,
            ..i.clone()
        }),
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

#[test]
fn shape_roundtrip_gate_samples_order() {
    let i = input(64, 8, 2, InverseKind::ClosedForm);
    check_order(&i, &samples(&i));
}

negative_control!(
    shape_roundtrip_gate_samples_order,
    "a list whose first two points are swapped must fail",
    expected = "the first three samples are not drawn in order",
    {
        let i = input(64, 8, 2, InverseKind::ClosedForm);
        let mut list = samples(&i);
        list.swap(0, 1);
        check_order(&i, &list)
    }
);

#[test]
fn shape_roundtrip_gate_samples_signs() {
    let i = input(64, 8, 2, InverseKind::ClosedForm);
    check_signs(&i, &samples(&i));
}

negative_control!(
    shape_roundtrip_gate_samples_signs,
    "a list whose phases are all positive must fail",
    expected = "the mass and phase draws do not take both signs",
    {
        let i = input(64, 8, 2, InverseKind::ClosedForm);
        let mut list = samples(&i);
        for s in &mut list {
            s.phase = s.phase.abs();
        }
        check_signs(&i, &list)
    }
);

// ── The miss ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// Over `input`'s samples, the closed form misses by at most `1e-14` and the superseded inverse by more than `|n₂|`
/// wherever `|n₂| > 1e-3`; on the equator, `n₂ = 0`, both are within `1e-14`.
fn check_miss(of: fn(&Sample, InverseKind) -> (f64, f64)) {
    for s in samples(&input(2048, 64, 2, InverseKind::ClosedForm)) {
        let (a, b) = of(&s, InverseKind::ClosedForm);
        assert!(
            a.max(b) <= 1e-14,
            "the closed form misses {:?} by {a:e}, {b:e}",
            s.n
        );
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
    assert_eq!(
        of(f64::from_bits(t.to_bits() + 1), t),
        Verdict::Fail,
        "a worst one ulp beyond the threshold"
    );
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
            got.as_ref()
                .is_err_and(|e| e.contains("is not the register's")),
            "the threshold {value:e} is not refused: {got:?}"
        );
    }
    let got = run(&config(), &input(0, 4, 4, InverseKind::ClosedForm));
    assert!(
        got.as_ref().is_err_and(|e| e.contains("no samples")),
        "no samples is not refused: {got:?}"
    );
    assert_eq!(
        run(&config(), &one),
        Ok(()),
        "one sample at the register's threshold"
    );
}

#[test]
fn shape_roundtrip_gate_refusals() {
    check_refusals(|c, i| ShapeRoundtripGate::judge(c, "t", i).map(|_| ()));
    let value = serde_json::json!({ "description": "x", "seed": 1 });
    let got = ShapeRoundtripGate.run(&config(), "bad.json", &value);
    assert!(
        got.as_ref()
            .is_err_and(|e| e.contains("is not a shape-roundtrip input")),
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
    for want in [
        "max by path",
        "on the lower hemisphere",
        "near n₀ = ±1",
        "at the eight poles",
        "margin",
    ] {
        assert!(text.contains(want), "the scatter lacks `{want}`: {text}");
    }
    assert!(
        pass.negative_results
            .iter()
            .any(|l| l.starts_with("0 of 304 samples beyond")),
        "the pass's negative results: {:?}",
        pass.negative_results
    );
    let fail = judge(&config(), &input(256, 16, 2, InverseKind::NegativeCross)).unwrap();
    assert_eq!(
        fail.verdict,
        Verdict::Fail,
        "the superseded inverse's verdict"
    );
    assert!(
        fail.negative_results
            .iter()
            .any(|l| l.contains("negative-cross inverse")),
        "the fail's negative results: {:?}",
        fail.negative_results
    );
    assert!(
        pass.check().is_ok() && fail.check().is_ok(),
        "a report section is empty"
    );
    check_numbers(&pass, &input(256, 16, 2, InverseKind::ClosedForm));
    check_numbers(&fail, &input(256, 16, 2, InverseKind::NegativeCross));
}

/// `report`'s numbers are `input`'s, measured here from `samples` and `miss`: the worst miss and the first sample
/// giving it, the random samples on the lower hemisphere, the near-pole and the pole maxima, and on a fail the
/// samples beyond the threshold.
fn check_numbers(report: &validation::gate::GateReport, input: &Input) {
    let all = samples(input);
    let misses: Vec<f64> = all
        .iter()
        .map(|s| {
            let (a, b) = miss(s, input.inverse);
            a.max(b)
        })
        .collect();
    let (random, near) = (input.samples as usize, 2 * input.near_pole_samples as usize);
    let worst = misses.iter().copied().fold(0.0, f64::max);
    let at = misses.iter().position(|&x| x == worst).unwrap();
    let lower = all[..random].iter().filter(|s| s.n[2] < 0.0).count();
    let near_max = misses[random..random + near]
        .iter()
        .copied()
        .fold(0.0, f64::max);
    let pole_max = misses[random + near..].iter().copied().fold(0.0, f64::max);
    let text = report.scatter.join("\n");
    for want in [
        format!("max {worst:.3e} at sample {at} "),
        format!("{random} random samples, {lower} on the lower hemisphere"),
        format!("{near} near n₀ = ±1 at polar radii 10⁻² to 10⁻¹⁵, their max {near_max:.3e}; "),
        format!(
            "{} at the eight poles (±1, ±0, ±0), their max {pole_max:.3e}",
            misses.len() - random - near
        ),
    ] {
        assert!(text.contains(&want), "the scatter lacks `{want}`: {text}");
    }
    if report.verdict == Verdict::Fail {
        let threshold = SHAPE_ROUNDTRIP_BOUND.number();
        let over = misses.iter().filter(|&&x| x > threshold).count();
        let want = format!(
            "{over} of {} samples beyond the threshold {threshold:.3e}; the worst, {worst:.3e} at sample {at}, fails",
            misses.len()
        );
        assert!(
            report.negative_results.contains(&want),
            "the fail's negative results lack `{want}`: {:?}",
            report.negative_results
        );
    }
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

/// The judge's reports on the closed form and on the superseded inverse carry their numbers.
fn check_report_numbers(
    judge: fn(&GateConfig, &Input) -> Result<validation::gate::GateReport, String>,
) {
    for inverse in [InverseKind::ClosedForm, InverseKind::NegativeCross] {
        let i = input(256, 16, 2, inverse);
        check_numbers(&judge(&config(), &i).unwrap(), &i);
    }
}

#[test]
fn shape_roundtrip_gate_report_numbers() {
    check_report_numbers(|c, i| ShapeRoundtripGate::judge(c, "t", i));
}

negative_control!(
    shape_roundtrip_gate_report_numbers,
    "a fail that counts one sample too many beyond the threshold must fail",
    expected = "the fail's negative results lack",
    check_report_numbers(|c, i| {
        let mut r = ShapeRoundtripGate::judge(c, "t", i)?;
        if r.verdict == Verdict::Fail {
            let line = &r.negative_results[0];
            let (n, rest) = line.split_once(' ').unwrap();
            r.negative_results[0] = format!("{} {rest}", n.parse::<usize>().unwrap() + 1);
        }
        Ok(r)
    })
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
    assert_eq!(
        random.inverse,
        InverseKind::ClosedForm,
        "random.json's inverse"
    );
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
