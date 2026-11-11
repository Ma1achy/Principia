//! The Benettin measurements and their gates (`validation::measure::benettin`; TASK-M3-14):
//! - `lagrange_known_exponent`: Routh's exponent of Lagrange's orbit, `½·√(√(27β) − 1)·ω` where `27β > 1` and 0 where
//!   not, at the equal-mass, `(0.5, 0.3, 0.2)` and `(0.98, 0.01, 0.01)` masses, against the roots of the rotating
//!   frame's characteristic equation (computed independently, to f64 rounding); the orbit's canonical `ω`.
//! - `test10_change`: dd test 10's change is the largest of the three variants' differences over `max(|λ_T|, 1/T)`,
//!   infinite where a read is not finite.
//! - `calibration_rule`: the rule picks the smallest worst change, ties at two significant digits going to the larger
//!   `n_renorm`, then the larger `‖δ₀‖`; the tolerance is the smallest one-digit value at or above twice it.
//! - `calibration_gate`: the gate's `test10` inputs pass at the proposed values on Lagrange's orbit and fail its control,
//!   and refuse an unknown orbit or precision; `sweep` passes only where the rule picks the proposal.
//! - `orbit_fixtures`: the orbits are what they are named: the Kepler-embedded pair's eccentricity 1/2 and its light
//!   third body's circular orbit, Burrau's (3, 4, 5) triangle at rest, Lagrange's equilateral triangle in rigid
//!   rotation at `ω² = M/a³`, all in the sim's units (`M = 1`, `I = 1`, CoM at rest at the origin).
//! - `shadow_form_gate`: its controls give their expected outcomes, and its measurement covers the periodic set at both
//!   `‖δ₀‖`s with every value finite.

use std::path::Path;

use kernel::decode::Ic;
use validation::gate::Gate;
use validation::gate::{self, GateSpec, MeasurementGate, Verdict};
use validation::measure::benettin::{
    burrau, calibration_orbit, figure_eight, kepler_embedded, lagrange, lagrange_exponent,
    periodic_set, routh_beta, select, tolerance, BenettinCalibrationGate, Candidate, Precision,
    ShadowFormGate, Test10, PROPOSED_DELTA_0, PROPOSED_N_RENORM, PROPOSED_TOLERANCE,
};
use validation::negative_control;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

// ── lagrange_known_exponent ─────────────────────────────────────────────────────────────────────────────────────

/// The largest real part of the roots `s` of `s⁴ + s² + (27/4)β = 0`, the exponent in units of `ω`, from the
/// quadratic in `s²` solved in complex arithmetic by hand.
fn routh_roots(beta: f64) -> f64 {
    let c = 27.0 / 4.0 * beta;
    let disc = 1.0 - 4.0 * c;
    if disc >= 0.0 {
        // s² real and negative (both roots of the quadratic are negative): s imaginary.
        0.0
    } else {
        // s² = (−1 ± i·√(−disc))/2; Re √z = √((|z| + Re z)/2).
        let (re, im) = (-0.5, (-disc).sqrt() / 2.0);
        let modulus = (re * re + im * im).sqrt();
        ((modulus + re) / 2.0).sqrt()
    }
}

fn check_exponent(exponent: fn([f64; 3]) -> f64) {
    for m in [
        [1.0 / 3.0; 3],
        [0.5, 0.3, 0.2],
        [0.98, 0.01, 0.01],
        [0.9, 0.05, 0.05],
    ] {
        let want = routh_roots(routh_beta(m));
        let got = exponent(m);
        assert!(
            (got - want).abs() <= 1e-15,
            "masses {m:?}: the exponent {got} is not Routh's {want}"
        );
    }
}

#[test]
fn lagrange_known_exponent() {
    check_exponent(lagrange_exponent);
    assert!((routh_beta([1.0 / 3.0; 3]) - 1.0 / 3.0).abs() < 1e-16);
    // β is a ratio: masses not summing to 1 give the same β as the same masses normalised.
    assert!((routh_beta([1.0; 3]) - 1.0 / 3.0).abs() < 1e-16);
    assert!((routh_beta([5.0, 3.0, 2.0]) - routh_beta([0.5, 0.3, 0.2])).abs() < 1e-16);
    assert!((lagrange_exponent([1.0 / 3.0; 3]) - 0.5f64.sqrt()).abs() < 1e-15);
    // The equal-mass orbit in canonical units: side √3, ω = 3^(−3/4), its exponent ω/√2.
    let known = lagrange("equal", [1.0 / 3.0; 3], 30.0)
        .known
        .expect("known");
    assert!(
        (known - 3f64.powf(-0.75) / 2f64.sqrt()).abs() < 1e-12,
        "{known}"
    );
    assert_eq!(
        lagrange("stable", [0.98, 0.01, 0.01], 30.0).known,
        Some(0.0)
    );
    assert_eq!(figure_eight(30.0).known, Some(0.0));
    assert_eq!(periodic_set().len(), 4);
}

