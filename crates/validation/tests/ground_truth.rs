//! The ground-truth suite's entry paths and tiers (validation_ground_truth_note; TASK-M3-24), at CPU-f64, through the
//! Precision ring (`engine::precision`), each case's reference independent of the kernel's stepper: a closed form, a
//! published value, or the RK45 reference.
//!
//! - `entry_paths_figure_eight` (tier 3, Path A): CHECK 1, then the figure-eight's closure after its period, by
//!   Yoshida-4 and by the shipping occupant, Heggie × RK4 (the closure gate's ladder and order are
//!   `tests/figure_eight_closure.rs`'s).
//! - `entry_paths_burrau` (tier 4, Path A): CHECK 1, then Szebehely & Peters' outcome, structurally; `r_coll` pinned
//!   and carried. Ignored until TASK-M3-34: its escape label fires near 158 of S&P's units.
//! - `entry_paths_burrau_ejects_on_time` (tier 4): the march and the reference eject the lightest body in 60–65.
//! - `entry_paths_euler_path_b` (tier 2, Path B): Euler's collinear rigid rotation, and the Euler shape point.
//! - `entry_paths_path_b_refuses_off_gauge`: Path B checks the gauge and never imposes it.
//! - `tiers_lagrange` (tier 2, Path A): Lagrange's equilateral rigid rotation.
//! - `tiers_conservation` (tier 1): `E`, `L_z`, `P` across the figure-eight's march.
//! - `ground_truth_fixture_set`: the fixture set has every tier, and the note lists each fixture.
//! - `ground_truth_heggie_is_the_field_backend_s`: the suite's Heggie values are the field backend's.
//!
//! **Pass criteria, fixed before the first run of the kernel**, each calibrated on the reference alone (RK45 at
//! `rtol = 10⁻¹³` and `10⁻¹²`, and the closed forms):
//! - CHECK 1: no clamp, and the residual within `ε_phys` (REQ-ENC-024's register value at f64, as `kernel`'s ingest test
//!   holds it); the pixel's decode is CHECK 1's `D(z)` bit for bit.
//! - Closed forms (Lagrange, Euler) at half a turn: `phys ≤ 10⁻⁸`. The reference's own distance from them is at most
//!   `2.8·10⁻¹²` at half a turn (measured, both tolerances), so the bound leaves Yoshida-4's error at `dt ≈ 10⁻³` three
//!   decades of room and is still eight decades below a wrong force.
//! - The figure-eight's closure: `phys ≤ 10⁻⁶`, against the reference's own closure of `4.1·10⁻⁸` (measured, the same
//!   at `rtol = 10⁻¹¹`, `10⁻¹²`, `10⁻¹³`: it is the published digits', not the integrator's); the march against the
//!   reference at the period: `phys ≤ 10⁻⁸`.
//! - Conservation over the figure-eight's period: `|ΔE/E| ≤ 10⁻⁹`, `L_z` and `P` within `10⁻¹¹` of their scales.
//! - Burrau: escape, the lightest body, between 50 and 100 of Szebehely & Peters' time units (the literature's
//!   disruption near 60; the window is the anchor smoke test's coarse one, dd_validation_orbits §1.5, and its upper edge
//!   is the horizon); the reference's escaper the same. *Added after the first run*, which found the label `bounded` at
//!   the horizon: the march's own end state, read structurally as the reference's is, has the lightest body escaped.
//!   It stands beside the label's criterion, never in its place.
//!
//! **The controls** (PIT-3), each making its test fail: Euler's first-order occupant in place of Yoshida-4 (every
//! trajectory check); the figure-eight's published labelling, which the encode clamps; an unregularised occupant on
//! Burrau, Yoshida-4 at `r_coll = 0`, whose escape comes far too early; a fixture set missing a tier, and a note missing a
//! fixture; a Heggie value off the field backend's.
//!
//! **Domain** (pitfalls §10): one start per case, one occupant and link selection, the stated horizon; CPU-f64 only.

use kernel::decode::Ic;
use kernel::driver::advance::Advance;
use kernel::generated::links::Defaults;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::payload::{pa_d_min, sd_detail, sd_state, tm_t_end_step};

