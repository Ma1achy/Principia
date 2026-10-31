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
#[cfg(feature = "controls")]
use validation::drift::Fixture;
use validation::drift::{self, Drifts, Measured, EPS_DEFAULT};
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
