//! QA tests for TASK-M3-78, written from REQ-VAL-045, REQ-VAL-357, REQ-VAL-170/171's Heggie and Aarseth–Zare arms and
//! REQ-INT-105's Heggie arm, and from the integrator contract's Part 2b and Part 6, not from the implementation:
//!
//! - the radial collision case is the gate's (dd_validation_orbits § 2): equal masses from rest, the third body far
//!   away, the pair's free-fall collision inside the run;
//! - the passage, under each occupant, ends at the analytic two-body radial Kepler separation at `t = 2`, a ground
//!   truth from outside the system (the pair has passed through collision and is separating), and goes below
//!   `d_min < 1e-10` with the regularised drift below `1e-12`;
//! - the gate's reader check fails a Cartesian reader, its region count is the ladder's and is flagged against a
//!   declared minimum it falls short of (REQ-VAL-357), and its "does not fall with η" control is read over a *halving*
//!   ladder only;
//! - the Hamiltonian the finite-difference check differentiates is the contract's: Heggie's `Γ* = (H − h)·R₀R₁R₂` from
//!   the contract's energy (Eq. 20), `Γ** = Γ*·S^{−3/2}` (Eq. 22), Aarseth–Zare's `Γ = A·B·(H − E)`; `dt/dτ` is the
//!   contract's; a sign flip of *any* rate, and a small scale error on `dt/dτ`, fails the check at its tolerance;
//! - Heggie's floor counts every step on which it acts (Part 6): a `TINY` below every pre-floor value floors none, one
//!   above the least `dt/dτ` floors some; the proposal covers both time transformations, both widths and the three
//!   fixtures, with the reference's values beside it.
//!
//! Each test registers its negative control (R-176).

use std::path::{Path, PathBuf};

use kernel::occupant::rk4::Rk4;
use kernel::occupant::State;
use kernel::regularisation::az as kaz;
use kernel::regularisation::heggie as hg;
use kernel::regularisation::pairs::HeggieTime;
use serde_json::json;
use validation::az::{conditions, energy, radial, Landing, Quiet, SeparationFloorGate};
use validation::decode_f32_eps::SplitMix;
use validation::gate::{self, Gate, GateSpec, MeasurementGate, Minimum, RegionCount, Verdict};
use validation::heggie::{
    az_at, az_gamma, discrepancy_at, draw, falls, heggie_at, heggie_hamiltonian, hg_conditions,
    hg_floor_run, passage, propose_floor_heggie, uniform, Draw, FdOccupant, Occupant,
    RadialCollisionGate, D_MIN_BELOW, RADIAL_NAME,
};
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

// ── the radial collision case (dd_validation_orbits § 2; REQ-VAL-045) ─────────────────────────────────────────────

/// Two-body radial (e = 1) Kepler separation at time `t` from rest at separation `d0`, total mass `mu` (G = 1): from
/// apocentre, `r = a(1 − cos E)`, `t = √(a³/μ)(E − sin E − π)`, `a = d0/2`, continued through the collision
/// (`E > 2π`), where the regularised orbit bounces.
fn kepler_radial(d0: f64, mu: f64, t: f64) -> f64 {
    let a = d0 / 2.0;
    let target = std::f64::consts::PI + t / (a.powi(3) / mu).sqrt();
    let mut e = target;
    for _ in 0..200 {
        let f = e - e.sin() - target;
        let fp = 1.0 - e.cos();
        if fp.abs() < 1e-300 {
            e += 1e-3;
            continue;
        }
        e -= f / fp;
    }
    a * (1.0 - e.cos())
}

/// The free-fall collision time of the pair from rest at `d0`: `π/2·√(d0³/(2μ))`.
fn t_collision(d0: f64, mu: f64) -> f64 {
    std::f64::consts::FRAC_PI_2 * (d0.powi(3) / (2.0 * mu)).sqrt()
}

