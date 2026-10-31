//! The evidence behind the drift thresholds and the `lin` range (TASK-M3-05; R-71, R-381, R-382; REQ-INT-085,
//! REQ-TOOL-150), measured through `validation::drift` on dd_integrator §5's test 2 and test 7, each occupant at f32 and
//! f64. Each test prints its table, with `--nocapture`, for the human's confirmation at the M3 gate:
//! - `drift_thresholds_proposal`: on test 2 Euler lights both predicates and no symplectic occupant lights either; on
//!   test 7 no symplectic occupant lights L_z. Test 7's energy spike and RK4 are recorded, not criteria. The measured
//!   populations are the register's.
//! - `drift_lin_range_proposal`: at the proposed `R`, Euler's and RK4's final drifts on test 2 are distinct 8-bit sRGB
//!   pixels, neither `c−` nor `c+`. The symplectic occupants may read `c0`, and test 7's drifts may saturate; both
//!   recorded, beside the default `symlog` view.
//!
//! **Domain** (pitfalls §10): the two fixtures, at the default sim key, on this platform's CPU; the GPU's f32 stands on
//! CPU-f32, as REQ-INT-030 has it.

#[cfg(feature = "controls")]
use kernel::occupant::euler::Euler;
#[cfg(feature = "controls")]
use kernel::occupant::kdk::Kdk;
use ledger::constants::{
    ConstantBuilder, Population, RelativeBasis, DRIFT_LIN_R_E, DRIFT_LIN_R_LZ,
    SUSPECT_ENERGY_THRESHOLD, SUSPECT_LZ_THRESHOLD,
};
use render::present::{self, DRIFT_LIN, DRIFT_SYMLOG};
use validation::drift::{self, Drifts, Fixture, Measured, EPS_DEFAULT};
use validation::negative_control;

const SYMPLECTIC: [&str; 3] = ["KDK", "Yoshida-4", "Yoshida-6"];

/// How far a measured population's end may sit from the register's, relatively: its figures are five digits, and a
/// libm's rounding moves a long f32 run's latch a little.
const REGISTER_TOL: f64 = 0.05;

/// The relative energy drift the stored latches give, which the energy predicate compares.
fn relative(d: &Drifts) -> f64 {
    d.de_max_stored / EPS_DEFAULT.max(f64::from(d.e_0_stored).abs())
}

fn runs(m: &Measured) -> [(&'static str, &Drifts); 2] {
    [("f32", &m.f32), ("f64", &m.f64)]
}

fn occupant<'a>(table: &'a [Measured], name: &str) -> &'a Measured {
    table
        .iter()
        .find(|m| m.occupant == name)
        .unwrap_or_else(|| panic!("no occupant `{name}`"))
}

/// Checks the criteria of REQ-INT-085 and R-382 at the thresholds `tau_e` and `tau_l`.
fn check_thresholds(test2: &[Measured], test7: &[Measured], tau_e: f64, tau_l: f64) {
    for (precision, d) in runs(occupant(test2, "Euler")) {
        assert!(
            relative(d) > tau_e,
            "Euler at {precision} doesn't light SUSPECT_ENERGY on test 2"
        );
        assert!(
            d.dlz_max_stored > tau_l,
            "Euler at {precision} doesn't light SUSPECT_LZ on test 2"
        );
    }
    for name in SYMPLECTIC {
        for (precision, d) in runs(occupant(test2, name)) {
            assert!(
                relative(d) <= tau_e,
                "{name} at {precision} lights SUSPECT_ENERGY on test 2"
            );
            assert!(
                d.dlz_max_stored <= tau_l,
                "{name} at {precision} lights SUSPECT_LZ on test 2"
            );
        }
        for (precision, d) in runs(occupant(test7, name)) {
            assert!(
                d.dlz_max_stored <= tau_l,
                "{name} at {precision} lights SUSPECT_LZ on test 7"
            );
        }
    }
}

/// Checks that the register's population holds what was measured, its ends within [`REGISTER_TOL`].
fn check_population(p: &Population, measured: &[f64]) {
    assert_eq!(measured.len() as u64, p.count, "`{}`'s count", p.name);
    let hi = measured.iter().copied().fold(f64::MIN, f64::max);
    assert!(
        (hi - p.hi).abs() <= REGISTER_TOL * p.hi,
        "`{}`: the register's top is {}, measured {hi}",
        p.name,
        p.hi
    );
    if p.lo > 0.0 {
        let lo = measured.iter().copied().fold(f64::MAX, f64::min);
        assert!(
            (lo - p.lo).abs() <= REGISTER_TOL * p.lo,
            "`{}`: the register's bottom is {}, measured {lo}",
            p.name,
            p.lo
        );
    }
}

