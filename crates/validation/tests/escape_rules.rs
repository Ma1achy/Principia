//! The escape rules measured against a continued-run truth (`validation::escape_checks::rules`; TASK-M3-34):
//! - `escape_rules_harness_is_production`: on `config stability` pixels and the Burrau anchor, every rule's commit in
//!   the harness is the production march's own under that rule: the same `detail` on the macro-step holding the
//!   harness's sampling point, or neither fires. Control: the harness's energy floor against production's closure rule.
//! - `escape_rules_truth`: prin-rs's continued-run truth on constructed records: three separations growing as `t` is a
//!   dispersal; a binary whose separation, read at two phases, differs by `(1 + e)/(1 − e)` and a third body receding as
//!   `t` is that body's escape; separations that do not grow are no escape. Control: prin-rs's first truth, which read
//!   the pair's own separation exponent (required under 0.3) and so called the eccentric binary's escape unresolved.
//!
//! - `escape_rules_default_is_the_kernels`: `RuleId::DEFAULT`'s production march is the default march
//!   (`engine::precision::integrate_ic`, the kernel's `DefaultRule`), bit for bit. Control: the energy floor named.
//!
//! - `escape_rules_gauge_r0_is_one`: the three fixtures' and the Burrau anchor's ICs have `R_0 = √(I_0/M) = 1`, the gauge's
//!   value the distance floor and the Chazy rule read. Control: the ICs scaled by 2.
//!
//! **Domain**: f64, Heggie × RK4 as the field backend marches it, for the harness; constructed records for the truth.

use kernel::occupant::State;

use validation::escape_checks::rules::{self, Record, RuleId, Truth};
use validation::escape_checks::{config_stability, GroundTruthOccupant};
use validation::negative_control;

fn check_harness(rule_of: fn(RuleId) -> RuleId) {
    let pop = config_stability(16, 50.0);
    // Pixel 104 fires the closure rule at t = 14.5; the others are its row's neighbours.
    for i in [100, 104, 108] {
        for rule in RuleId::ALL {
            rules::same_as_production::<GroundTruthOccupant>(
                &pop.ics[i],
                &pop.keys[i],
                rule,
                rule_of(rule),
                20_000,
            )
            .unwrap_or_else(|e| panic!("pixel {i}: the harness is not production: {e}"));
        }
    }
    let (ic, run, scale) = validation::escape_checks::revalidation::burrau();
    let _ = run;
    let key = validation::ground_truth::march_key(170.0 * scale, 0.0).expect("the Burrau key");
    let steps = (170.0 * scale / key.loop_values.dt_macro).round() as u32;
    for rule in RuleId::ALL {
        rules::same_as_production::<GroundTruthOccupant>(ic, &key, rule, rule_of(rule), steps)
            .unwrap_or_else(|e| panic!("Burrau: the harness is not production: {e}"));
    }
}

#[test]
fn escape_rules_harness_is_production() {
    check_harness(|r| r);
}

negative_control!(
    escape_rules_harness_is_production,
    "the harness's energy floor against production's closure rule",
    expected = "the harness is not production",
    check_harness(|r| if r == RuleId::EnergyFloor {
        RuleId::Closure
    } else {
        r
    })
);

/// A record every 2 time units from 2 to 1000, separations `d(t)` by the body each pair leaves out, the hierarchy ratio
/// `ratio(t)` of body 2.
fn records(d: impl Fn(f64) -> [f64; 3], ratio: impl Fn(f64) -> f64) -> Vec<Record> {
    (1..=500)
        .map(|k| {
            let t = 2.0 * f64::from(k);
            Record {
                t,
                d: d(t),
                ratio: ratio(t),
                body: 2,
            }
        })
        .collect()
}

/// A bound circular pair, bodies 0 and 1 of mass 0.4 at separation 0.2, and body 2 far away.
fn bound_last() -> State<f64> {
    State {
        m: [0.4, 0.4, 0.2],
        r: [[-0.1, 0.0], [0.1, 0.0], [500.0, 0.0]],
        p: [[0.0, -0.4], [0.0, 0.4], [0.2, 0.0]],
    }
}

type TruthFn = fn(&[Record], &[f64; 3], &State<f64>) -> Truth;