/// The case the gate's values name: two equal masses (the colliding pair, the closest two) from rest, the third far
/// away (at least ten pair separations from the pair's centre), the free-fall collision inside the run.
fn check_case(s: &State<f64>, t_end: f64) {
    let d = |i: usize, j: usize| (s.r[i][0] - s.r[j][0]).hypot(s.r[i][1] - s.r[j][1]);
    let pairs = [(0, 1, 2), (0, 2, 1), (1, 2, 0)];
    let (i, j, k) = pairs
        .into_iter()
        .min_by(|a, b| d(a.0, a.1).total_cmp(&d(b.0, b.1)))
        .expect("three pairs");
    assert_eq!(s.m[i], s.m[j], "the colliding pair's masses are not equal");
    for b in 0..3 {
        assert!(
            s.p[b] == [0.0, 0.0],
            "body {b} is not at rest: {:?}",
            s.p[b]
        );
    }
    let centre = [(s.r[i][0] + s.r[j][0]) / 2.0, (s.r[i][1] + s.r[j][1]) / 2.0];
    let far = (s.r[k][0] - centre[0]).hypot(s.r[k][1] - centre[1]);
    assert!(
        far >= 10.0 * d(i, j),
        "the third body is not far away: {far} against the pair's {}",
        d(i, j)
    );
    let tc = t_collision(d(i, j), s.m[i] + s.m[j]);
    assert!(
        tc < t_end,
        "the pair does not collide inside the run: {tc} against {t_end}"
    );
}

#[test]
fn qa_radial_case_is_the_gates() {
    let f = radial();
    let t_end = f.delta_sync * f64::from(f.boundaries);
    check_case(&f.state, t_end);
}

negative_control!(
    qa_radial_case_is_the_gates,
    "the pair given momentum: no longer from rest",
    expected = "is not at rest",
    {
        let f = radial();
        let mut s = f.state;
        s.p[0] = [0.1, 0.0];
        s.p[1] = [-0.1, 0.0];
        check_case(&s, f.delta_sync * f64::from(f.boundaries));
    }
);

/// The pair's separation after a march to `t_end` against the analytic radial Kepler separation. The third body at
/// distance `D = 20` perturbs the pair's relative motion by its tide, `≈ m₃·d/D³` against the pair's `μ/d²`, a
/// relative `≈ 6e-5` at `d ≈ 1`; the tolerance `1e-3` is that bound times about 15. A march that stalls before the
/// collision, stops at it or loses the bounce does not land on the analytic separation.
fn check_kepler(name: &str, s: &State<f64>, t: f64) {
    let f = radial();
    let s0 = f.state;
    let d0 = (s0.r[0][0] - s0.r[1][0]).hypot(s0.r[0][1] - s0.r[1][1]);
    let expect = kepler_radial(d0, s0.m[0] + s0.m[1], t);
    let got = (s.r[0][0] - s.r[1][0]).hypot(s.r[0][1] - s.r[1][1]);
    assert!(
        ((got - expect) / expect).abs() < 1e-3,
        "{name}: the pair's separation at t = {t} is {got}, the radial Kepler orbit's {expect}"
    );
}

