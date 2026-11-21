//! Re-registration's guarding tests (`validation::gate::reregistration`; pitfalls §4; TASK-M3-32), each with its
//! control. The measurements themselves are the gates' (`cargo xtask gate heggie-vs-az --res <n>`,
//! `reregistration-doubling --propose --res <n>`, `logh-falsification --res <n>`), which assert nothing.
//!
//! The criteria, written before the first run:
//! - **`unregularised_control`:** the pair table holds KDK under `none`, the leapfrog: order 2, symplectic, no
//!   regularisation (so no chart, no registration and no time mapping of its own), and the comparison harness refuses
//!   it as an arm, so it cannot share the regularised occupants' step control either; and it runs, through the
//!   wrapper, on the figure-eight to its horizon with its energy held to `1e-3` relative. A regularised pair fails.
//! - **The matrix:** `fixtures/case_matrix.toml` gives 32 cases, in its order, `config_stability` first with horizon
//!   50, the rest 13; the cases built are exactly the five Burrau regions, `body_plane`, `plane_00deg`, the ten
//!   axis-aligned latent planes, the two oblique planes, the eight presets, `burrau_nu_k`, `invariant_lz_k` and
//!   `config_stability` (30); `shape_sphere` and `mass_simplex` are listed as not run with their reasons.
//! - **The ICs are in their centre-of-mass frame**, positions and momenta, to round-off.
//! - **A dropped arm loses:** an arm that reached the horizon wins an IC the other did not reach, where the reference
//!   reached it.
//! - **The acceptance rule:** a level only in a decade gap, at the geometric mean rounded up to one significant figure.
//! - **The logH verdict:** a case logH lost by the reference is a loss even where energy says otherwise; a case no
//!   score decided is not counted.

use std::path::Path;

use kernel::occupant::kdk::Kdk;
use kernel::occupant::{Occupant, Selection, State};
use kernel::regularisation::az::End;
use kernel::regularisation::pairs::{Choice, Pair, PairTable, STEPPERS};
use validation::compare::{admit, Comparison, Reference, Refusal, Trajectory, Verdict};
use validation::gate::ablation::Raster;
use validation::gate::reregistration::{
    arm, case_named, cases, centred, ics, judge, logh_verdict, propose_level, Form, LogHRow,
    BASE_ETA, HEGGIE, UNREGULARISED,
};
use validation::negative_control;

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

// ── unregularised_control ──────────────────────────────────────────────────────────────────────────────────────

fn check_unregularised(pair: Pair) {
    assert!(
        STEPPERS.contains(&pair.stepper) && Choice::ALL.contains(&pair.regularisation),
        "the pair {pair:?} is not in the pair table"
    );
    let table = PairTable::part_2b();
    println!(
        "unregularised_control: {pair:?}: status {:?}; evidence: {}",
        pair.status(&table),
        table.evidence(pair.stepper, pair.regularisation)
    );
    assert_eq!(
        pair.regularisation,
        Choice::None,
        "the control {pair:?} shares coordinate machinery: it is regularised"
    );
    assert!(
        !pair.regularisation.kind().owns_time_mapping(),
        "the control {pair:?} shares coordinate machinery: it owns a time mapping"
    );
    let profile = pair.stepper.profile();
    assert!(
        pair.stepper == Selection::Production(Occupant::Kdk)
            && profile.order == 2
            && profile.symplectic,
        "the control {pair:?} is not the leapfrog (order {}, symplectic {})",
        profile.order,
        profile.symplectic
    );
    let mut a = arm(pair.regularisation, BASE_ETA, 0.125, 13.0);
    a.stepper = pair.stepper;
    let b = arm(HEGGIE, BASE_ETA, 0.125, 13.0);
    assert_eq!(
        admit(&[b, a]),
        Err(Refusal::Unregularised { arm: 1 }),
        "the harness admits the control {pair:?} beside a regularised arm: it shares their step control"
    );
    // It runs: the leapfrog through the wrapper on the figure-eight to its horizon, energy held to 1e-3 relative.
    let d = validation::drift::run::<Kdk, f64>(&validation::drift::figure_eight());
    println!(
        "unregularised_control: KDK on the figure-eight: {} steps, max |dE|/|E0| {:.3e}",
        d.steps,
        d.de_max / d.e_0.abs()
    );
    assert!(
        d.steps > 0 && (d.de_max / d.e_0.abs()) < 1e-3,
        "the control does not run: {} steps, max |dE| {:e}",
        d.steps,
        d.de_max
    );
}

#[test]
fn unregularised_control() {
    check_unregularised(UNREGULARISED);
}

