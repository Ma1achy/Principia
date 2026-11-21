//! The energy-suspect threshold on the default occupant (integrator_contract Part 1; R-566, R-605; TASK-M3-41):
//! **`suspect-default-occupant`**, a measurement (R-649) — REQ-INT-106's: Heggie × RK4's regularised drift on
//! dd_integrator §5's tests 2 and 7 at f32 and f64 and what `SUSPECT_ENERGY` lights on it at REQ-INT-085's threshold
//! — and its `--propose`, REQ-INT-109's recalibrated threshold, an R-71 calibration the human confirms at the M3 gate.
//!
//! **What is read** (written before any run): each run through the one wrapper and its cadence, as the fixtures of
//! `crate::drift` run (test 2's figure-eight to `T = 200`, test 7's close-encounter hierarchy to `T = 50`, the default
//! sim key), its stored `dE_reg_max` — the regularised drift's latch under Heggie, the Cartesian under a stepper, where
//! the two energies are one — over `max(ε_E, |E_0|)`, `ε_E = 10⁻⁶`: the relative drift the energy-suspect predicate
//! reads (`kernel::payload::suspect_energy`). Heggie × RK4 at the CPU field's values (`η = 0.01`, `Δ_sync` the
//! register's, the reference's `TINY`, `f = 0.02`), with a budget that never binds on these runs.
//!
//! **The proposal** (the rule written before the runs): the threshold `τ_E` is the power of two nearest, in `log₂`, to
//! the geometric middle of the gap between the greatest relative drift that must not light and the least that must:
//! below, test 2's symplectic occupants (KDK, Yoshida-4, Yoshida-6) at f32 and f64, REQ-INT-085's population, and
//! Heggie × RK4's regularised drift on tests 2 and 7 at f32 and f64, the default occupant on two clean fixtures; above,
//! test 2's Euler at f32 and f64, REQ-INT-085's and REQ-VAL-031's. It is proposed only if it lies strictly inside the
//! gap. At it, the report gives what Euler, RK4, KDK, Yoshida-4, Yoshida-6 and Heggie × RK4 light on tests 2 and 7 at
//! each precision, and whether REQ-INT-085's pass conditions (on test 2 Euler lights and no symplectic occupant does)
//! and REQ-VAL-031's (Euler lights test 2) still hold.

use std::fmt::Write as _;
use std::path::Path;

use serde::Deserialize;

use kernel::driver::advance::Advance;
use kernel::driver::wrapper::WrapperReal;
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::payload::{dr_dE_reg_max, pb_dE_max, SUSPECT_ENERGY_THRESHOLD};
use kernel::regularisation::az::AzReal;
use kernel::regularisation::heggie::{Heggie, HgConfig, HgParams, DEFAULT_TIME};
use kernel::regularisation::Composed;

use crate::drift::{close_encounter, figure_eight, integrate_fixture, AsR, Fixture, EPS_DEFAULT};
use crate::gate::report::Verdict;
use crate::gate::{MeasurementConfig, MeasurementControl, MeasurementGate};

/// The gate's name.
pub const SUSPECT_NAME: &str = "suspect-default-occupant";

/// The CPU field's Heggie values with a budget that never binds on these runs (the module's note).
pub struct Field;

impl HgConfig for Field {
    fn params<R: AzReal>() -> HgParams<R> {
        HgParams {
            eta: R::at_width(0.01, 0.01),
            delta_sync: kernel::regularisation::sync::delta_sync::<R>(),
            tiny: R::at_width(1e-37, 1e-300),
            predictive_f: R::at_width(0.02, 0.02),
            step_budget_rate: R::at_width(1e7, 1e7),
            n_min: 100_000_000,
            clamp: true,
            secant_tries: 4,
            bisect_tries: 64,
            time: DEFAULT_TIME,
        }
    }
}

/// Heggie × RK4 at [`Field`]'s values.
pub type DefaultOccupant = Composed<Rk4, Heggie<Field>>;

/// One run's relative drifts, as the predicate reads them: `dE_reg_max` and the Cartesian `dE_max`, each over
/// `max(ε_E, |E_0|)`.
#[derive(Clone, Copy, Debug)]
pub struct Read {
    pub regularised: f64,
    pub cartesian: f64,
}

/// `A`'s run on `fixture` at `R`, read as the predicate reads it.
pub fn read<A: Advance, R: WrapperReal + Into<f64>>(fixture: &Fixture) -> Read
where
    f64: AsR<R>,
{
    let (out, _) = integrate_fixture::<A, R>(fixture);
    let e_0: f64 = out.E_0.into();
    let floor = EPS_DEFAULT.max(e_0.abs());
    Read {
        regularised: f64::from(dr_dE_reg_max(out.dE_reg)) / floor,
        cartesian: f64::from(pb_dE_max(out.packed_b)) / floor,
    }
}