/// Both occupants' marches of the passage to `t = 2` at `η = 0.01`: the march ended normally at `t = 2`, past the
/// analytic collision time, on the analytic separation; the gate's passage goes below `1e-10` and the regularised drift
/// below `1e-12` (REQ-VAL-045's values; R-257, R-566).
fn check_marches(kepler: impl Fn(&str, &State<f64>, f64)) {
    let f = radial();
    let t_end = f.delta_sync * f64::from(f.boundaries);
    assert_eq!(t_end, 2.0);
    let hp = hg_conditions::<f64>(0.01, f.delta_sync, hg::DEFAULT_TIME);
    let h = hg::march::<Rk4, f64, Quiet, kaz::ExplicitTest>(
        &f.state,
        f.boundaries,
        &hp,
        &mut Quiet,
        &mut kaz::ExplicitTest,
    );
    assert_eq!(h.count.end, kaz::End::Marching, "Heggie: {:?}", h.count.end);
    assert_eq!(h.t, t_end, "Heggie: the march ended at {}", h.t);
    kepler("Heggie", &h.state, h.t);
    let ap = conditions::<f64>(0.01, f.delta_sync, Landing::Fix);
    let a = kaz::march::<Rk4, f64, Quiet, kaz::ExplicitTest>(
        &f.state,
        0,
        f.boundaries,
        kaz::AzCount::new(),
        &ap,
        &mut Quiet,
        &mut kaz::ExplicitTest,
    );
    assert_eq!(
        a.count.end,
        kaz::End::Marching,
        "Aarseth–Zare: {:?}",
        a.count.end
    );
    assert_eq!(a.t, t_end, "Aarseth–Zare: the march ended at {}", a.t);
    kepler("Aarseth–Zare", &a.state, a.t);
    assert!(
        t_collision(1.0, 2.0 / 3.0) < t_end,
        "the collision is not inside the run"
    );
    // The Cartesian energy of the end state, the diagnostic read-out after the passage, is bounded too (R-566: read
    // only from a run that did not stop at an event, as these did not).
    for (name, s) in [("Heggie", &h.state), ("Aarseth–Zare", &a.state)] {
        let rel = ((energy(s) - energy(&f.state)) / energy(&f.state)).abs();
        assert!(
            rel < 1e-9,
            "{name}: the Cartesian energy after the passage drifted {rel:e}"
        );
    }
    for occupant in [Occupant::Heggie, Occupant::AarsethZare] {
        let p = passage(occupant, 0.01);
        assert!(p.d_min < 1e-10, "{occupant:?}: d_min {:e}", p.d_min);
        assert!(
            p.regularised < 1e-12,
            "{occupant:?}: drift {:e}",
            p.regularised
        );
        assert!(p.d_min >= 0.0 && p.d_min.is_finite());
        assert_eq!(p.t, t_end, "{occupant:?}: the passage ended at {}", p.t);
    }
    assert_eq!(D_MIN_BELOW, 1e-10, "dd_validation_orbits § 2's d_min");
}

#[test]
fn qa_radial_passage_meets_kepler() {
    check_marches(check_kepler);
}

negative_control!(
    qa_radial_passage_meets_kepler,
    "the analytic separation read at the run's start, as if the march had not moved",
    expected = "the radial Kepler orbit's",
    check_marches(|name, s, _t| check_kepler(name, s, 0.0))
);

// ── the gate's verdicts, regions and ladder (REQ-VAL-045, REQ-VAL-357) ─────────────────────────────────────────────

fn input(occupant: &str, reader: &str, ladder: &[f64]) -> serde_json::Value {
    json!({ "description": "qa", "occupant": occupant, "reader": reader, "eta": 0.01, "ladder": ladder })
}

const LADDER: [f64; 4] = [0.01, 0.005, 0.0025, 0.00125];

/// The gate's threshold is REQ-VAL-045's `1e-12`, its region minimum REQ-VAL-357's; under each occupant the
/// regularised reader passes and the Cartesian reader fails; the region count is the ladder's rungs, and a ladder
/// shorter than a declared minimum of 4 (REQ-VAL-357's proposal) is flagged.
fn check_gate(reader: &str) {
    let c = gate::config(&root(), RADIAL_NAME).expect("gate.json");
    assert_eq!(c.threshold.value, 1e-12, "REQ-VAL-045's |dE/E| < 1e-12");
    assert_eq!(c.threshold.requirement, "REQ-VAL-045");
    assert!(
        matches!(&c.min_regions, Minimum::NotCalibrated { requirement } if requirement == "REQ-VAL-357")
            || matches!(&c.min_regions, Minimum::Declared { requirement, .. } if requirement == "REQ-VAL-357"),
        "the region minimum is not REQ-VAL-357's: {:?}",
        c.min_regions
    );
    for occupant in ["heggie", "aarseth-zare"] {
        let r = RadialCollisionGate
            .run(&c, "qa", &input(occupant, reader, &LADDER))
            .expect("a report");
        assert_eq!(
            r.verdict,
            Verdict::Pass,
            "{occupant}: {:?}",
            r.negative_results
        );
        let r = RadialCollisionGate
            .run(&c, "qa", &input(occupant, "cartesian", &LADDER))
            .expect("a report");
        assert_eq!(
            r.verdict,
            Verdict::Fail,
            "{occupant}: the Cartesian reader passed"
        );
    }
    let mut declared = c.clone();
    declared.min_regions = Minimum::Declared {
        value: 4,
        requirement: "REQ-VAL-357".to_owned(),
    };
    let r = RadialCollisionGate
        .run(&declared, "qa", &input("heggie", reader, &LADDER[..3]))
        .expect("a report");
    let regions = r.regions.expect("a region section");
    assert_eq!(
        regions.count,
        RegionCount::Recorded(3),
        "the count is not the ladder's"
    );
    assert!(
        regions
            .flags()
            .iter()
            .any(|f| f.contains("fewer than the 4")),
        "a verdict from 3 regions against a minimum of 4 is not flagged: {:?}",
        regions.flags()
    );
    let r = RadialCollisionGate
        .run(&declared, "qa", &input("heggie", reader, &LADDER))
        .expect("a report");
    let regions = r.regions.expect("a region section");
    assert_eq!(regions.count, RegionCount::Recorded(4));
    assert!(regions.flags().is_empty(), "{:?}", regions.flags());
}