negative_control!(
    lagrange_known_exponent,
    "the other root's real part must fail",
    expected = "is not Routh's",
    check_exponent(|m| {
        let b = 27.0 * routh_beta(m);
        if b > 1.0 {
            0.5 * (b.sqrt() + 1.0).sqrt()
        } else {
            0.0
        }
    })
);

// ── test10_change ───────────────────────────────────────────────────────────────────────────────────────────────

fn t10(lambdas: [f64; 4], horizon: f64) -> Test10 {
    Test10 {
        delta_0: 1e-3,
        n_renorm: 10,
        horizon,
        lambdas,
    }
}

fn check_change(change: fn(&Test10) -> f64) {
    let cases = [
        (t10([0.5, 0.55, 0.45, 0.5], 50.0), 0.1),
        (t10([0.5, 0.5, 0.5, 0.4], 50.0), 0.2),
        (t10([-0.5, -0.5, -0.45, -0.5], 50.0), 0.1),
        (t10([0.001, 0.011, 0.001, 0.001], 10.0), 0.1),
        (t10([0.2, 0.2, 0.2, 0.2], 10.0), 0.0),
    ];
    for (t, want) in cases {
        let got = change(&t);
        assert!(
            (got - want).abs() < 1e-12,
            "{:?}: the change {got} is not {want}",
            t.lambdas
        );
    }
    for bad in [f64::NAN, f64::INFINITY] {
        for k in 0..4 {
            let mut l = [0.5; 4];
            l[k] = bad;
            assert_eq!(
                change(&t10(l, 50.0)),
                f64::INFINITY,
                "the change is not {}",
                f64::INFINITY
            );
        }
    }
}

#[test]
fn test10_change() {
    check_change(Test10::change);
}

negative_control!(
    test10_change,
    "a change without the 1/T floor must fail",
    expected = "is not",
    check_change(|t| {
        let [l, rest @ ..] = t.lambdas;
        rest.iter().map(|x| (x - l).abs()).fold(0.0, f64::max) / l.abs()
    })
);

// ── calibration_rule ────────────────────────────────────────────────────────────────────────────────────────────

/// A candidate whose one run's change is `worst`.
fn cand(delta_0: f64, n_renorm: u32, worst: f64) -> Candidate {
    Candidate {
        delta_0,
        n_renorm,
        runs: vec![(
            "synthetic",
            Precision::F64,
            t10([1.0, 1.0 + worst, 1.0, 1.0], 50.0),
        )],
    }
}

fn check_rule(tol: fn(f64) -> f64) {
    for (worst, want) in [
        (0.0944, 0.2),
        (0.1, 0.2),
        (0.05, 0.1),
        (0.04, 0.08),
        (0.46, 1.0),
        (0.5, 1.0),
        (0.051, 0.2),
        (0.0004, 0.0008),
    ] {
        assert_eq!(
            tol(worst),
            want,
            "the tolerance from worst change {worst} is not {want}"
        );
    }
    let picks = |c: Vec<Candidate>| {
        let s = select(&c).expect("a candidate");
        (s.delta_0, s.n_renorm)
    };
    assert_eq!(
        picks(vec![
            cand(1e-3, 10, 0.3),
            cand(1e-4, 100, 0.2),
            cand(1e-2, 1, 0.25)
        ]),
        (1e-4, 100)
    );
    // Tied at two digits: the larger n_renorm, then the larger ‖δ₀‖.
    assert_eq!(
        picks(vec![cand(1e-3, 10, 0.1234), cand(1e-4, 100, 0.1225)]),
        (1e-4, 100)
    );
    assert_eq!(
        picks(vec![cand(1e-4, 100, 0.1225), cand(1e-3, 10, 0.1234)]),
        (1e-4, 100)
    );
    assert_eq!(
        picks(vec![cand(1e-4, 100, 0.12), cand(1e-3, 100, 0.12)]),
        (1e-3, 100)
    );
    assert_eq!(
        picks(vec![cand(1e-3, 100, 0.12), cand(1e-4, 100, 0.12)]),
        (1e-3, 100)
    );
    // Not tied: the smaller worst, whatever n_renorm.
    assert_eq!(
        picks(vec![cand(1e-3, 10, 0.11), cand(1e-4, 100, 0.12)]),
        (1e-3, 10)
    );
    assert!(select(&[]).is_none());
    assert_eq!(cand(1e-3, 10, f64::NAN).worst(), f64::INFINITY);
}

#[test]
fn calibration_rule() {
    check_rule(tolerance);
    assert_eq!(tolerance(0.0944), PROPOSED_TOLERANCE);
}

