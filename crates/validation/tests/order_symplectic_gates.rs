//! `order-scaling` and `symplectic-drift` (`validation::gate::{order_scaling, symplectic}`; dd_integrator §5 tests 1
//! and 2; REQ-VAL-030, REQ-VAL-031, their tolerances REQ-VAL-139's), each check with its control:
//! - `order_scaling_slopes`: every occupant's fitted slope is within the gate's threshold of its profile's order; its
//!   control is RK4 mislabelled as order 3.
//! - `order_scaling_range`: a ladder with a rung outside the asymptotic regime is refused whole, at the coarse bound
//!   and at the rounding floor; its control is the floor taken to 0, where the below-floor ladder is not refused.
//! - `kepler_period`: the fixture's closed-form period is the orbit's: Yoshida-6 returns to the start after one; its
//!   control is the period 0.1% long.
//! - `trend_statistic`: the statistic reads an oscillation over the figure-eight's 13.5 periods as bounded, within the
//!   first-principles bound, and drift of the oscillation's size as secular; its control judges a ramp bounded.
//! - `symplectic_claims`: KDK, Yoshida-4 and Yoshida-6 read bounded, RK4 and Euler secular, and Euler lights
//!   `SUSPECT_ENERGY`; each run resolvable above rounding, Yoshida-4, Yoshida-6 and RK4 at a coarser `dt_macro`
//!   since at the default key they are below it; its control is RK4 claimed bounded.
//! - `recorder_is_passive`: the recording cadence leaves the run bitwise the drift view's; its control drops one
//!   forwarded call.

use std::path::{Path, PathBuf};

use validation::drift::{figure_eight, integrate_fixture};
use validation::gate::order_scaling::{
    judge, kepler_embedded, Input as OrderInput, Regime, REGIME,
};
use validation::gate::symplectic::{
    measure, record, rounding_bound, trend, Claim, Measured, Trend,
};
use validation::gate::{self};
use validation::negative_control;

use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::{Geom, Occupant, Selection};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A gate's threshold, from its `gate.json`.
fn threshold(name: &str) -> f64 {
    gate::config(&root(), name)
        .expect("gate.json is accepted")
        .threshold
        .value
}

/// An `order-scaling` input from its fixture file.
fn order_input(file: &str) -> OrderInput {
    let path = root().join("fixtures/gates/order-scaling").join(file);
    let mut v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("fixture")).expect("json");
    v.as_object_mut().expect("object").remove("expected");
    serde_json::from_value(v).expect("an order-scaling input")
}

// ── order-scaling ──────────────────────────────────────────────────────────────────────────────────────────────

/// The criterion, written before the run: the fitted slope within the gate's threshold of the order judged.
fn check_slope(input: &OrderInput, tol: f64) {
    let j = judge(input, &REGIME).unwrap_or_else(|e| panic!("{}: refused: {e}", input.occupant));
    let off = (j.fit.slope - f64::from(j.order)).abs();
    let local: Vec<String> = j.fit.local.iter().map(|s| format!("{s:.3}")).collect();
    for r in &j.rungs {
        println!(
            "{}: n = {}, dt/τ_p = {:.3e}, error {:.4e}",
            input.occupant, r.n, r.ratio, r.error
        );
    }
    println!(
        "{}: slope {:.4} ± {:.4}, local [{}], order {}",
        input.occupant,
        j.fit.slope,
        j.fit.std_err,
        local.join(", "),
        j.order
    );
    assert!(
        off <= tol,
        "{}: the fitted slope {:.4} is off the order {} by {off:.4}, above {tol}",
        input.occupant,
        j.fit.slope,
        j.order
    );
}

#[test]
fn order_scaling_slopes() {
    let tol = threshold("order-scaling");
    for file in [
        "euler.json",
        "kdk.json",
        "yoshida4.json",
        "rk4.json",
        "yoshida6.json",
    ] {
        check_slope(&order_input(file), tol);
    }
}

negative_control!(
    order_scaling_slopes,
    "RK4 mislabelled as order 3 must be off its claimed order",
    expected = "is off the order",
    check_slope(
        &order_input("rk4_claimed_3.json"),
        threshold("order-scaling")
    )
);