#[test]
fn qa_radial_gate_verdicts_and_regions() {
    check_gate("regularised");
}

negative_control!(
    qa_radial_gate_verdicts_and_regions,
    "the gate's reader pointed at the Cartesian read-out",
    expected = "Pass",
    check_gate("cartesian")
);

/// The checked-in ladder is a halving ladder of four rungs, coarsest first, the passage's `η` its first (REQ-VAL-357's
/// proposal of 4); and the "does not fall with η" control is read over a halving ladder only: a ladder that does not
/// halve rung by rung (a constant one, on which no read-out can fall, or one finest first) is refused or fails, never
/// passes.
fn check_ladder(ladders: &[&[f64]]) {
    for name in [
        "heggie.json",
        "aarseth_zare.json",
        "cartesian_reader.json",
        "aarseth_zare_cartesian_reader.json",
    ] {
        let text =
            std::fs::read_to_string(root().join("fixtures/gates/radial-collision").join(name))
                .expect("the fixture");
        let v: serde_json::Value = serde_json::from_str(&text).expect("json");
        let ladder: Vec<f64> = v["ladder"]
            .as_array()
            .expect("a ladder")
            .iter()
            .map(|x| x.as_f64().expect("a number"))
            .collect();
        assert_eq!(ladder.len(), 4, "{name}: REQ-VAL-357's 4 rungs");
        assert_eq!(v["eta"].as_f64(), Some(ladder[0]), "{name}");
        for w in ladder.windows(2) {
            assert_eq!(
                w[1],
                w[0] / 2.0,
                "{name}: the ladder does not halve: {ladder:?}"
            );
        }
    }
    let c = gate::config(&root(), RADIAL_NAME).expect("gate.json");
    for ladder in ladders {
        let verdict = RadialCollisionGate
            .run(&c, "qa", &input("heggie", "regularised", ladder))
            .map(|r| r.verdict);
        assert!(
            verdict != Ok(Verdict::Pass),
            "a ladder that does not halve, {ladder:?}, passes the \"does not fall with η\" control"
        );
    }
}

#[test]
fn qa_radial_ladder_halves() {
    check_ladder(&[&[0.01, 0.01, 0.01, 0.01], &[0.00125, 0.0025, 0.005, 0.01]]);
}

negative_control!(
    qa_radial_ladder_halves,
    "the checked-in halving ladder offered as one that does not halve",
    expected = "passes the \"does not fall with η\" control",
    check_ladder(&[&LADDER])
);

/// `falls`, the control's criterion, against read-outs of a known order in a halving `η`: a first-order fall
/// (`∝ η`) and a fourth-order fall read as falling; a flat read-out and one rising with refinement do not.
fn check_falls(falls: impl Fn(&[f64]) -> bool) {
    let at = |p: i32| -> Vec<f64> { LADDER.iter().map(|e: &f64| 1e-10 * e.powi(p)).collect() };
    assert!(falls(&at(1)), "a first-order fall does not read as falling");
    assert!(
        falls(&at(4)),
        "a fourth-order fall does not read as falling"
    );
    assert!(!falls(&at(0)), "a flat read-out reads as falling");
    assert!(
        !falls(&at(-1)),
        "a read-out rising with refinement reads as falling"
    );
}