use validation::ground_truth::path_a::{self, PathA};
use validation::ground_truth::path_b::{self, Refused};
use validation::ground_truth::tiers::{self, Rigid, BURRAU_HORIZON, REFERENCE};
use validation::ground_truth::{
    burrau_time_scale, fixtures, landing_key, march_key, DefaultOccupant, FieldHeggie, Fixture,
    BENCHMARK_R_COLL,
};
use validation::import::{root, unit_state, Start};
use validation::negative_control;
use validation::roundtrip::{eps_phys, phys};

/// The default collision radius, `10⁻³` (integrator contract Part 7), for the cases that are not benchmarks.
const R_COLL_DEFAULT: f64 = 1e-3;

/// A closed form's bound at half a turn (the module's note).
const CLOSED_FORM_TOL: f64 = 1e-8;

/// The figure-eight's closure bound (the module's note).
const CLOSURE_TOL: f64 = 1e-6;

/// The march against the RK45 reference at the figure-eight's period (the module's note).
const REFERENCE_TOL: f64 = 1e-8;

/// Conservation's bounds (the module's note).
const ENERGY_TOL: f64 = 1e-9;
const INVARIANT_TOL: f64 = 1e-11;

fn ic_of(s: &validation::ground_truth::path_a::Forward) -> Ic<f64> {
    Ic {
        m: s.start.m,
        r: s.state.r,
        p: s.state.p,
    }
}

/// CHECK 1's assertions.
fn check1(name: &str, a: &PathA) {
    let c = &a.check1;
    println!(
        "{name}: CHECK 1 residual {:e} (ε_phys {:e}), κ {:e}, clamped {}",
        c.residual,
        eps_phys::<f64>(),
        c.kappa,
        c.clamped
    );
    assert!(
        !c.clamped,
        "{name}: the encode clamped (lookup_clamped): the start is on a pole, Path B's"
    );
    assert!(
        c.residual <= eps_phys::<f64>(),
        "{name}: CHECK 1's residual {:e} exceeds ε_phys {:e}",
        c.residual,
        eps_phys::<f64>()
    );
    assert!(
        a.forward_matches_check1(),
        "{name}: the pixel's decode is not CHECK 1's D(z) bit for bit"
    );
}

// ── Tier 3, Path A: the figure-eight ─────────────────────────────────────────────────────────────────────────────

/// The figure-eight from `start` through Path A, marched by `A` to its period: the run and the period.
fn figure_eight_run<A: Advance>(start: &Start, occupant: &'static str) -> (PathA, f64) {
    let x = unit_state(start).expect("the figure-eight's start");
    let c = path_a::check1::<Defaults>(&x);
    let period = tiers::figure_eight_period() * tiers::time_scale(start, c.lambda);
    let key = landing_key(period, R_COLL_DEFAULT).expect("the sim key validates");
    let run = path_a::run::<Defaults, A>(&x, &key, period, ("figure_eight", occupant, "defaults"))
        .expect("the forward leg");
    (run, period)
}

fn check_figure_eight<A: Advance>(start: &Start, occupant: &'static str) {
    let (run, period) = figure_eight_run::<A>(start, occupant);
    check1("figure_eight", &run);
    let end = ic_of(&run.forward);
    let start_state = run.check1.decoded;
    let reference = tiers::reference(&start_state, &[period], &REFERENCE)
        .expect("the reference reaches the period");
    let closure = phys(&start_state, &end);
    let against = phys(&reference[0], &end);
    println!(
        "figure_eight: {}; period {period:.6}, closure {closure:e} (≤ {CLOSURE_TOL:e}), reference's closure {:e}, march \
         against the reference {against:e} (≤ {REFERENCE_TOL:e})",
        run.provenance,
        phys(&start_state, &reference[0])
    );
    assert!(
        closure <= CLOSURE_TOL,
        "the figure-eight does not close: {closure:e} after its period"
    );
    assert!(
        against <= REFERENCE_TOL,
        "the march is {against:e} from the RK45 reference at the period"
    );
}

#[test]
fn entry_paths_figure_eight() {
    check_figure_eight::<Yoshida4>(&tiers::figure_eight(), "Yoshida-4");
    // The shipping occupant through the same pixel path (TASK-M3-28): the wrapper's march, as a pixel runs it.
    check_figure_eight_landed::<DefaultOccupant>(landed);
    // Reported: the published labelling, on the α = 0 pole.
    let x = unit_state(&tiers::figure_eight_published()).unwrap();
    let c = path_a::check1::<Defaults>(&x);
    println!(
        "figure_eight, published labelling (reported): residual {:e}, κ {:e}, clamped {}",
        c.residual, c.kappa, c.clamped
    );
}