negative_control!(
    calibration_rule,
    "a tolerance without the margin of two must fail",
    expected = "is not",
    check_rule(|w| tolerance(w / 2.0))
);

// ── calibration_gate ────────────────────────────────────────────────────────────────────────────────────────────

fn calibration_config() -> gate::GateConfig {
    gate::config(root(), "benettin-calibration").expect("the gate's gate.json")
}

fn run(input: serde_json::Value) -> Result<Verdict, String> {
    BenettinCalibrationGate
        .run(&calibration_config(), "test", &input)
        .map(|r| {
            r.check().expect("a complete report");
            r.verdict
        })
}

fn check_gate_test10(expect_control: Verdict) {
    let input = |extra: serde_json::Value| {
        let mut v = serde_json::json!({ "kind": "test10", "description": "t", "orbit": "lagrange", "precision": "f64" });
        v.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        v
    };
    assert_eq!(run(input(serde_json::json!({}))), Ok(Verdict::Pass));
    assert_eq!(
        run(input(
            serde_json::json!({ "orbit": "burrau", "precision": "f32", "delta_0": 1e-4, "n_renorm": 10 })
        )),
        Ok(expect_control),
        "the control is not told apart"
    );
    assert!(run(input(serde_json::json!({ "orbit": "pluto" }))).is_err());
    assert!(run(input(serde_json::json!({ "precision": "f16" }))).is_err());
    assert!(run(serde_json::json!({ "kind": "neither", "description": "t" })).is_err());
}

#[test]
fn calibration_gate() {
    assert_eq!(calibration_config().threshold.value, PROPOSED_TOLERANCE);
    check_gate_test10(Verdict::Fail);
    assert!(calibration_orbit("kepler").is_some() && calibration_orbit("burrau").is_some());
    // The sweep: the rule picks the proposal, and the gate's tolerance is the rule's.
    assert_eq!(
        run(serde_json::json!({ "kind": "sweep", "description": "t" })),
        Ok(Verdict::Pass)
    );
    assert_eq!((PROPOSED_DELTA_0, PROPOSED_N_RENORM), (3e-3, 300));
}

negative_control!(
    calibration_gate,
    "a gate that cannot tell the control from the proposal must fail",
    expected = "the control is not told apart",
    check_gate_test10(Verdict::Pass)
);

// ── shadow_form_gate ────────────────────────────────────────────────────────────────────────────────────────────

fn check_form_controls(doubled: f64) {
    let GateSpec::Measurement(config) = gate::spec(root(), "shadow-form").expect("gate.json")
    else {
        panic!("shadow-form is not a measurement");
    };
    let control = |v: serde_json::Value| ShadowFormGate.control(&config, "t", &v);
    let same = control(serde_json::json!({ "kind": "compare", "description": "s", "scale": 1.0 }))
        .expect("runs");
    assert_eq!(same.verdict, Verdict::Pass);
    let other =
        control(serde_json::json!({ "kind": "compare", "description": "d", "scale": doubled }))
            .expect("runs");
    assert_eq!(
        other.verdict,
        Verdict::Fail,
        "the comparison cannot tell two reads apart"
    );
    let set =
        control(serde_json::json!({ "kind": "periodic-set", "description": "p" })).expect("runs");
    assert_eq!(set.verdict, Verdict::Pass);
    for o in periodic_set() {
        assert_eq!(
            set.markdown.matches(o.name).count(),
            2,
            "{} not measured twice",
            o.name
        );
    }
    assert!(ShadowFormGate.flags(root(), &["--measure"]).is_err());
    assert!(control(serde_json::json!({ "kind": "compare", "description": "x" })).is_err());
}

#[test]
fn shadow_form_gate() {
    check_form_controls(2.0);
}

negative_control!(
    shadow_form_gate,
    "a control at the same ‖δ₀‖ must not be told apart",
    expected = "the comparison cannot tell two reads apart",
    check_form_controls(1.0)
);

// ── orbit_fixtures ──────────────────────────────────────────────────────────────────────────────────────────────

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn norm(a: [f64; 2]) -> f64 {
    a[0].hypot(a[1])
}

fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

/// `M = 1`, `I = 1`, the centre of mass at the origin and at rest.
fn check_units(ic: &Ic<f64>, what: &str) {
    let m = ic.m;
    let total = m[0] + m[1] + m[2];
    let i: f64 = (0..3).map(|k| m[k] * dot(ic.r[k], ic.r[k])).sum();
    let com = [0, 1].map(|c| (0..3).map(|k| m[k] * ic.r[k][c]).sum::<f64>());
    let mom = [0, 1].map(|c| (0..3).map(|k| ic.p[k][c]).sum::<f64>());
    assert!((total - 1.0).abs() < 1e-12, "{what}: M = {total}");
    assert!((i - 1.0).abs() < 1e-12, "{what}: I = {i}");
    assert!(
        norm(com) < 1e-12 && norm(mom) < 1e-12,
        "{what}: CoM {com:?}, P {mom:?}"
    );
}