#[test]
fn qa_radial_falls_criterion() {
    check_falls(falls);
}

negative_control!(
    qa_radial_falls_criterion,
    "a criterion that never reads a fall",
    expected = "a first-order fall does not read as falling",
    check_falls(|_x: &[f64]| false)
);

// ── hamiltonian-fd (REQ-VAL-170, REQ-VAL-171) ─────────────────────────────────────────────────────────────────────

/// `L(u)w` (integrator contract Part 2b).
fn l(u: [f64; 2], w: [f64; 2]) -> [f64; 2] {
    [u[0] * w[0] - u[1] * w[1], u[1] * w[0] + u[0] * w[1]]
}

fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

/// Heggie's `Γ* = (H − h)·R₀R₁R₂` with `H` the contract's energy of the pair variables `qᵢ = L(Qᵢ)Qᵢ`,
/// `pᵢ = L(Qᵢ)Pᵢ/(2Rᵢ)`: `E = Σᵢ [|pᵢ|²/(2μᵢ) − pⱼ·pₖ/mᵢ − mⱼmₖ/|qᵢ|]`. `coupling` is the coupling term's sign.
fn heggie_gamma_from_energy(x: &[f64], m: [f64; 3], coupling: f64) -> f64 {
    let qq = |i: usize| [x[2 * i], x[2 * i + 1]];
    let pp = |i: usize| [x[6 + 2 * i], x[7 + 2 * i]];
    let h = x[12];
    let r: Vec<f64> = (0..3).map(|i| dot(qq(i), qq(i))).collect();
    let p: Vec<[f64; 2]> = (0..3)
        .map(|i| {
            let w = l(qq(i), pp(i));
            [w[0] / (2.0 * r[i]), w[1] / (2.0 * r[i])]
        })
        .collect();
    let q: Vec<[f64; 2]> = (0..3).map(|i| l(qq(i), qq(i))).collect();
    let mut e = 0.0;
    for i in 0..3 {
        let (j, k) = ((i + 1) % 3, (i + 2) % 3);
        let mu = m[j] * m[k] / (m[j] + m[k]);
        let qi = dot(q[i], q[i]).sqrt();
        e += dot(p[i], p[i]) / (2.0 * mu) - coupling * dot(p[j], p[k]) / m[i] - m[j] * m[k] / qi;
    }
    (e - h) * r[0] * r[1] * r[2]
}

/// Aarseth–Zare's `Γ = A·B·(H − E)`, `H` the contract's physical energy of `R₁ = L(u₁)u₁`, `R₂ = L(u₂)u₂`,
/// `P₁ = L₁p₁/(2A)`, `P₂ = L₂p₂/(2B)`, `R₃ = R₂ − R₁`.
fn az_gamma_from_energy(x: &[f64], m: [f64; 3]) -> f64 {
    let (u1, u2, p1, p2, e) = ([x[0], x[1]], [x[2], x[3]], [x[4], x[5]], [x[6], x[7]], x[8]);
    let [ma, mb, mc] = m;
    let (a, b) = (dot(u1, u1), dot(u2, u2));
    let (r1, r2) = (l(u1, u1), l(u2, u2));
    let w1 = l(u1, p1);
    let w2 = l(u2, p2);
    let pp1 = [w1[0] / (2.0 * a), w1[1] / (2.0 * a)];
    let pp2 = [w2[0] / (2.0 * b), w2[1] / (2.0 * b)];
    let (mu1, mu2) = (ma * mb / (ma + mb), ma * mc / (ma + mc));
    let r3 = (r2[0] - r1[0]).hypot(r2[1] - r1[1]);
    let h = dot(pp1, pp1) / (2.0 * mu1) + dot(pp1, pp2) / ma + dot(pp2, pp2) / (2.0 * mu2)
        - ma * mb / a
        - ma * mc / b
        - mb * mc / r3;
    a * b * (h - e)
}