/// `c`'s gap populations.
fn gap(c: &ConstantBuilder) -> (Population, Population) {
    match c.relative_basis {
        Some(RelativeBasis::Gap { below, above, .. }) => (below, above),
        _ => panic!("`{}` isn't set from a gap", c.name),
    }
}

/// `read` of each of the named occupants' runs on each table.
fn values(tables: &[&[Measured]], names: &[&str], read: fn(&Drifts) -> f64) -> Vec<f64> {
    tables
        .iter()
        .flat_map(|t| names.iter().map(|n| *occupant(t, n)))
        .flat_map(|m| [read(&m.f32), read(&m.f64)])
        .collect()
}

fn print_table(fixture: &str, table: &[Measured], tau_e: f64, tau_l: f64) {
    println!("{fixture}: occupant, precision: relative stored δE, |ΔE_final|, max|ΔE|; ΔL_z final, max|ΔL_z| (stored); E lit, L_z lit");
    for m in table {
        for (precision, d) in runs(m) {
            println!(
                "  {:<9} {precision}: {:.4e} {:.4e} {:.4e}; {:.4e} {:.4e} ({:.4e}); {} {}",
                m.occupant,
                relative(d),
                d.de_final.abs(),
                d.de_max,
                d.dlz_final,
                d.dlz_max,
                d.dlz_max_stored,
                relative(d) > tau_e,
                d.dlz_max_stored > tau_l,
            );
        }
    }
}

#[test]
fn drift_thresholds_proposal() {
    let test2 = drift::measure(&drift::figure_eight());
    let test7 = drift::measure(&drift::close_encounter());
    let (tau_e, tau_l) = (
        SUSPECT_ENERGY_THRESHOLD.number(),
        SUSPECT_LZ_THRESHOLD.number(),
    );
    println!("proposed: τ_E = {tau_e:e} on relative δE, τ_L = {tau_l:e} on absolute ΔL_z");
    print_table("test 2", &test2, tau_e, tau_l);
    print_table("test 7", &test7, tau_e, tau_l);
    check_thresholds(&test2, &test7, tau_e, tau_l);

    let (below, above) = gap(&SUSPECT_ENERGY_THRESHOLD);
    check_population(&below, &values(&[&test2], &SYMPLECTIC, relative));
    check_population(&above, &values(&[&test2], &["Euler"], relative));
    let (below, above) = gap(&SUSPECT_LZ_THRESHOLD);
    let stored_lz = |d: &Drifts| d.dlz_max_stored;
    check_population(&below, &values(&[&test2, &test7], &SYMPLECTIC, stored_lz));
    check_population(&above, &values(&[&test2], &["Euler"], stored_lz));
}

/// A short run of Euler and KDK, for the controls: whatever the horizon, Euler drifts and KDK's round-off is finite.
#[cfg(feature = "controls")]
fn short(fixture: Fixture) -> Vec<Measured> {
    let fixture = Fixture {
        horizon: 1.0,
        ..fixture
    };
    let both = |occupant, profile, f32: Drifts, f64: Drifts| Measured {
        occupant,
        profile,
        f32,
        f64,
    };
    use kernel::occupant::Stepper;
    SYMPLECTIC
        .iter()
        .map(|n| {
            both(
                n,
                Kdk::profile(),
                drift::run::<Kdk, f32>(&fixture),
                drift::run::<Kdk, f64>(&fixture),
            )
        })
        .chain([both(
            "Euler",
            Euler::profile(),
            drift::run::<Euler, f32>(&fixture),
            drift::run::<Euler, f64>(&fixture),
        )])
        .collect()
}

negative_control!(
    drift_thresholds_energy_over_euler,
    "an energy threshold above Euler's drift",
    expected = "doesn't light SUSPECT_ENERGY on test 2",
    {
        let t = short(drift::figure_eight());
        check_thresholds(&t, &t, 1e30, 1e30);
    }
);

negative_control!(
    drift_thresholds_lz_over_euler,
    "an L_z threshold above Euler's drift",
    expected = "doesn't light SUSPECT_LZ on test 2",
    {
        let t = short(drift::figure_eight());
        check_thresholds(&t, &t, 0.0, 1e30);
    }
);