fn check_truth(truth: TruthFn) {
    let last = bound_last();
    let m = last.m;
    let dispersal = records(|t| [60.0 + t, 70.0 + t, 80.0 + t], |_| 1.0);
    assert_eq!(truth(&dispersal, &m, &last), Truth::Dispersal);
    // The eccentric pair (e = 0.9) read at pericentre then apocentre: its separation grows 19-fold between the second
    // half's ends; body 2 recedes from both members as t.
    let escape = records(|t| [t, t, if t < 600.0 { 0.01 } else { 0.19 }], |t| t / 0.1);
    assert_eq!(
        truth(&escape, &m, &last),
        Truth::Escape(2),
        "the eccentric binary's escaper is not the truth's"
    );
    let bound = records(|_| [1.0, 1.2, 0.8], |_| 1.5);
    assert_eq!(truth(&bound, &m, &last), Truth::NoEscape);
}

/// prin-rs's first truth: the escape needs the pair's own separation exponent under 0.3.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn first_truth(series: &[Record], m: &[f64; 3], last: &State<f64>) -> Truth {
    match rules::truth(series, m, last) {
        Truth::Escape(b) => {
            let end = series[series.len() - 1];
            let start = series.iter().find(|x| x.t >= 0.5 * end.t).expect("a start");
            let k = b as usize;
            let beta = (end.d[k] / start.d[k]).ln() / (end.t / start.t).ln();
            if beta < 0.3 {
                Truth::Escape(b)
            } else {
                Truth::NoEscape
            }
        }
        other => other,
    }
}

#[test]
fn escape_rules_truth() {
    check_truth(rules::truth);
}

negative_control!(
    escape_rules_truth,
    "prin-rs's first truth, the pair's own separation exponent",
    expected = "the eccentric binary's escaper is not the truth's",
    check_truth(first_truth)
);

fn check_default(rule: RuleId) {
    let pop = config_stability(16, 50.0);
    let ic = pop.ics[104];
    // Both modes: live, two rules can end on the same label at the horizon; under the stop option their first labels'
    // times differ (found when the energy floor's control stopped failing under the live default).
    for stop_on_escape in [false, true] {
        let key = engine::precision::MarchKey64 {
            stop_on_escape,
            ..pop.keys[104]
        };
        let own = engine::precision::integrate_ic::<GroundTruthOccupant>(&ic, None, &key);
        let named = rules::production::<GroundTruthOccupant>(&ic, &key, rule);
        assert_eq!(
            format!("{own:?}"),
            format!("{named:?}"),
            "RuleId::DEFAULT is not the kernel's DefaultRule (stop option {stop_on_escape})"
        );
    }
}

#[test]
fn escape_rules_default_is_the_kernels() {
    check_default(RuleId::DEFAULT);
}

negative_control!(
    escape_rules_default_is_the_kernels,
    "the energy floor named as the default",
    expected = "is not the kernel's DefaultRule",
    check_default(RuleId::EnergyFloor)
);

/// `R_0 = √(I_0/M)` of `ic`, against 1 (the canonical gauge's, which the distance floor and the Chazy rule read).
fn check_r0(scale: f64) {
    let pop = config_stability(16, 50.0);
    let basin = validation::escape_checks::config_basin(16, 50.0, 0.0);
    let deep = validation::escape_checks::deep_interior(16, 0.0);
    let (burrau, _, _) = validation::escape_checks::revalidation::burrau();
    for (name, ic) in [
        ("config pixel 0", &pop.ics[0]),
        ("config pixel 104", &pop.ics[104]),
        ("config pixel 255", &pop.ics[255]),
        ("config basin pixel 0", &basin.ics[0]),
        ("config basin pixel 255", &basin.ics[255]),
        ("deep interior pixel 0", &deep.ics[0]),
        ("deep interior pixel 255", &deep.ics[255]),
        ("Burrau", burrau),
    ] {
        let r = ic.r.map(|x| [scale * x[0], scale * x[1]]);
        let r_0 = kernel::chazy::hyperradius(&ic.m, &r);
        assert!(
            (r_0 - 1.0).abs() < 1e-12,
            "{name}: R_0 = {r_0}, not the gauge's 1"
        );
    }
}

#[test]
fn escape_rules_gauge_r0_is_one() {
    check_r0(1.0);
}

negative_control!(
    escape_rules_gauge_r0_is_one,
    "the ICs scaled by 2",
    expected = "not the gauge's 1",
    check_r0(2.0)
);