/// At random states, the Hamiltonians the check differentiates are the contract's, to round-off: Heggie's `Γ*`
/// (Eq. 20) equals `(H − h)·R₀R₁R₂` from the contract's energy, `Γ**` (Eq. 22) is `Γ*·S^{−3/2}`, Aarseth–Zare's `Γ` is
/// `A·B·(H − E)`; and the analytic `dt/dτ` is the contract's, `R₀R₁R₂` under Eq. 20 and `R₀R₁R₂/S^{3/2}` under Eq. 22.
fn check_hamiltonians(coupling: f64) {
    let mut rng = SplitMix::new(0x5161_7878);
    for n in 0..64 {
        let m = [
            uniform(&mut rng, 0.3, 3.0),
            uniform(&mut rng, 0.3, 3.0),
            uniform(&mut rng, 0.3, 3.0),
        ];
        let mut x: Vec<f64> = (0..6).map(|_| uniform(&mut rng, -1.2, 1.2)).collect();
        x.extend((0..6).map(|_| uniform(&mut rng, -1.0, 1.0)));
        x.push(uniform(&mut rng, -3.0, 1.0));
        let r: Vec<f64> = (0..3)
            .map(|i| x[2 * i].powi(2) + x[2 * i + 1].powi(2))
            .collect();
        let s: f64 = r.iter().sum();
        let ours = heggie_gamma_from_energy(&x, m, coupling);
        let hg20 = heggie_at(&x, m, HeggieTime::Eq20);
        let theirs = heggie_hamiltonian(&hg20.system, &hg20.q, &hg20.p);
        let scale = ours.abs().max(theirs.abs()).max(1.0);
        assert!(
            (ours - theirs).abs() <= 1e-12 * scale,
            "state {n}: Heggie's Γ* {theirs} is not (H − h)·R₀R₁R₂ {ours}"
        );
        let hg22 = heggie_at(&x, m, HeggieTime::Eq22);
        let g22 = heggie_hamiltonian(&hg22.system, &hg22.q, &hg22.p);
        let want22 = ours / s.powf(1.5);
        assert!(
            (g22 - want22).abs() <= 1e-12 * want22.abs().max(1.0),
            "state {n}: Heggie's Γ** {g22} is not Γ*·S^(−3/2) {want22}"
        );
        // dt/dτ from the kernel's rates, as the gate's draws read them.
        for (time, want) in [
            (FdOccupant::HeggieEq20, r[0] * r[1] * r[2]),
            (FdOccupant::HeggieEq22, r[0] * r[1] * r[2] / s.powf(1.5)),
        ] {
            let rates = heggie_dt(&x, m, time);
            assert!(
                (rates - want).abs() <= 1e-13 * want.abs(),
                "state {n}: {time:?}'s dt/dτ {rates} is not {want}"
            );
        }
        let mut xa: Vec<f64>;
        loop {
            xa = (0..4).map(|_| uniform(&mut rng, -1.2, 1.2)).collect();
            let (r1, r2) = (
                l([xa[0], xa[1]], [xa[0], xa[1]]),
                l([xa[2], xa[3]], [xa[2], xa[3]]),
            );
            if (r2[0] - r1[0]).hypot(r2[1] - r1[1]) >= 0.1 {
                break;
            }
        }
        xa.extend((0..4).map(|_| uniform(&mut rng, -1.0, 1.0)));
        xa.push(uniform(&mut rng, -3.0, 1.0));
        let at = az_at(&xa, m);
        let theirs = az_gamma(&at.system, &at.q, &at.p);
        let ours = az_gamma_from_energy(&xa, m);
        assert!(
            (ours - theirs).abs() <= 1e-11 * ours.abs().max(theirs.abs()).max(1.0),
            "state {n}: Aarseth–Zare's Γ {theirs} is not A·B·(H − E) {ours}"
        );
    }
}

/// `dt/dτ` from the kernel's rates at the flat state `x` with masses `m`, as the gate reads them.
fn heggie_dt(x: &[f64], m: [f64; 3], occupant: FdOccupant) -> f64 {
    use kernel::regularisation::form::Flow;
    let time = match occupant {
        FdOccupant::HeggieEq20 => HeggieTime::Eq20,
        _ => HeggieTime::Eq22,
    };
    let at = heggie_at(x, m, time);
    let (dq, _) = at.system.rates(&at.q, &at.p);
    dq.t
}

#[test]
fn qa_fd_hamiltonians_are_the_contracts() {
    check_hamiltonians(1.0);
}