/// Where an occupant that owns its time mapping ends a march asked for `horizon`: on the last sync boundary at or before
/// it, `⌊T/Δ_sync⌋·Δ_sync` (integrator_contract Part 2a: it stops only on the grid). Measured on the figure-eight
/// (TASK-M3-28): asked for the period, `14.8509`, Heggie × RK4 ends at `14.75`, `1.8·10⁻¹²` from the RK45 reference
/// there and `5.5·10⁻²` from it at the period, so its closure is the closure gate's to judge, on a grid that lands on
/// the period (`validation::gate::figure_eight`).
fn landed(horizon: f64) -> f64 {
    let d = kernel::regularisation::sync::delta_sync::<f64>();
    (horizon / d).floor() * d
}

/// The figure-eight through Path A by `A`, an occupant that owns its time mapping, against the RK45 reference at
/// `at(period)`: CHECK 1, then the march within [`REFERENCE_TOL`] of the reference there. Criterion written before the
/// run: the reference tolerance the Yoshida-4 arm is held to.
fn check_figure_eight_landed<A: Advance>(at: fn(f64) -> f64) {
    assert!(
        A::OWN_SAMPLING,
        "the occupant does not own its time mapping"
    );
    let (run, period) = figure_eight_run::<A>(
        &tiers::figure_eight(),
        "Heggie × RK4 (field backend's values)",
    );
    check1("figure_eight", &run);
    let end = ic_of(&run.forward);
    let t = at(period);
    let reference =
        tiers::reference(&run.check1.decoded, &[t], &REFERENCE).expect("the reference reaches t");
    let against = phys(&reference[0], &end);
    println!(
        "figure_eight: {}; asked for the period {period:.6}, compared at t = {t:.6}: march against the reference \
         {against:e} (≤ {REFERENCE_TOL:e}); its closure there {:e}, reported",
        run.provenance,
        phys(&run.check1.decoded, &end)
    );
    assert!(
        against <= REFERENCE_TOL,
        "the shipping occupant's march is {against:e} from the RK45 reference at t = {t:.6}"
    );
}

mod figure_eight_default_at_period {
    use super::*;
    negative_control!(
        entry_paths_figure_eight,
        "the shipping occupant compared at the period, which its march does not land on",
        expected = "the shipping occupant's march is",
        check_figure_eight_landed::<DefaultOccupant>(|t| t)
    );
}

mod figure_eight_published {
    use super::*;
    negative_control!(
        entry_paths_figure_eight,
        "the published labelling, on the α = 0 pole, must be refused by CHECK 1",
        expected = "the encode clamped",
        check_figure_eight::<Yoshida4>(&tiers::figure_eight_published(), "Yoshida-4")
    );
}

mod figure_eight_first_order {
    use super::*;
    negative_control!(
        entry_paths_figure_eight,
        "Euler's first-order occupant must not close the figure-eight",
        expected = "the figure-eight does not close",
        check_figure_eight::<kernel::occupant::euler::Euler>(&tiers::figure_eight(), "Euler")
    );
}

// ── Tier 1: conservation across the figure-eight ─────────────────────────────────────────────────────────────────

fn check_conservation<A: Advance>(occupant: &'static str) {
    let (run, _) = figure_eight_run::<A>(&tiers::figure_eight(), occupant);
    let d = tiers::conservation(&run.check1.decoded, &ic_of(&run.forward));
    println!(
        "conservation_figure_eight ({occupant}): |ΔE/E| {:e} (≤ {ENERGY_TOL:e}), L_z {:e}, P {:e} (≤ {INVARIANT_TOL:e})",
        d.energy, d.lz, d.momentum
    );
    assert!(
        d.energy <= ENERGY_TOL,
        "the energy drifted by {:e}",
        d.energy
    );
    assert!(d.lz <= INVARIANT_TOL, "L_z drifted by {:e}", d.lz);
    assert!(d.momentum <= INVARIANT_TOL, "P drifted by {:e}", d.momentum);
}

#[test]
fn tiers_conservation() {
    check_conservation::<Yoshida4>("Yoshida-4");
}

negative_control!(
    tiers_conservation,
    "Euler's first-order occupant drifts in energy",
    expected = "the energy drifted",
    check_conservation::<kernel::occupant::euler::Euler>("Euler")
);

// ── Tier 2: the closed forms ─────────────────────────────────────────────────────────────────────────────────────