/// An occupant's runs: its name, whether it is symplectic, and its reads on test 2 and test 7 at f32 and f64.
pub struct Row {
    pub occupant: &'static str,
    pub symplectic: bool,
    /// `[test 2 f32, test 2 f64, test 7 f32, test 7 f64]`.
    pub reads: [Read; 4],
}

fn row<A: Advance>(occupant: &'static str, symplectic: bool) -> Row {
    let (t2, t7) = (figure_eight(), close_encounter());
    Row {
        occupant,
        symplectic,
        reads: [
            read::<A, f32>(&t2),
            read::<A, f64>(&t2),
            read::<A, f32>(&t7),
            read::<A, f64>(&t7),
        ],
    }
}

/// Every occupant's runs: the five steppers, then the default occupant.
pub fn measure() -> Vec<Row> {
    vec![
        row::<Euler>("Euler", false),
        row::<Rk4>("RK4", false),
        row::<Kdk>("KDK", true),
        row::<Yoshida4>("Yoshida-4", true),
        row::<Yoshida6>("Yoshida-6", true),
        row::<DefaultOccupant>("Heggie × RK4", false),
    ]
}

/// The runs' columns, in [`Row::reads`]' order.
pub const COLUMNS: [&str; 4] = ["test 2 f32", "test 2 f64", "test 7 f32", "test 7 f64"];

/// The table of `rows`' relative regularised drifts (what the predicate reads), each marked lit at `tau`.
pub fn table(rows: &[Row], tau: f64) -> String {
    let mut out = format!(
        "| occupant | {} |\n|---|---|---|---|---|\n",
        COLUMNS.join(" | ")
    );
    for r in rows {
        let cells: Vec<String> = r
            .reads
            .iter()
            .map(|x| {
                let lit = if x.regularised > tau { " **lit**" } else { "" };
                format!("{:.3e}{lit}", x.regularised)
            })
            .collect();
        let _ = writeln!(out, "| {} | {} |", r.occupant, cells.join(" | "));
    }
    out
}

/// The default occupant's regularised and Cartesian reads side by side (REQ-INT-106).
fn default_table(rows: &[Row]) -> String {
    let h = rows
        .iter()
        .find(|r| r.occupant == "Heggie × RK4")
        .expect("the default occupant's row");
    let mut out = String::from("| Heggie × RK4 | regularised | Cartesian |\n|---|---|---|\n");
    for (c, x) in COLUMNS.iter().zip(h.reads) {
        let _ = writeln!(out, "| {c} | {:.3e} | {:.3e} |", x.regularised, x.cartesian);
    }
    out
}

/// The proposal's populations (the module's note): `(below, above)`.
pub fn populations(rows: &[Row]) -> (Vec<f64>, Vec<f64>) {
    let mut below = Vec::new();
    let mut above = Vec::new();
    for r in rows {
        match r.occupant {
            "Euler" => above.extend([r.reads[0].regularised, r.reads[1].regularised]),
            "Heggie × RK4" => below.extend(r.reads.iter().map(|x| x.regularised)),
            _ if r.symplectic => below.extend([r.reads[0].regularised, r.reads[1].regularised]),
            _ => {}
        }
    }
    (below, above)
}

/// The power of two nearest in `log₂` to the geometric middle of `(lo, hi)`, if it lies strictly inside.
pub fn power_of_two_in_gap(lo: f64, hi: f64) -> Option<f64> {
    if !(lo > 0.0 && hi > lo) {
        return None;
    }
    let tau = 2f64.powf((0.5 * (lo.log2() + hi.log2())).round());
    (tau > lo && tau < hi).then_some(tau)
}

/// Whether REQ-INT-085's pass conditions hold at `tau`: on test 2 Euler lights and no symplectic occupant does, at
/// either precision. REQ-VAL-031's is its Euler half.
pub fn int085_holds(rows: &[Row], tau: f64) -> (bool, bool) {
    let mut euler = true;
    let mut quiet = true;
    for r in rows {
        let t2 = [r.reads[0].regularised, r.reads[1].regularised];
        if r.occupant == "Euler" {
            euler &= t2.iter().all(|&x| x > tau);
        } else if r.symplectic {
            quiet &= t2.iter().all(|&x| x <= tau);
        }
    }
    (euler && quiet, euler)
}