negative_control!(
    unregularised_control,
    "a regularised pair (KDK under Aarseth–Zare) is not the control",
    expected = "shares coordinate machinery",
    check_unregularised(Pair::select(
        Selection::Production(Occupant::Kdk),
        Choice::AarsethZare
    ))
);

// ── the matrix ─────────────────────────────────────────────────────────────────────────────────────────────────

const BUILT: [&str; 30] = [
    "config_stability",
    "near_field",
    "mid_field",
    "far",
    "body2_core",
    "deep_interior",
    "body_plane",
    "plane_00deg",
    "latent_shape",
    "latent_inner_p",
    "latent_outer_p",
    "latent_mass",
    "latent_mixed",
    "latent_oblique_a",
    "latent_oblique_b",
    "burrau_nu_k",
    "invariant_lz_k",
    "preset_shape",
    "preset_prho",
    "preset_plambda",
    "preset_shape_pl",
    "preset_shape_h1",
    "preset_prho_h1",
    "preset_plambda_h1",
    "preset_shape_pl_h1",
    "latent_shape_h3",
    "latent_inner_p_h3",
    "latent_outer_p_h3",
    "latent_mass_h3",
    "latent_mixed_h3",
];

fn check_matrix(names: &[(String, f64, bool)]) {
    assert_eq!(
        names.len(),
        32,
        "the matrix has {} cases, not 32",
        names.len()
    );
    assert_eq!(
        names[0].0, "config_stability",
        "the matrix does not start at config_stability"
    );
    for (name, horizon, built) in names {
        let want = if name == "config_stability" {
            50.0
        } else {
            13.0
        };
        assert_eq!(
            *horizon, want,
            "case {name}'s horizon is {horizon}, not {want}"
        );
        assert_eq!(
            *built,
            BUILT.contains(&name.as_str()),
            "case {name}: built {built}, not as the matrix's charts say"
        );
    }
}

fn matrix() -> Vec<(String, f64, bool)> {
    cases(&root())
        .expect("the matrix")
        .into_iter()
        .map(|c| {
            let built = !matches!(c.form, Form::NotBuilt(_));
            (c.name, c.horizon, built)
        })
        .collect()
}

#[test]
fn the_matrix_is_the_fixture() {
    check_matrix(&matrix());
}

negative_control!(
    the_matrix_is_the_fixture,
    "a matrix that lost a case is not the fixture",
    expected = "cases, not 32",
    {
        let mut m = matrix();
        m.pop();
        check_matrix(&m);
    }
);

// ── the centre-of-mass frame ───────────────────────────────────────────────────────────────────────────────────

fn check_centred(s: &State<f64>) {
    let m: f64 = s.m.iter().sum();
    for k in 0..2 {
        let r: f64 = (0..3).map(|i| s.m[i] * s.r[i][k]).sum::<f64>() / m;
        let p: f64 = (0..3).map(|i| s.p[i][k]).sum();
        let rs = (0..3).map(|i| s.r[i][k].abs()).fold(1.0, f64::max);
        let ps = (0..3).map(|i| s.p[i][k].abs()).fold(1.0, f64::max);
        assert!(
            r.abs() <= 1e-14 * rs && p.abs() <= 1e-14 * ps,
            "the IC is not in its centre-of-mass frame: centre {r:e}, momentum {p:e}"
        );
    }
}

fn an_off_centre_ic() -> State<f64> {
    let case = case_named(&root(), "near_field").expect("near_field");
    let Form::BodyPlane { .. } = case.form else {
        panic!("near_field is not a body plane")
    };
    let mut r = validation::gate::reregistration::BURRAU_R;
    r[0] = [1.01, 3.02];
    State {
        m: validation::gate::reregistration::BURRAU_M,
        r,
        p: [[0.0; 2]; 3],
    }
}

#[test]
fn the_ics_are_centred() {
    for name in [
        "near_field",
        "body2_core",
        "latent_mass_h3",
        "config_stability",
        "burrau_nu_k",
    ] {
        let case = case_named(&root(), name).expect(name);
        let all = ics(&case, Raster::full(3));
        assert!(all.iter().any(Option::is_some), "{name}: no IC decoded");
        for s in all.iter().flatten() {
            check_centred(s);
        }
    }
    check_centred(&centred(&an_off_centre_ic()));
}

negative_control!(
    the_ics_are_centred,
    "a body-plane IC off its centre of mass, uncentred",
    expected = "not in its centre-of-mass frame",
    check_centred(&an_off_centre_ic())
);

// ── a dropped arm loses ────────────────────────────────────────────────────────────────────────────────────────