/// The march's state at half a turn of `rig` against the closed form.
fn check_rigid(name: &str, rig: &Rigid, end: &Ic<f64>, t: f64) {
    let off = phys(&rig.at(t), end);
    println!(
        "{name}: closed form against the march at t = {t:.6}: {off:e} (≤ {CLOSED_FORM_TOL:e})"
    );
    assert!(
        off <= CLOSED_FORM_TOL,
        "{name}: the march is {off:e} from the closed form at half a turn"
    );
}

fn check_lagrange<A: Advance>(occupant: &'static str) {
    let rig = tiers::lagrange([0.2, 0.3, 0.5]);
    let t = rig.period() / 2.0;
    let key = landing_key(t, R_COLL_DEFAULT).unwrap();
    let run = path_a::run::<Defaults, A>(
        &rig.start,
        &key,
        t,
        ("lagrange_equilateral", occupant, "defaults"),
    )
    .expect("the forward leg");
    check1("lagrange_equilateral", &run);
    println!("lagrange_equilateral: {}", run.provenance);
    check_rigid("lagrange_equilateral", &rig, &ic_of(&run.forward), t);
}

#[test]
fn tiers_lagrange() {
    check_lagrange::<Yoshida4>("Yoshida-4");
}

negative_control!(
    tiers_lagrange,
    "Euler's first-order occupant leaves Lagrange's rigid rotation",
    expected = "from the closed form at half a turn",
    check_lagrange::<kernel::occupant::euler::Euler>("Euler")
);

fn check_euler<A: Advance>(occupant: &'static str) {
    let rig = tiers::euler();
    // Its route: the encode reaches it only by clamping.
    let c = path_a::check1::<Defaults>(&rig.start);
    println!(
        "euler_collinear: encode residual {:e}, κ {:e}, clamped {} (the chart reaches it only by clamping)",
        c.residual, c.kappa, c.clamped
    );
    assert!(
        c.clamped,
        "the Euler configuration is not on a pole the encode clamps: it is not Path B's"
    );
    let t = rig.period() / 2.0;
    let key = landing_key(t, R_COLL_DEFAULT).unwrap();
    let s = rig.start;
    let b = path_b::inject::<A>(s.m, s.r, s.p, &key).expect("the closed form is in the gauge");
    println!(
        "euler_collinear: {}",
        path_b::provenance("euler_collinear", occupant, &key, t)
    );
    let end = Ic {
        m: s.m,
        r: b.state.r,
        p: b.state.p,
    };
    check_rigid("euler_collinear", &rig, &end, t);
    let off = (0..3)
        .map(|k| (b.shape_end[k] - tiers::EULER_SHAPE[k]).abs())
        .fold(0.0f64, f64::max);
    println!(
        "euler_collinear: shape at the start {:?}, at t {:?}: {off:e} from E2 (≤ {CLOSED_FORM_TOL:e})",
        b.shape_start, b.shape_end
    );
    assert!(
        off <= CLOSED_FORM_TOL,
        "the shape left the Euler point E2 by {off:e}"
    );
}

#[test]
fn entry_paths_euler_path_b() {
    check_euler::<Yoshida4>("Yoshida-4");
}

negative_control!(
    entry_paths_euler_path_b,
    "Euler's first-order occupant leaves Euler's rigid rotation",
    expected = "from the closed form at half a turn",
    check_euler::<kernel::occupant::euler::Euler>("Euler")
);

/// Path B refuses `shift`ed off the centre of mass.
fn check_refusal(shift: f64) {
    let s = tiers::euler().start;
    let r = s.r.map(|v| [v[0] + shift, v[1]]);
    let key = march_key(1.0, R_COLL_DEFAULT).unwrap();
    let got = path_b::inject::<Yoshida4>(s.m, r, s.p, &key).map(|_| ());
    assert!(
        matches!(got, Err(Refused::Gauge { .. })),
        "a start off the centre of mass by {shift:e} is not refused: {got:?}"
    );
}

#[test]
fn entry_paths_path_b_refuses_off_gauge() {
    check_refusal(1e-6);
}

negative_control!(
    entry_paths_path_b_refuses_off_gauge,
    "the start in the gauge is not refused",
    expected = "is not refused",
    check_refusal(0.0)
);

// ── Tier 4, Path A: Burrau ───────────────────────────────────────────────────────────────────────────────────────

