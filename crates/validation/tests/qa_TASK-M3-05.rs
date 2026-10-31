//! QA tests for TASK-M3-05, from the requirements, not the implementation:
//! - REQ-INT-110: the gate's report gives the classes it fed and their count against the minimum, and flags a verdict
//!   drawn from fewer (`qa_projection_parity_*`).
//! - REQ-INT-085, REQ-TOOL-150: each proposal's pass condition, recomputed here on the measurement, with the diverging
//!   ramp's colours taken from render contract Part 5 and the sRGB transfer written out (`qa_proposal_*`).
//! - dd_integrator §5 test 2: the fixture's orbit is the published figure-eight, by its period, not by its digits
//!   (`qa_figure_eight_returns_at_its_period`); test 7: the definition's "every step is one substep"
//!   (`qa_close_encounter_is_one_substep`).

use kernel::canon::canonicalise;
use kernel::decode::Ic;
use kernel::driver::wrapper::integrate;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{Geom, State, Stepper};
use kernel::payload::{suspect_energy, suspect_lz};
use ledger::constants::{DRIFT_LIN_R_E, DRIFT_LIN_R_LZ};
use std::path::PathBuf;
use validation::drift::{
    close_encounter, figure_eight, measure, sim_key, Drifts, Measured, FIGURE_EIGHT_V3,
    FIGURE_EIGHT_X1, T_DEFAULT,
};
use validation::gate::{self, Gate};
use validation::negative_control;
use validation::projection_parity::ProjectionParityGate;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn report_on(states: u32) -> gate::report::GateReport {
    let config = gate::config(&root(), "projection-parity").expect("the gate's config");
    let input = serde_json::json!({
        "description": "qa",
        "fuzz": { "seed": 9, "states": states },
        "dt": 0.001
    });
    ProjectionParityGate
        .run(&config, "qa.json", &input)
        .expect("the gate runs")
}

fn check_flagged(states: u32) {
    let report = report_on(states);
    let flags = report.flags().expect("a well-formed report");
    assert!(
        flags.iter().any(|f| f.contains("fewer than the 4")),
        "a verdict from {states} state(s) is not flagged: {flags:?}"
    );
}

#[test]
fn qa_projection_parity_flags_fewer_classes() {
    for states in 1..4 {
        check_flagged(states);
    }
    let flags = report_on(4).flags().expect("a well-formed report");
    assert!(flags.is_empty(), "four classes fed, yet flagged: {flags:?}");
}

negative_control!(
    qa_projection_parity_flags_fewer_classes,
    "four classes are the minimum, so nothing flags them",
    expected = "is not flagged",
    check_flagged(4)
);

fn check_prints(text: &str, count: u32) {
    for class in [
        "generic",
        "close pair",
        "high momentum",
        "far off the centre of mass",
    ] {
        assert!(
            text.contains(class),
            "the report doesn't name `{class}`: {text}"
        );
    }
    assert!(text.contains(&format!("region count: {count}")), "{text}");
    assert!(text.contains("region minimum: 4 (REQ-INT-110)"), "{text}");
}

#[test]
fn qa_projection_parity_prints_its_classes_against_the_minimum() {
    let text = report_on(8).markdown().expect("a well-formed report");
    check_prints(&text, 4);
    let few = report_on(2).markdown().expect("a well-formed report");
    assert!(few.contains("region count: 2"), "{few}");
}

negative_control!(
    qa_projection_parity_prints_its_classes_against_the_minimum,
    "a report with no classes",
    expected = "doesn't name",
    check_prints("region count: 4", 4)
);

// ── The proposals ───────────────────────────────────────────────────────────────────────────────────────────────