fn comparison(second: End, errors: [f64; 2]) -> Comparison {
    let s = an_off_centre_ic();
    let t = |end: End| Trajectory {
        state: s,
        boundary: if end == End::Marching { 104 } else { 50 },
        end,
        steps: 1,
        re_registrations: 0,
    };
    Comparison {
        arms: vec![arm(HEGGIE, BASE_ETA, 0.125, 13.0); 2],
        provenance: vec![String::new(); 2],
        ramp: validation::compare::RampConstants::POSITION_ERROR,
        boundary: 104,
        runs: vec![vec![t(End::Marching)], vec![t(second)]],
        reference: vec![Reference {
            state: Some(s),
            uncertainty: 1e-12,
        }],
        errors: vec![vec![errors[0]], vec![errors[1]]],
        panels: Vec::new(),
    }
}

fn check_dropped_loses(c: &Comparison) {
    assert_eq!(
        judge(c, 0, 1, 0),
        Verdict::A,
        "a dropped arm must lose the IC"
    );
    assert_eq!(
        judge(c, 1, 0, 0),
        Verdict::B,
        "a dropped arm must lose the IC"
    );
}

#[test]
fn a_dropped_arm_loses() {
    for end in [End::NonFinite, End::Exhausted] {
        check_dropped_loses(&comparison(end, [1e-9, f64::NAN]));
    }
    let both = comparison(End::Marching, [1e-9, 1e-9]);
    assert_eq!(
        judge(&both, 0, 1, 0),
        Verdict::Tie,
        "two equal arrivals tie"
    );
}

negative_control!(
    a_dropped_arm_loses,
    "both arms arrived with equal errors: nobody dropped, nobody loses",
    expected = "a dropped arm must lose the IC",
    check_dropped_loses(&comparison(End::Marching, [1e-9, 1e-9]))
);

// ── the acceptance rule ────────────────────────────────────────────────────────────────────────────────────────

fn check_level(default: f64, az: f64, want: Option<f64>) {
    let got = propose_level(default, az);
    let same = match (got, want) {
        (Some(g), Some(w)) => (g - w).abs() <= 1e-12 * w,
        (None, None) => true,
        _ => false,
    };
    assert!(
        same,
        "the level for {default:e} against {az:e} is {got:?}, not {want:?}"
    );
}

#[test]
fn the_level_sits_in_a_decade_gap() {
    // √(0.01 · 0.45) = 0.0671 → 0.07; √(0.002 · 0.4) = 0.0283 → 0.03.
    check_level(0.01, 0.45, Some(0.07));
    check_level(0.002, 0.4, Some(0.03));
    check_level(0.05, 0.1, None);
    check_level(0.05, 0.49, None);
    check_level(f64::NAN, 0.4, None);
}

negative_control!(
    the_level_sits_in_a_decade_gap,
    "a ratio of two is no decade gap: a level there is refused",
    expected = "the level for",
    check_level(0.05, 0.1, Some(0.08))
);

// ── the logH verdict ───────────────────────────────────────────────────────────────────────────────────────────

fn row(case: &str, tally: [usize; 5], energy: [usize; 5]) -> LogHRow {
    LogHRow {
        case: case.into(),
        ics: 4,
        tally,
        energy,
        log_ratio: 0.0,
        steps: [0.0; 2],
        energy_p50: [0.0; 2],
        dropped: [0; 2],
    }
}

fn check_verdict(rows: &[LogHRow], supported: bool) {
    let (got, lost) = logh_verdict(rows);
    assert_eq!(
        got, supported,
        "the verdict is {got} (lost {lost:?}), not {supported}"
    );
}

#[test]
fn the_logh_verdict_reads_each_decided_case() {
    // logH loses `a` by the reference although energy favours it; `b` decided nothing; `c` logH won.
    let rows = [
        row("a", [2, 1, 0, 1, 0], [0, 4, 0, 0, 0]),
        row("b", [0, 0, 0, 4, 0], [0, 0, 4, 0, 0]),
        row("c", [0, 1, 0, 3, 0], [0, 0, 4, 0, 0]),
    ];
    check_verdict(&rows, false);
    check_verdict(&rows[1..], true);
    // Where the reference decided nothing, energy decides.
    check_verdict(&[row("d", [0, 0, 0, 4, 0], [3, 1, 0, 0, 0])], false);
}

negative_control!(
    the_logh_verdict_reads_each_decided_case,
    "a case logH lost by the reference is a loss",
    expected = "the verdict is false",
    check_verdict(&[row("a", [2, 1, 0, 1, 0], [0, 4, 0, 0, 0])], true)
);