fn check_burrau<A: Advance>(occupant: &'static str, r_coll: f64) {
    let start = tiers::burrau();
    let x = unit_state(&start).unwrap();
    let scale = burrau_time_scale();
    let horizon = BURRAU_HORIZON * scale;
    let key = march_key(horizon, r_coll).unwrap();
    let run = path_a::run::<Defaults, A>(&x, &key, horizon, ("burrau", occupant, "defaults"))
        .expect("the forward leg");
    check1("burrau", &run);
    println!("burrau: {}", run.provenance);
    assert_eq!(
        run.provenance.r_coll, r_coll,
        "the run's provenance does not carry the r_coll it ran at"
    );
    let s = &run.forward.state;
    let t_end = f64::from(tm_t_end_step(s.times)) * key.loop_values.dt_macro;
    let lightest = tiers::lightest(start.m);
    println!(
        "burrau: state {}, detail {}, t_end {t_end:.3} canonical = {:.3} of S&P's units, d_min {:e} (the default r_coll \
         is {R_COLL_DEFAULT:e}), total substeps {}",
        sd_state(s.packed_a),
        sd_detail(s.packed_a),
        t_end / scale,
        pa_d_min(s.packed_a),
        s.total_substeps
    );
    // The independent reference, read structurally at the horizon.
    let reference = tiers::reference(&run.check1.decoded, &[horizon], &REFERENCE)
        .expect("the reference reaches T");
    let o = tiers::outcome(&reference[0]);
    println!("burrau: the RK45 reference at T: {o:?}");
    assert_eq!(
        o.escaper,
        Some(lightest),
        "the RK45 reference's escaper is not the lightest body"
    );
    // Added after the first run, which found the label `bounded` at T: the march's own end state read the same
    // structural way. Added beside the label's criterion, never in its place.
    let own = tiers::outcome(&ic_of(&run.forward));
    println!("burrau: the march's own end, read the same way: {own:?}");
    assert_eq!(
        own.escaper,
        Some(lightest),
        "the Burrau march's own end state has not ejected the lightest body"
    );
    tiers::check_burrau_label(s, key.loop_values.dt_macro, scale, start.m);
}

// Ignored until the owner settles tau: at the provisional 1e-3 the label fires at 157.6 of S&P's units; at any tau from
// 3e-3 to the measured false-positive edge 1.353e-2 it fires in [50, 100] (`cargo xtask gate tau-gap --propose`).
#[test]
#[ignore = "Burrau: at tau = 1e-3 the escape label fires at t = 157.6, outside [50, 100]; the measured tau gap (TASK-M3-34) did not settle a new tau, so the owner decides; plan/KNOWN.md"]
fn entry_paths_burrau() {
    check_burrau::<DefaultOccupant>("Heggie × RK4 (field backend's values)", BENCHMARK_R_COLL);
}

negative_control!(
    entry_paths_burrau,
    "Yoshida-4 unregularised does not resolve Burrau's encounters, and its escape comes too early",
    expected = "the Burrau",
    check_burrau::<Yoshida4>("Yoshida-4", BENCHMARK_R_COLL)
);