/// The sRGB decode, written out.
fn decode(c: u8) -> f64 {
    let c = f64::from(c) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// The sRGB encode to 8 bits, written out.
fn encode(l: f64) -> u8 {
    let c = if l <= 0.0031308 {
        12.92 * l
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    };
    (c * 255.0).round() as u8
}

/// Render contract Part 5's `diverging`, 8-bit sRGB ends and neutral, mixed in linear RGB, `c0` at `t = 0.5`.
const C_MINUS: [u8; 3] = [0x3B, 0x4C, 0xC0];
const C_0: [u8; 3] = [0xDD, 0xDD, 0xDD];
const C_PLUS: [u8; 3] = [0xB4, 0x04, 0x26];

fn lin_pixel(x: f64, r: f64) -> [u8; 3] {
    let t = ((x + r) / (2.0 * r)).clamp(0.0, 1.0);
    let (a, b, u) = if t < 0.5 {
        (C_MINUS, C_0, t / 0.5)
    } else {
        (C_0, C_PLUS, (t - 0.5) / 0.5)
    };
    [0, 1, 2].map(|k| encode(decode(a[k]) + (decode(b[k]) - decode(a[k])) * u))
}

fn on<'a>(table: &'a [Measured], name: &str) -> &'a Measured {
    table
        .iter()
        .find(|m| m.occupant == name)
        .expect("an occupant measured")
}

/// REQ-TOOL-150's condition: Euler's and RK4's final drifts on test 2, at each precision, are distinct 8-bit pixels,
/// neither at an end.
fn check_range(table: &[Measured], read: fn(&Drifts) -> f64, r: f64) {
    for (euler, rk4) in [
        (&on(table, "Euler").f32, &on(table, "RK4").f32),
        (&on(table, "Euler").f64, &on(table, "RK4").f64),
    ] {
        let (a, b) = (lin_pixel(read(euler), r), lin_pixel(read(rk4), r));
        assert_ne!(a, b, "Euler's and RK4's pixels at R = {r:e} are equal");
        for c in [a, b] {
            assert!(
                c != C_MINUS && c != C_PLUS,
                "a final drift at R = {r:e} saturates: {c:?}"
            );
        }
    }
}

fn final_e(d: &Drifts) -> f64 {
    d.de_final
}

fn final_l(d: &Drifts) -> f64 {
    d.dlz_final
}

#[test]
fn qa_proposal_lin_range_separates_euler_and_rk4() {
    let table = measure(&figure_eight());
    check_range(&table, final_e, DRIFT_LIN_R_E.number());
    check_range(&table, final_l, DRIFT_LIN_R_LZ.number());
}

negative_control!(
    qa_proposal_lin_range_separates_euler_and_rk4,
    "an R far under Euler's drift saturates it",
    expected = "saturates",
    check_range(
        &measure(&figure_eight()),
        final_e,
        DRIFT_LIN_R_E.number() / 1024.0
    )
);

fn lit(d: &Drifts) -> (bool, bool) {
    (
        suspect_energy(d.de_max_stored as f32, d.e_0_stored, 1e-6),
        suspect_lz(d.dlz_max_stored as f32),
    )
}

/// REQ-INT-085's condition, read as the predicates read it: the stored latches.
fn check_thresholds(test2: &[Measured], test7: &[Measured]) {
    for run in [&on(test2, "Euler").f32, &on(test2, "Euler").f64] {
        assert_eq!(
            lit(run),
            (true, true),
            "Euler on test 2 must light both: {run:?}"
        );
    }
    for name in ["KDK", "Yoshida-4", "Yoshida-6"] {
        for run in [&on(test2, name).f32, &on(test2, name).f64] {
            assert_eq!(lit(run), (false, false), "{name} on test 2 lit: {run:?}");
        }
        for run in [&on(test7, name).f32, &on(test7, name).f64] {
            assert!(!lit(run).1, "{name} on test 7 lights L_z: {run:?}");
        }
    }
}

#[test]
fn qa_proposal_thresholds_hold_the_criterion() {
    check_thresholds(&measure(&figure_eight()), &measure(&close_encounter()));
}

negative_control!(
    qa_proposal_thresholds_hold_the_criterion,
    "test 7 read as test 2: KDK's spike lights the energy predicate, which test 2's condition forbids",
    expected = "on test 2 lit",
    {
        let t7 = measure(&close_encounter());
        let mut as_test2 = measure(&figure_eight());
        as_test2.iter_mut().for_each(|m| {
            if let Some(from) = t7.iter().find(|n| n.occupant == m.occupant) {
                if m.occupant == "KDK" {
                    *m = *from;
                }
            }
        });
        check_thresholds(&as_test2, &t7);
    }
);