/// `--propose`'s report (the module's note).
pub fn propose() -> Result<String, String> {
    let rows = measure();
    let now = f64::from(SUSPECT_ENERGY_THRESHOLD);
    let (below, above) = populations(&rows);
    let lo = below.iter().copied().fold(0.0, f64::max);
    let hi = above.iter().copied().fold(f64::INFINITY, f64::min);
    let mut out = format!(
        "# {SUSPECT_NAME} --propose (REQ-INT-106, REQ-INT-109)\n\nThe relative drift the energy-suspect predicate \
         reads, the stored `dE_reg_max` over max(ε_E, |E_0|), ε_E = 1e-6; Heggie × RK4 at the CPU field's values.\n\n\
         ## REQ-INT-106: the default occupant's two drifts\n\n{}\n## At REQ-INT-085's threshold, {now:e}\n\n{}",
        default_table(&rows),
        table(&rows, now)
    );
    let _ = writeln!(
        out,
        "\n## The gap\n\nMust not light: the greatest is {lo:.4e} (test 2's KDK, Yoshida-4 and Yoshida-6, and Heggie × \
         RK4 on tests 2 and 7, at f32 and f64). Must light: the least is {hi:.4e} (test 2's Euler at f32 and f64)."
    );
    let Some(tau) = power_of_two_in_gap(lo, hi) else {
        let _ = writeln!(
            out,
            "\nNo power of two lies strictly inside the gap: no threshold is proposed."
        );
        return Err(out);
    };
    let (int085, val031) = int085_holds(&rows, tau);
    let _ = writeln!(
        out,
        "\n**Proposed τ_E = {tau:e} = 2^{}**, the power of two nearest the gap's geometric middle, {:.4e}.\n\n## At the \
         proposed threshold\n\n{}\nREQ-INT-085's pass conditions (on test 2 Euler lights, no symplectic occupant \
         does): {}. REQ-VAL-031's (Euler lights test 2): {}.",
        tau.log2().round(),
        (lo * hi).sqrt(),
        table(&rows, tau),
        if int085 { "hold" } else { "DO NOT HOLD" },
        if val031 { "holds" } else { "DOES NOT HOLD" },
    );
    if int085 && val031 {
        Ok(out)
    } else {
        Err(out)
    }
}

/// The gate, a measurement (R-649).
pub struct SuspectGate;

/// A control input: which latch the reader under test reads.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    description: String,
    /// `regularised` or `cartesian`.
    reader: String,
}

impl MeasurementGate for SuspectGate {
    fn name(&self) -> &'static str {
        SUSPECT_NAME
    }

    /// The control: on test 7 at f64 under Heggie × RK4, whose close passages leave the Cartesian read-out far above the
    /// regularised one, the reader under test reads the regularised drift: below the Cartesian by more than four
    /// times. A Cartesian reader fails it.
    fn control(
        &self,
        _config: &MeasurementConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<MeasurementControl, String> {
        let c = Control::deserialize(fixture).map_err(|e| format!("{name}: {e}"))?;
        let x = read::<DefaultOccupant, f64>(&close_encounter());
        let got = match c.reader.as_str() {
            "regularised" => x.regularised,
            "cartesian" => x.cartesian,
            other => {
                return Err(format!(
                    "{name}: no reader `{other}` (regularised, cartesian)"
                ))
            }
        };
        let reads_regularised = got * 4.0 < x.cartesian;
        // REQ-INT-106's measurement rides on the regularised control: the default occupant's two drifts on tests 2
        // and 7, and what lights at REQ-INT-085's threshold.
        let measured = if c.reader == "regularised" {
            let rows = measure();
            let now = f64::from(SUSPECT_ENERGY_THRESHOLD);
            format!(
                "\n## REQ-INT-106\n\n{}\n### At REQ-INT-085's threshold, {now:e}\n\n{}",
                default_table(&rows),
                table(&rows, now)
            )
        } else {
            String::new()
        };
        Ok(MeasurementControl {
            description: c.description.clone(),
            verdict: if reads_regularised {
                Verdict::Pass
            } else {
                Verdict::Fail
            },
            markdown: format!(
                "# {SUSPECT_NAME}: {name}\n\n{}\n\n- test 7, Heggie × RK4, f64: the {} reader reads {got:.4e}; the \
                 regularised drift is {:.4e}, the Cartesian {:.4e}\n{measured}",
                c.description, c.reader, x.regularised, x.cartesian
            ),
        })
    }

    fn flags(&self, _root: &Path, args: &[&str]) -> Result<(), String> {
        match args {
            ["--propose"] => propose().map(|s| print!("{s}")).map_err(|s| {
                print!("{s}");
                format!("gate {SUSPECT_NAME}: no threshold proposed, or its conditions do not hold")
            }),
            _ => Err(format!(
                "gate {SUSPECT_NAME}: unrecognised flags `{}`; usage: --propose",
                args.join(" ")
            )),
        }
    }
}