negative_control!(
    qa_fd_hamiltonians_are_the_contracts,
    "Heggie's energy with its coupling term's sign flipped",
    expected = "is not (H − h)·R₀R₁R₂",
    check_hamiltonians(-1.0)
);

/// A draw whose analytic rates are `d`'s with `edit` applied to the flat vector `(∂Γ/∂p…, −∂Γ/∂q…, dt/dτ)`.
fn edited(d: Draw, edit: impl Fn(&mut [f64]) + 'static) -> Draw {
    let Draw {
        x,
        hamiltonian,
        rates,
    } = d;
    Draw {
        x,
        hamiltonian,
        rates: Box::new(move |y| {
            let (dq, dp, dt) = rates(y);
            let n = dq.len();
            let mut all: Vec<f64> = dq.into_iter().chain(dp).chain([dt]).collect();
            edit(&mut all);
            (all[..n].to_vec(), all[n..2 * n].to_vec(), all[2 * n])
        }),
    }
}

/// At the gate's step and tolerance (`gate.json`, REQ-VAL-171's proposal, the fixtures' step), each occupant's
/// unedited rates pass over 16 random states; a sign flip of any one rate, position, momentum or `dt/dτ`, fails at
/// least one of them; and so does `dt/dτ` scaled by `1 + 1e-6`, a wrong time transformation's error two orders above
/// the tolerance.
fn check_fd(edit_baseline: bool) {
    let c = gate::config(&root(), "hamiltonian-fd").expect("gate.json");
    let tol = c.threshold.value;
    assert_eq!(c.threshold.requirement, "REQ-VAL-171");
    let text =
        std::fs::read_to_string(root().join("fixtures/gates/hamiltonian-fd/heggie_eq22.json"))
            .expect("the fixture");
    let v: serde_json::Value = serde_json::from_str(&text).expect("json");
    let step = v["step"].as_f64().expect("a step");
    for occupant in FdOccupant::ALL {
        let n = match occupant {
            FdOccupant::AarsethZare => 9,
            _ => 13,
        };
        let base = worst_of(occupant, step, None, edit_baseline);
        assert!(
            base <= tol,
            "{occupant:?}: the unedited rates fail: {base:e} > {tol:e}"
        );
        for c in 0..n {
            let d = worst_of(occupant, step, Some(Edit::Flip(c)), false);
            assert!(
                d > tol,
                "{occupant:?}: component {c}'s flipped sign passes: {d:e} <= {tol:e}"
            );
        }
        let d = worst_of(occupant, step, Some(Edit::ScaleDt(1.0 + 1e-6)), false);
        assert!(
            d > tol,
            "{occupant:?}: dt/dτ scaled by 1 + 1e-6 passes: {d:e} <= {tol:e}"
        );
    }
}

#[derive(Clone, Copy)]
enum Edit {
    Flip(usize),
    ScaleDt(f64),
}

/// The largest discrepancy over 16 draws of `occupant` at `step`, the rates edited by `edit`; with `flip_first`, the
/// first momentum rate's sign flipped as well (the control's baseline).
fn worst_of(occupant: FdOccupant, step: f64, edit: Option<Edit>, flip_first: bool) -> f64 {
    let mut rng = SplitMix::new(0x7161);
    (0..16)
        .map(|_| {
            let d = draw(occupant, &mut rng);
            let d = match edit {
                None => d,
                Some(Edit::Flip(c)) => edited(d, move |a: &mut [f64]| a[c] = -a[c]),
                Some(Edit::ScaleDt(k)) => edited(d, move |a: &mut [f64]| {
                    let last = a.len() - 1;
                    a[last] *= k;
                }),
            };
            discrepancy_at(&d, step, flip_first)
        })
        .fold(0.0, f64::max)
}

#[test]
fn qa_fd_every_rate_can_fail() {
    check_fd(false);
}

negative_control!(
    qa_fd_every_rate_can_fail,
    "the baseline with the first momentum rate's sign flipped",
    expected = "the unedited rates fail",
    check_fd(true)
);

// ── separation-floor's Heggie arm (REQ-INT-105; integrator contract Part 6) ───────────────────────────────────────