/// The ladder is refused whole under `regime`, naming the regime.
fn check_refused(input: &OrderInput, regime: &Regime) {
    match judge(input, regime) {
        Err(e) => {
            println!("{}: refused: {e}", input.occupant);
            assert!(
                e.contains("outside the asymptotic regime"),
                "{}: refused for another reason: {e}",
                input.occupant
            );
        }
        Ok(j) => panic!(
            "{}: the ladder was not refused: slope {:.4} over {} rungs",
            input.occupant,
            j.fit.slope,
            j.rungs.len()
        ),
    }
}

#[test]
fn order_scaling_range() {
    check_refused(&order_input("kdk_too_coarse.json"), &REGIME);
    check_refused(&order_input("yoshida6_below_floor.json"), &REGIME);
}

negative_control!(
    order_scaling_range,
    "with the rounding floor taken to 0 the below-floor ladder is not refused",
    expected = "the ladder was not refused",
    check_refused(
        &order_input("yoshida6_below_floor.json"),
        &Regime {
            floor: 0.0,
            ..REGIME
        }
    )
);

/// Yoshida-6 at 800 steps over `period` returns to the start: the largest position change over the largest position.
fn check_period(period: f64) {
    let k = kepler_embedded();
    let occupant = Selection::Production(Occupant::Yoshida6);
    let n = 800;
    let (mut s, mut g) = (k.state, Geom::new(&k.state.m, &k.state.r));
    for _ in 0..n {
        (s, g) = occupant.step(&s, period / n as f64, &g);
    }
    let scale = (0..2)
        .flat_map(|i| k.state.r[i])
        .fold(0.0f64, |a, x| a.max(x.abs()));
    let moved = (0..2)
        .flat_map(|i| (0..2).map(move |c| (i, c)))
        .fold(0.0f64, |a, (i, c)| {
            a.max((s.r[i][c] - k.state.r[i][c]).abs())
        });
    println!(
        "one period {period:.6}: the pair moved {:.3e} of {scale:.3e}",
        moved
    );
    assert!(
        moved / scale < 1e-6,
        "after the period {period} the pair is {:.3e} from its start, relative",
        moved / scale
    );
}

#[test]
fn kepler_period() {
    let k = kepler_embedded();
    println!("period {:.6}, τ_p {:.6}", k.period, k.tau_peri);
    check_period(k.period);
}

negative_control!(
    kepler_period,
    "a period 0.1% long does not return the pair to its start",
    expected = "from its start, relative",
    check_period(kepler_embedded().period * 1.001)
);

// ── symplectic-drift ───────────────────────────────────────────────────────────────────────────────────────────

/// `t` judged against `claim` at `tol`, the gate's reading.
fn check_claim(what: &str, t: &Trend, claim: Claim, tol: f64) {
    let secular = t.ratio.is_nan() || t.ratio > tol;
    println!(
        "{what}: trend {:.4e}, oscillation {:.4e}, ρ = {:.4} ({}), claimed {claim:?}",
        t.trend,
        t.oscillation,
        t.ratio,
        if secular { "secular" } else { "bounded" }
    );
    assert_eq!(
        secular,
        claim == Claim::Secular,
        "{what}: ρ = {:.4} against {tol} is not the claimed {claim:?}",
        t.ratio
    );
}

/// The figure-eight's horizon in its periods: `T = 200` over the period 14.8 (the gate's note).
const PERIODS: f64 = 200.0 / 14.8;

/// The first-principles bound on `ρ` for an oscillation at the orbit's period over [`PERIODS`] periods.
fn oscillation_bound() -> f64 {
    let wt = std::f64::consts::TAU * PERIODS;
    12.0 * (1.0 / wt + 2.0 / (wt * wt))
}

/// Synthetic series on the sim key's grid: `δ(t) = c + A sin(ωt + φ) + s·t`.
fn synthetic(phase: f64, drift: f64) -> Vec<(f64, f64)> {
    let n = 65535;
    let dt = 200.0 / n as f64;
    let w = std::f64::consts::TAU * PERIODS / 200.0;
    (1..=n)
        .map(|k| {
            let t = k as f64 * dt;
            (t, 3e-6 + 1e-6 * (w * t + phase).sin() + drift * t)
        })
        .collect()
}

#[test]
fn trend_statistic() {
    let tol = threshold("symplectic-drift");
    let bound = oscillation_bound();
    println!("the first-principles bound on ρ over {PERIODS:.2} periods: {bound:.4}");
    assert!(bound < tol, "the bound {bound} is not below {tol}");
    for k in 0..16 {
        let phase = std::f64::consts::TAU * k as f64 / 16.0;
        let t = trend(&synthetic(phase, 0.0));
        assert!(
            t.ratio <= bound * 1.01,
            "phase {phase}: ρ {} above the bound {bound}",
            t.ratio
        );
        check_claim("oscillation", &t, Claim::Bounded, tol);
        // Drift of the oscillation's size over the run: `s·T = A`.
        check_claim(
            "oscillation with drift",
            &trend(&synthetic(phase, 1e-6 / 200.0)),
            Claim::Secular,
            tol,
        );
    }
}