// ── The fixtures' definitions (dd_integrator §5) ────────────────────────────────────────────────────────────────

/// The figure-eight's period in `G = 1`, `mᵢ = 1`, the published value (Chenciner & Montgomery 2000; Simó).
const PERIOD: f64 = 6.325_913_98;

/// Marches `fixture`'s state for `t` by Yoshida-6 at `dt` and returns the largest coordinate change from the start.
fn moved(ic: &Ic<f64>, t: f64, dt: f64) -> f64 {
    let mut s = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let mut g = Geom::new(&s.m, &s.r);
    for _ in 0..(t / dt).round() as u64 {
        let (n, h) = Yoshida6::step(&s, dt, &g);
        (s, g) = (n, h);
    }
    (0..3)
        .flat_map(|i| (0..2).map(move |k| (i, k)))
        .map(|(i, k)| (s.r[i][k] - ic.r[i][k]).abs())
        .fold(0.0, f64::max)
}

/// The period in the fixture's units: `√3` from the masses `1/3` (`a ∝ m`), then `λ^{3/2}`, `λ` the canonicalisation's
/// length scale, at `G = M = 1`.
fn fixture_period() -> f64 {
    let third = 1.0 / 3.0;
    let [x, y] = FIGURE_EIGHT_X1;
    let [vx, vy] = FIGURE_EIGHT_V3;
    let s = third_sqrt();
    let v = [[-vx / 2.0, -vy / 2.0], [-vx / 2.0, -vy / 2.0], [vx, vy]];
    let raw = Ic {
        m: [third; 3],
        r: [[x, y], [-x, -y], [0.0, 0.0]],
        p: v.map(|[a, b]| [third * s * a, third * s * b]),
    };
    PERIOD * 3f64.sqrt() * canonicalise(&raw).lambda.powf(1.5)
}

fn third_sqrt() -> f64 {
    (1.0f64 / 3.0).sqrt()
}

/// The orbit comes back to its start at its period, and is elsewhere at half of it (the bodies exchanged).
fn check_return(t: f64) {
    let ic = figure_eight().ic;
    let halfway = moved(&ic, fixture_period() / 2.0, 1e-3);
    let back = moved(&ic, t, 1e-3);
    assert!(
        back <= 1e-3 * halfway,
        "the orbit at t = {t} is {back:e} from its start, halfway {halfway:e}"
    );
}

#[test]
fn qa_figure_eight_returns_at_its_period() {
    check_return(fixture_period());
}

negative_control!(
    qa_figure_eight_returns_at_its_period,
    "a tenth of a period off, the orbit isn't back",
    expected = "from its start",
    check_return(1.1 * fixture_period())
);

/// Test 7's definition: the pericentre stays above `r_sub`, "so every step is one substep".
fn check_one_substep(ic: &Ic<f64>) {
    let key = sim_key::<f64>(T_DEFAULT);
    let out = integrate::<Kdk, f64>(ic, &key);
    let steps = kernel::payload::tm_t_end_step(out.times);
    assert_eq!(steps, key.horizon_steps, "the run ended early");
    assert_eq!(
        out.total_substeps, key.horizon_steps,
        "dd test 7's definition: every step is one substep"
    );
}

#[test]
fn qa_close_encounter_is_one_substep() {
    let f = close_encounter();
    assert_eq!(f.horizon, T_DEFAULT);
    check_one_substep(&f.ic);
}

negative_control!(
    qa_close_encounter_is_one_substep,
    "e = 0.99 takes the pericentre under r_sub, so the law substeps",
    expected = "every step is one substep",
    check_one_substep(
        &validation::drift::hierarchy(
            validation::drift::HIERARCHY_MASSES,
            validation::drift::HIERARCHY_A,
            0.99,
            validation::drift::HIERARCHY_D,
        )
        .0
    )
);