negative_control!(
    drift_thresholds_energy_under_symplectic,
    "an energy threshold under the symplectic occupants' drift",
    expected = "lights SUSPECT_ENERGY on test 2",
    {
        let t = short(drift::figure_eight());
        check_thresholds(&t, &t, 0.0, 0.0);
    }
);

negative_control!(
    drift_thresholds_lz_under_symplectic,
    "an L_z threshold under the symplectic occupants' drift",
    expected = "lights SUSPECT_LZ on test 2",
    {
        let t = short(drift::figure_eight());
        // Above KDK's energy drift and under Euler's, so that the L_z check is the one reached.
        let tau_e =
            relative(&occupant(&t, "Euler").f32).min(relative(&occupant(&t, "Euler").f64)) / 2.0;
        check_thresholds(&t, &t, tau_e, 0.0);
    }
);

negative_control!(
    drift_thresholds_population,
    "a population whose top isn't what was measured",
    expected = "the register's top",
    check_population(
        &Population {
            name: "control",
            lo: 0.0,
            hi: 1.0,
            count: 1
        },
        &[2.0]
    )
);

/// The final drift of an energy or an `L_z` run.
fn final_energy(d: &Drifts) -> f64 {
    d.de_final
}

fn final_lz(d: &Drifts) -> f64 {
    d.dlz_final
}

/// A linear colour as the 8-bit sRGB pixel it shows.
fn pixel(c: [f64; 3]) -> [u8; 3] {
    c.map(|v| (present::linear_to_srgb(v) * 255.0).round() as u8)
}

fn lin_pixel(x: f64, r: f64) -> [u8; 3] {
    pixel(present::dbg_drift(
        x as f32,
        DRIFT_LIN,
        false,
        r,
        EPS_DEFAULT,
        [0.5, 0.5],
    ))
}

fn symlog_pixel(x: f64, r: f64) -> [u8; 3] {
    pixel(present::dbg_drift(
        x as f32,
        DRIFT_SYMLOG,
        false,
        r,
        EPS_DEFAULT,
        [0.5, 0.5],
    ))
}

/// Checks REQ-TOOL-150's criterion on test 2's table at `r`: Euler's and RK4's final drifts, at each precision, are
/// distinct pixels, neither the saturated end.
fn check_lin_range(table: &[Measured], read: fn(&Drifts) -> f64, r: f64) {
    let [lo, _, hi] = present::DRIFT_DIVERGING_SRGB8;
    for (euler, rk4, precision) in [
        (
            read(&occupant(table, "Euler").f32),
            read(&occupant(table, "RK4").f32),
            "f32",
        ),
        (
            read(&occupant(table, "Euler").f64),
            read(&occupant(table, "RK4").f64),
            "f64",
        ),
    ] {
        let (a, b) = (lin_pixel(euler, r), lin_pixel(rk4, r));
        for c in [a, b] {
            assert!(
                c != lo && c != hi,
                "at {precision}, a final drift is saturated at R = {r}"
            );
        }
        assert_ne!(
            a, b,
            "at {precision}, Euler's and RK4's final drifts are one pixel at R = {r}"
        );
    }
}

fn print_lin(
    fixture: &str,
    field: &str,
    table: &[Measured],
    read: fn(&Drifts) -> f64,
    max: fn(&Drifts) -> f64,
    r: f64,
) {
    println!(
        "{fixture}, {field}, R = {r:e}: occupant, precision: final, max; lin pixel, symlog pixel"
    );
    for m in table {
        for (precision, d) in runs(m) {
            println!(
                "  {:<9} {precision}: {:.4e} {:.4e}; {:?} {:?}",
                m.occupant,
                read(d),
                max(d),
                lin_pixel(read(d), r),
                symlog_pixel(read(d), r),
            );
        }
    }
}

#[test]
fn drift_lin_range_proposal() {
    let test2 = drift::measure(&drift::figure_eight());
    let test7 = drift::measure(&drift::close_encounter());
    let (r_e, r_l) = (DRIFT_LIN_R_E.number(), DRIFT_LIN_R_LZ.number());
    let max_e = |d: &Drifts| d.de_max;
    let max_l = |d: &Drifts| d.dlz_max;
    print_lin("test 2", "energy_drift", &test2, final_energy, max_e, r_e);
    print_lin("test 2", "Lz_drift", &test2, final_lz, max_l, r_l);
    print_lin("test 7", "energy_drift", &test7, final_energy, max_e, r_e);
    print_lin("test 7", "Lz_drift", &test7, final_lz, max_l, r_l);
    check_lin_range(&test2, final_energy, r_e);
    check_lin_range(&test2, final_lz, r_l);
}