/// On the radial collision passage at f64, under Eq. 20 and Eq. 22, with `factor` scaling the `TINY` below every
/// pre-floor value: a `TINY` half the least of the pre-floor `dt/dτ`, `S` and `Rᵢ` floors no step (the floor acts only
/// where one is below it); one twice the least `dt/dτ` floors at least one step and no more than were taken. Through the
/// gate, the Heggie control above binds and the reference's does not, and another occupant is refused.
fn check_floor(factor: f64) {
    let f = radial();
    for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
        let base = hg_floor_run::<f64>(&f, time, None);
        assert_eq!(
            base.floored, 0,
            "{time:?}: the reference's TINY floors a step on the passage"
        );
        let dt = base.least.dt_dtau.expect("a step");
        let s = base.least.s.expect("a step");
        let r = base.least.r.expect("a step");
        assert!(dt > 0.0 && s > 0.0 && r > 0.0, "{time:?}: {:?}", base.least);
        let below = factor * dt.min(s).min(r);
        let run = hg_floor_run::<f64>(&f, time, Some(below));
        assert_eq!(
            run.floored, 0,
            "{time:?}: a TINY {below:e} below every pre-floor value floors {} step(s)",
            run.floored
        );
        let above = hg_floor_run::<f64>(&f, time, Some(2.0 * dt));
        assert!(
            above.floored >= 1 && above.floored <= above.steps,
            "{time:?}: a TINY twice the least dt/dτ floors {} of {} steps",
            above.floored,
            above.steps
        );
    }
    let spec = gate::spec(&root(), "separation-floor").expect("gate.json");
    let GateSpec::Measurement(config) = spec else {
        panic!("separation-floor is a measurement");
    };
    let control = |tiny: &str, occupant: &str| {
        SeparationFloorGate.control(
            &config,
            "qa",
            &json!({ "description": "qa", "tiny": tiny, "occupant": occupant }),
        )
    };
    let up = control("above", "heggie").expect("the control");
    assert_eq!(up.verdict, Verdict::Pass, "{}", up.markdown);
    let reference = control("reference", "heggie").expect("the control");
    assert_eq!(reference.verdict, Verdict::Fail, "{}", reference.markdown);
    assert!(
        control("above", "logh-or-other").is_err(),
        "an unknown occupant is not refused"
    );
}

#[test]
fn qa_floor_counts_every_floored_step() {
    check_floor(0.5);
}

negative_control!(
    qa_floor_counts_every_floored_step,
    "a TINY ten times the least pre-floor value offered as one below every value",
    expected = "below every pre-floor value floors",
    check_floor(10.0)
);

/// The proposal's Heggie arm covers each fixture (the radial collision passage and the close-encounter fixtures) under
/// both time transformations at both widths, each row with its floored count, and reports the reference's values beside
/// it (REQ-INT-105's verify).
fn check_proposal(proposal: &str) {
    let rows: Vec<&str> = proposal
        .lines()
        .filter(|l| l.starts_with("| ") && (l.contains("| Eq. 20 |") || l.contains("| Eq. 22 |")))
        .collect();
    for fixture in [
        radial().name,
        validation::az::hierarchy().name,
        validation::az::burrau().name,
    ] {
        for time in ["Eq. 20", "Eq. 22"] {
            for width in ["f32", "f64"] {
                let want = format!("| {fixture} | {time} | {width} |");
                assert!(
                    rows.iter().any(|r| r.starts_with(&want)),
                    "the proposal has no row {want}:\n{proposal}"
                );
            }
        }
    }
    assert!(
        proposal.contains("1e-37") && proposal.contains("1e-300"),
        "the reference's values are not reported:\n{proposal}"
    );
}

#[test]
fn qa_floor_proposal_covers_heggie() {
    check_proposal(&propose_floor_heggie());
}

negative_control!(
    qa_floor_proposal_covers_heggie,
    "the proposal with its Eq. 20 rows dropped",
    expected = "the proposal has no row",
    {
        let p = propose_floor_heggie();
        let kept: Vec<&str> = p.lines().filter(|l| !l.contains("| Eq. 20 |")).collect();
        check_proposal(&kept.join("\n"));
    }
);