negative_control!(
    trend_statistic,
    "a ramp judged bounded must fail",
    expected = "is not the claimed Bounded",
    check_claim(
        "ramp",
        &trend(&synthetic(0.0, 1e-5)),
        Claim::Bounded,
        threshold("symplectic-drift")
    )
);

/// One occupant's run judged: resolvable above rounding, its claim, and `SUSPECT_ENERGY` when claimed.
fn check_run(name: &str, m: &Measured, claim: Claim, suspect: Option<bool>, tol: f64) {
    let steps = m.run.series.len();
    let floor = rounding_bound(steps);
    println!(
        "{name}: {steps} steps, largest |δ| {:.4e} (rounding bound {floor:.3e}), SUSPECT_ENERGY {} ({:.4e})",
        m.trend.peak, m.suspect, m.suspect_ratio
    );
    for q in validation::gate::symplectic::quarters(&m.run.series) {
        println!("{name}: {q}");
    }
    assert!(
        m.trend.peak >= floor,
        "{name}: the largest |δ| {:.3e} is below the rounding bound {floor:.3e}",
        m.trend.peak
    );
    check_claim(name, &m.trend, claim, tol);
    if let Some(lit) = suspect {
        assert_eq!(m.suspect, lit, "{name}: SUSPECT_ENERGY is not as claimed");
    }
}

#[test]
fn symplectic_claims() {
    let tol = threshold("symplectic-drift");
    let orbit = figure_eight();
    // Yoshida-4, Yoshida-6 and RK4 at the default key sit below the rounding bound (the gate refuses them there), so
    // they are judged with only `dt_macro` changed, ×4, ×8 and ×4, the gate's `_coarse` inputs.
    for (name, scale, claim, suspect) in [
        ("kdk", 1.0, Claim::Bounded, Some(false)),
        ("yoshida4", 4.0, Claim::Bounded, Some(false)),
        ("yoshida6", 8.0, Claim::Bounded, Some(false)),
        ("rk4", 4.0, Claim::Secular, Some(false)),
        ("euler", 1.0, Claim::Secular, Some(true)),
    ] {
        let m = measure(name, &orbit, scale).expect("an occupant");
        assert_eq!(m.profile.symplectic, claim == Claim::Bounded);
        check_run(&format!("{name} ×{scale}"), &m, claim, suspect, tol);
    }
    for name in ["yoshida4", "yoshida6", "rk4"] {
        let m = measure(name, &orbit, 1.0).expect("an occupant");
        let floor = rounding_bound(m.run.series.len());
        println!(
            "{name} at the default key: largest |δ| {:.4e}, rounding bound {floor:.3e}, ρ = {:.4}",
            m.trend.peak, m.trend.ratio
        );
        assert!(
            m.trend.peak < floor,
            "{name} at the default key is above the rounding bound: its fixture should judge it, not refuse it"
        );
    }
}

negative_control!(
    symplectic_claims,
    "RK4 at the ×4 key claimed bounded, as if it were symplectic, must fail",
    expected = "is not the claimed Bounded",
    check_run(
        "rk4 ×4",
        &measure("rk4", &figure_eight(), 4.0).expect("rk4"),
        Claim::Bounded,
        None,
        threshold("symplectic-drift")
    )
);

/// The recorded run's stored state equals the drift view's run, bitwise.
fn check_passive(forward_clock: bool) {
    let orbit = figure_eight();
    let kdk = record::<Kdk>(&orbit, 1.0, forward_clock).out;
    let euler = record::<Euler>(&orbit, 1.0, forward_clock).out;
    assert!(
        kdk == integrate_fixture::<Kdk, f64>(&orbit).0
            && euler == integrate_fixture::<Euler, f64>(&orbit).0,
        "the recorded run is not the drift view's"
    );
}

#[test]
fn recorder_is_passive() {
    check_passive(true);
}

negative_control!(
    recorder_is_passive,
    "a recorder that does not pass the clock on changes the stored run",
    expected = "is not the drift view's",
    check_passive(false)
);