/// Euler and RK4 at a short horizon, for the controls.
#[cfg(feature = "controls")]
fn short_lin() -> Vec<Measured> {
    let fixture = Fixture {
        horizon: 1.0,
        ..drift::figure_eight()
    };
    drift::measure(&fixture)
}

negative_control!(
    drift_lin_range_too_wide,
    "a range so wide that Euler and RK4 are one pixel",
    expected = "are one pixel",
    check_lin_range(&short_lin(), final_energy, 1e9)
);

negative_control!(
    drift_lin_range_too_narrow,
    "a range so narrow that Euler's drift saturates",
    expected = "is saturated",
    check_lin_range(&short_lin(), final_energy, 1e-12)
);

/// `M`, `I` of an IC in the sim's units (integrator contract Part 5), and its centre of mass.
fn units(ic: &kernel::decode::Ic<f64>) -> (f64, f64, [f64; 2]) {
    let total: f64 = ic.m.iter().sum();
    let com = [0, 1].map(|k| (0..3).map(|i| ic.m[i] * ic.r[i][k]).sum::<f64>() / total);
    let inertia = (0..3)
        .map(|i| ic.m[i] * (ic.r[i][0].powi(2) + ic.r[i][1].powi(2)))
        .sum();
    (total, inertia, com)
}

/// Checks that `fixture` holds the sim's units and its centre of mass at the origin.
fn check_units(fixture: &Fixture) {
    let (total, inertia, com) = units(&fixture.ic);
    assert!(
        (total - 1.0).abs() < 1e-12,
        "`{}`: M = {total}",
        fixture.name
    );
    assert!(
        (inertia - 1.0).abs() < 1e-12,
        "`{}`: I = {inertia}",
        fixture.name
    );
    assert!(
        com.iter().all(|c| c.abs() < 1e-12),
        "`{}`: R_com = {com:?}",
        fixture.name
    );
}

#[test]
fn drift_fixtures_are_the_definitions() {
    let test2 = drift::figure_eight();
    let test7 = drift::close_encounter();
    check_units(&test2);
    check_units(&test7);
    assert_eq!(test2.horizon, drift::T_CAP);
    assert_eq!(test7.horizon, drift::T_DEFAULT);
    // The figure-eight's masses are equal; its L_z, zero; its shape, closed: one period on, about where it began.
    assert!(test2.ic.m.iter().all(|m| (m - 1.0 / 3.0).abs() < 1e-12));
    let lz: f64 = (0..3)
        .map(|i| test2.ic.r[i][0] * test2.ic.p[i][1] - test2.ic.r[i][1] * test2.ic.p[i][0])
        .sum();
    assert!(lz.abs() < 1e-7, "L_z = {lz}");
    // Test 7's pericentre, above r_sub = 0.05, and about 0.067 (the module's note).
    let (ic, pericentre) = drift::hierarchy(
        drift::HIERARCHY_MASSES,
        drift::HIERARCHY_A,
        drift::HIERARCHY_E,
        drift::HIERARCHY_D,
    );
    assert!(
        (pericentre - 0.067).abs() < 0.001,
        "the pericentre is {pericentre}"
    );
    assert_eq!(ic.m, test7.ic.m);
}

negative_control!(
    drift_fixtures_are_the_definitions,
    "an IC out of the sim's units",
    expected = "I =",
    {
        let mut f = drift::figure_eight();
        f.ic.r[0][0] += 1.0;
        check_units(&f);
    }
);

/// Checks the sim key at horizon `t`: `dt_macro = max(1e-3, t/65535)`, `horizon_steps = ⌈t/dt_macro⌉`.
fn check_key(t: f64, dt: f64, steps: u32) {
    let key = drift::sim_key::<f64>(t);
    assert!(
        (key.dt_macro - dt).abs() <= 1e-15,
        "dt_macro {}",
        key.dt_macro
    );
    assert_eq!(key.horizon_steps, steps, "horizon_steps");
}

#[test]
fn drift_sim_key_at_and_past_the_floor() {
    check_key(50.0, 1e-3, 50_000);
    check_key(200.0, 200.0 / 65535.0, 65_535);
    // t/65535 = 1e-3 exactly at the floor's edge.
    check_key(65.535, 1e-3, 65_535);
    assert_eq!(drift::sim_key::<f32>(50.0).horizon_steps, 50_000);
}