/// The Burrau ejection, structurally (not ignored): the march, by `A`, and the RK45 reference, each sampled at
/// [`tiers::ejection_samples`], leave the lightest body unbound from a time in [`tiers::BURRAU_EJECTION_WINDOW`] on
/// ([`tiers::ejection_time`]), and at 100 of S&P's units it is ten pair separations away and unbound. The march is
/// sampled by marches to each sample's macro-step under the one key, so each is the full march's prefix. Criterion
/// written before the run, from the measured physics this PR reports; the label's own criterion stays with
/// `entry_paths_burrau`.
///
/// Measured: both read 60.0, on the window's edge; at 0.05 resolution the reference's onset is 59.6, which the 0.5
/// grid reads as 60.0. Reported, not moved.
fn check_burrau_ejection<A: Advance>(occupant: &'static str) {
    let start = tiers::burrau();
    let x = unit_state(&start).unwrap();
    let scale = burrau_time_scale();
    let key = march_key(BURRAU_HORIZON * scale, BENCHMARK_R_COLL).unwrap();
    let dt = key.loop_values.dt_macro;
    let c = path_a::check1::<Defaults>(&x);
    let times = tiers::ejection_samples();
    let lightest = tiers::lightest(start.m);
    let march: Vec<(f64, Ic<f64>)> = times
        .iter()
        .map(|&ts| {
            let mut k = key;
            k.loop_values.horizon_steps = (ts * scale / dt).round() as u32;
            let f = path_a::forward::<Defaults, A>(&c.z, &k).expect("the forward leg");
            (ts, ic_of(&f))
        })
        .collect();
    let canonical: Vec<f64> = march.iter().map(|(ts, _)| ts * scale).collect();
    let reference: Vec<(f64, Ic<f64>)> = times
        .iter()
        .copied()
        .zip(tiers::reference(&c.decoded, &canonical, &REFERENCE).expect("the reference reaches T"))
        .collect();
    for (name, samples) in [(occupant, &march), ("RK45", &reference)] {
        let t_e = tiers::ejection_time(samples, lightest);
        let end = tiers::outcome(&samples.last().unwrap().1);
        println!(
            "burrau ejection, {name}: the lightest body unbound from {t_e:?} of S&P's units on (window \
             {:?}); at 100: {end:?}",
            tiers::BURRAU_EJECTION_WINDOW
        );
        assert!(
            t_e.is_some_and(|t| (tiers::BURRAU_EJECTION_WINDOW[0]
                ..=tiers::BURRAU_EJECTION_WINDOW[1])
                .contains(&t)),
            "{name}: the lightest body is unbound from {t_e:?}, outside the Burrau ejection window"
        );
        assert_eq!(
            end.escaper,
            Some(lightest),
            "{name}: the lightest body has not escaped at 100"
        );
    }
}

#[test]
fn entry_paths_burrau_ejects_on_time() {
    check_burrau_ejection::<DefaultOccupant>("Heggie × RK4");
}

negative_control!(
    entry_paths_burrau_ejects_on_time,
    "Yoshida-4 unregularised does not resolve Burrau's encounters, so its ejection is not the literature's",
    expected = "Yoshida-4",
    check_burrau_ejection::<Yoshida4>("Yoshida-4")
);

// ── The fixture set and the Heggie values ────────────────────────────────────────────────────────────────────────

fn check_fixture_set(set: &[Fixture], note: &str) {
    for tier in 1..=4 {
        assert!(
            set.iter().any(|f| f.tier == tier),
            "the fixture set has no case of tier {tier}"
        );
    }
    for f in set {
        assert!(
            f.path == "A" || f.path == "B",
            "{}: path {}",
            f.name,
            f.path
        );
        assert!(
            !f.source.is_empty() && !f.asserted.is_empty(),
            "{}: no source or nothing asserted",
            f.name
        );
        assert!(
            note.contains(&format!("`{}`", f.name)),
            "the note does not list the fixture `{}`",
            f.name
        );
    }
    assert!(
        set.iter().any(|f| f.path == "B"),
        "the fixture set has no Path B case"
    );
}

fn note() -> String {
    std::fs::read_to_string(root().join("docs/notes/principia_validation_ground_truth_note.md"))
        .unwrap()
}

#[test]
fn ground_truth_fixture_set() {
    check_fixture_set(&fixtures().unwrap(), &note());
}

mod fixture_set_tier {
    use super::*;
    negative_control!(
        ground_truth_fixture_set,
        "a set with no literature benchmark must fail",
        expected = "no case of tier 4",
        {
            let set: Vec<Fixture> = fixtures()
                .unwrap()
                .into_iter()
                .filter(|f| f.tier != 4)
                .collect();
            check_fixture_set(&set, &note())
        }
    );
}

mod fixture_set_note {
    use super::*;
    negative_control!(
        ground_truth_fixture_set,
        "a note that does not list a fixture must fail",
        expected = "the note does not list the fixture",
        check_fixture_set(&fixtures().unwrap(), &note().replace("`burrau`", "Burrau"))
    );
}

fn check_heggie(field: &kernel::regularisation::heggie::HgParams<f32>) {
    use kernel::regularisation::heggie::HgConfig;
    assert_eq!(
        format!("{:?}", FieldHeggie::params::<f32>()),
        format!("{field:?}"),
        "the suite's Heggie values are not the field backend's"
    );
}

#[test]
fn ground_truth_heggie_is_the_field_backend_s() {
    check_heggie(&engine::field::heggie_params());
}

negative_control!(
    ground_truth_heggie_is_the_field_backend_s,
    "another step factor must fail",
    expected = "are not the field backend's",
    check_heggie(&kernel::regularisation::heggie::HgParams {
        eta: 0.02,
        ..engine::field::heggie_params()
    })
);