#[test]
fn orbit_fixtures() {
    // The Kepler-embedded orbit: the pair's eccentricity from its relative orbit, and body 2 circular about the pair.
    let k = kepler_embedded().ic;
    check_units(&k, "Kepler-embedded");
    assert!((k.m[2] - 1e-9).abs() < 1e-21 && (k.m[0] - k.m[1]).abs() < 1e-15);
    let (mp, mu) = (k.m[0] + k.m[1], k.m[0] * k.m[1] / (k.m[0] + k.m[1]));
    let rel = sub(k.r[1], k.r[0]);
    let vrel = sub(
        [k.p[1][0] / k.m[1], k.p[1][1] / k.m[1]],
        [k.p[0][0] / k.m[0], k.p[0][1] / k.m[0]],
    );
    let energy = mu * dot(vrel, vrel) / 2.0 - k.m[0] * k.m[1] / norm(rel);
    let a = -k.m[0] * k.m[1] / (2.0 * energy);
    let h = cross(rel, vrel);
    let e = (1.0 - h * h / (mp * a)).sqrt();
    assert!((e - 0.5).abs() < 1e-9, "the pair's eccentricity is {e}");
    assert!(
        dot(rel, vrel).abs() < 1e-12,
        "the pair does not start at an apsis"
    );
    assert!(norm(rel) > a, "the pair does not start at apocentre");
    let c = [0, 1].map(|j| (k.m[0] * k.r[0][j] + k.m[1] * k.r[1][j]) / mp);
    let vc = [0, 1].map(|j| (k.p[0][j] + k.p[1][j]) / mp);
    let (r2, v2) = (
        sub(k.r[2], c),
        sub([k.p[2][0] / k.m[2], k.p[2][1] / k.m[2]], vc),
    );
    assert!(
        dot(r2, v2).abs() < 1e-9 * norm(r2) * norm(v2),
        "body 2 moves radially"
    );
    assert!(
        (dot(v2, v2) * norm(r2) - 1.0).abs() < 1e-6,
        "body 2 is not circular"
    );
    assert!(
        (norm(r2) / a - 20.0).abs() < 1e-9,
        "body 2 is not at 20 semi-major axes"
    );
    // Burrau: the (3, 4, 5) triangle at rest, masses (5, 4, 3)/12 opposite the sides.
    let b = burrau().ic;
    check_units(&b, "Burrau");
    assert!(b.p.iter().flatten().all(|&x| x == 0.0));
    let side = |i: usize, j: usize| norm(sub(b.r[i], b.r[j]));
    let (s01, s02, s12) = (side(0, 1), side(0, 2), side(1, 2));
    assert!(
        (s01 / s12 - 0.6).abs() < 1e-12 && (s02 / s12 - 0.8).abs() < 1e-12,
        "{s01} {s02} {s12}"
    );
    for (got, want) in b.m.iter().zip([5.0 / 12.0, 4.0 / 12.0, 3.0 / 12.0]) {
        assert!((got - want).abs() < 1e-15);
    }
    // Lagrange: equilateral, rigid rotation, ω² = M/a³.
    for m in [[1.0 / 3.0; 3], [0.5, 0.3, 0.2], [0.98, 0.01, 0.01]] {
        let l = lagrange("L", m, 30.0);
        check_units(&l.ic, "Lagrange");
        let r = l.ic.r;
        let a = norm(sub(r[0], r[1]));
        assert!(
            (norm(sub(r[0], r[2])) - a).abs() < 1e-12 && (norm(sub(r[1], r[2])) - a).abs() < 1e-12
        );
        let omega = (1.0 / (a * a * a)).sqrt();
        for i in 0..3 {
            let v = [l.ic.p[i][0] / m[i], l.ic.p[i][1] / m[i]];
            let rot = [-omega * r[i][1], omega * r[i][0]];
            let anti = [omega * r[i][1], -omega * r[i][0]];
            assert!(
                norm(sub(v, rot)) < 1e-12 || norm(sub(v, anti)) < 1e-12,
                "body {i} of {m:?} is not in rigid rotation at ω = {omega}"
            );
        }
        let want = lagrange_exponent(m) * omega;
        assert!((l.known.expect("known") - want).abs() < 1e-12);
    }
    assert_eq!(
        (Precision::F32.word(), Precision::F64.word()),
        ("f32", "f64")
    );
}

negative_control!(
    orbit_fixtures,
    "a fixture outside the sim's units must fail",
    expected = "I = ",
    check_units(
        &Ic {
            m: [1.0 / 3.0; 3],
            r: [[1.0, 0.0], [-1.0, 0.0], [0.0, 0.0]],
            p: [[0.0; 2]; 3]
        },
        "a unit-I check"
    )
);