negative_control!(
    drift_sim_key_at_and_past_the_floor,
    "a dt_macro under the floor",
    expected = "dt_macro",
    check_key(50.0, 50.0 / 65535.0, 50_000)
);

#[test]
fn drift_runs_are_read_whole() {
    let table = short_horizon_table();
    let euler = occupant(&table, "Euler");
    for d in [&euler.f32, &euler.f64] {
        assert_eq!(d.steps, 1000, "dt 1e-3 to T = 1");
        assert!(d.de_max >= d.de_final.abs() && d.de_max > 0.0);
        assert!(d.dlz_max >= d.dlz_final.abs());
        assert!(!d.saturated);
        let e = d.e_0.abs().max(EPS_DEFAULT);
        assert_eq!(d.rel_e_final(EPS_DEFAULT), d.de_final / e);
        assert_eq!(d.rel_e_max(EPS_DEFAULT), d.de_max / e);
        assert!(d.lz_0.abs() < 1e-5);
        assert_eq!(d.e_0_stored, d.e_0 as f32);
    }
    // The floor binds where |E₀| is under it.
    let mut small = euler.f64;
    small.e_0 = 1e-9;
    assert_eq!(small.rel_e_max(EPS_DEFAULT), small.de_max / EPS_DEFAULT);
    let names: Vec<_> = table.iter().map(|m| m.occupant).collect();
    assert_eq!(names, ["Euler", "RK4", "KDK", "Yoshida-4", "Yoshida-6"]);
    assert!(table
        .iter()
        .all(|m| { m.profile.symplectic == SYMPLECTIC.contains(&m.occupant) }));
}

negative_control!(
    drift_runs_are_read_whole,
    "an occupant the table doesn't hold",
    expected = "no occupant",
    {
        occupant(&short_horizon_table(), "Leapfrog");
    }
);

/// Every occupant to `T = 1` on test 2's orbit.
fn short_horizon_table() -> Vec<Measured> {
    let mut f = drift::figure_eight();
    f.horizon = 1.0;
    drift::measure(&f)
}

/// A builder of the hierarchy, as `drift::hierarchy`.
type Hierarchy = fn([f64; 3], f64, f64, f64) -> (kernel::decode::Ic<f64>, f64);

/// Checks the hierarchy's geometry in canonical units, the length scale `λ` from its pericentre: the binary's
/// separation and relative speed at apocentre, and the outer body's distance and relative speed about the binary's
/// centre of mass. `M = 1`, so lengths scale by `λ` and speeds by `λ^(−1/2)`. Unequal masses, so no weight is masked.
fn check_hierarchy(hierarchy: Hierarchy) {
    let (m, a, e, d) = ([0.5, 0.3, 0.2], 1.25, 0.8, 5.0);
    let (ic, pericentre) = hierarchy(m, a, e, d);
    let lambda = pericentre / (a * (1.0 - e));
    let v = |i: usize| [ic.p[i][0] / ic.m[i], ic.p[i][1] / ic.m[i]];
    let dist = |x: [f64; 2], y: [f64; 2]| (x[0] - y[0]).hypot(x[1] - y[1]);
    let mb = m[0] + m[1];
    let bary = |x: [f64; 2], y: [f64; 2]| [0, 1].map(|k| (m[0] * x[k] + m[1] * y[k]) / mb);
    let close = |got: f64, want: f64, what: &str| {
        assert!((got - want).abs() < 1e-9, "{what}: {got}, not {want}");
    };
    assert_eq!(ic.m, m);
    close(
        dist(ic.r[0], ic.r[1]),
        a * (1.0 + e) * lambda,
        "the binary's separation",
    );
    close(
        dist(ic.r[2], bary(ic.r[0], ic.r[1])),
        d * lambda,
        "the outer distance",
    );
    let va = (mb * (1.0 - e) / (a * (1.0 + e))).sqrt();
    close(
        dist(v(0), v(1)),
        va / lambda.sqrt(),
        "the binary's relative speed",
    );
    close(
        dist(v(2), bary(v(0), v(1))),
        (1.0 / d).sqrt() / lambda.sqrt(),
        "the outer speed",
    );
}

#[test]
fn drift_hierarchy_is_its_definition() {
    check_hierarchy(drift::hierarchy);
}

negative_control!(
    drift_hierarchy_is_its_definition,
    "a hierarchy whose pericentre is misreported",
    expected = "the binary's separation",
    check_hierarchy(|m, a, e, d| {
        let (ic, p) = drift::hierarchy(m, a, e, d);
        (ic, 2.0 * p)
    })
);
