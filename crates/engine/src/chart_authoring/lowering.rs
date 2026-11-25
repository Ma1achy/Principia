//! `resolve`'s lowering and labels for a registry chart (design § 6.2, § 7; TASK-M2-66): a validated [`ChartSpec`]
//! lowers one of three ways, and a reduced capability is a label, never a refusal.
//!
//! - **Uniform affine**: both axes latent directions, the latent chart's `z₀, q₁, q₂`.
//! - **Baked entry**: a registered pair, today's chart, its compute entry (§ 7.1).
//! - **CPU**: everything else, evaluated on the CPU in f64. A closed pair with no entry is labelled
//!   `CPU batch: not baked`; a registered numerical inverse `CPU batch: numerical`, its branch rule and seam shown
//!   (§ 7.3); a typed combination with no registered inverse takes R-849's general numerical solve, labelled as such
//!   and never as a registered numerical inverse (RQ-816 (a), point (i)).
//!
//! **The route.** The task's notes of 11 and 12 Nov say the lowering label becomes the fragment's route, `closed form`
//! or `table` (R-935), and that every chart is evaluated on the CPU (R-926). The plan is unclear here; chose to keep the
//! acceptance test's labels and give each lowering its route beside them: uniform and baked charts have closed-form
//! code, `closed form`; every other chart has none yet, `table`. Both are read off one [`Resolution`].
//!
//! Further labels: `approximate` for an axis on a conserved row steady only in a limit (§ 3.3), and
//! `paper scale differs` for a `gauge` axis at `I = 1` (§ 8.6).

use std::fmt;

use super::registry::{pair, Class, Constraints, Exactness, Inverse, PairMap, Scale, Warp};
use super::validate::{AxisInverse, GeneralSolve, ValidatedSpec};
use crate::contract::sim_config::Latent;

/// The fragment's route (R-935).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// The chart's closed-form code.
    ClosedForm,
    /// The CPU-built table.
    Table,
}

/// How a CPU chart inverts.
#[derive(Clone, Debug, PartialEq)]
pub enum CpuSolve {
    /// Closed, with no baked entry.
    Closed,
    /// A registered numerical inverse.
    Numerical {
        /// What it solves for.
        solves_for: &'static str,
        /// Its branch rule.
        branch: &'static str,
    },
    /// R-849's general numerical solve.
    General(GeneralSolve),
}

/// One of the three ways a pair runs (§ 7).
#[derive(Clone, Debug, PartialEq)]
pub enum Lowering {
    /// Both axes latent directions: the latent chart's uniforms.
    Uniform {
        /// The slice centre.
        z0: Latent,
        /// The horizontal direction.
        q1: Latent,
        /// The vertical direction.
        q2: Latent,
    },
    /// A registered pair's compute entry.
    Baked {
        /// The chart, by `kernel::chart::REGISTRY` name.
        chart: &'static str,
        /// Its compute entry.
        entry: &'static str,
    },
    /// The CPU.
    Cpu {
        /// The fragment's route.
        route: Route,
        /// How it inverts.
        solve: CpuSolve,
    },
}

impl Lowering {
    /// The fragment's route.
    pub fn route(&self) -> Route {
        match self {
            Lowering::Uniform { .. } | Lowering::Baked { .. } => Route::ClosedForm,
            Lowering::Cpu { route, .. } => *route,
        }
    }
}

/// A label: a reduced capability, never a refusal (§ 6.2).
#[derive(Clone, Debug, PartialEq)]
pub enum Label {
    /// Uniform affine, or a baked entry.
    Live,
    /// Closed-form, not in the baked set.
    NotBaked,
    /// A registered numerical inverse.
    Numerical,
    /// R-849's general numerical solve.
    GeneralSolve,
    /// The branch rule a solve takes.
    Branch(&'static str),
    /// Where the solve's seams are.
    Seam(&'static str),
    /// A conserved row steady only in a limit.
    Approximate,
    /// A `gauge` axis at `I = 1`.
    PaperScaleDiffers,
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Label::Live => f.write_str("live"),
            Label::NotBaked => f.write_str("CPU batch: not baked"),
            Label::Numerical => f.write_str("CPU batch: numerical"),
            Label::GeneralSolve => f.write_str("CPU batch: general numerical solve"),
            Label::Branch(rule) => write!(f, "branch: {rule}"),
            Label::Seam(at) => write!(f, "seam: {at}"),
            Label::Approximate => f.write_str("approximate"),
            Label::PaperScaleDiffers => f.write_str("paper scale differs"),
        }
    }
}

/// A validated chart's lowering and labels.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolution {
    /// How it runs.
    pub lowering: Lowering,
    /// Its labels.
    pub labels: Vec<Label>,
}

/// The lowering of each axis's row name, a latent direction along one coordinate counting as that coordinate's row.
fn names(chart: &ValidatedSpec) -> [Option<&'static str>; 2] {
    chart.rows().map(|r| r.map(|r| r.name))
}

/// Whether a validated chart says nothing beyond its two rows: no constraint, every axis at its default warp,
/// extension and holds, and each latent axis the unit direction of its coordinate. Only such a chart takes a baked
/// entry, which carries no orientation, scale, warp or hold of its own; any other goes to the CPU, so nothing a spec
/// says is dropped silently.
fn plain(chart: &ValidatedSpec) -> bool {
    let spec = chart.spec();
    let unit = |q: &Latent| {
        q.iter().filter(|&&x| x == 1.0).count() == 1 && q.iter().all(|&x| x == 0.0 || x == 1.0)
    };
    spec.constraints == Constraints::default()
        && spec.pair_map == PairMap::Independent
        && spec
            .axes
            .iter()
            .all(|a| a.warp == Warp::default() && a.extension.is_none() && a.holds.is_none())
        && chart.axes().iter().all(|a| match a {
            AxisInverse::Latent(q) => unit(q),
            _ => true,
        })
}

/// `resolve` for a registry chart (the module's note): it takes only a [`ValidatedSpec`], so a refused chart never
/// reaches it, and it returns a lowering for every one, never a refusal.
pub fn resolve_spec(chart: &ValidatedSpec) -> Resolution {
    resolve_spec_with(chart, true)
}

/// [`resolve_spec`], with the guard that only a chart saying nothing beyond its rows takes a baked entry on or off
/// (its control turns it off).
pub fn resolve_spec_with(chart: &ValidatedSpec, guard: bool) -> Resolution {
    let [h, v] = chart.axes();
    let mut labels = Vec::new();
    // The latent chart's uniforms carry no warp and no pair map: a chart with either goes to the CPU.
    let affine = chart.spec().pair_map == PairMap::Independent
        && chart
            .spec()
            .axes
            .iter()
            .all(|a| matches!(a.warp, Warp::Feasibility | Warp::Linear));
    let lowering = match (h, v) {
        (AxisInverse::Latent(q1), AxisInverse::Latent(q2)) if affine => Lowering::Uniform {
            z0: chart.spec().slice,
            q1: *q1,
            q2: *q2,
        },
        _ => match names(chart) {
            [Some(a), Some(b)] if pair(a, b).is_some() && (!guard || plain(chart)) => {
                let (chart, entry) = pair(a, b).expect("the pair");
                Lowering::Baked { chart, entry }
            }
            _ => {
                let general = chart.axes().iter().find_map(|a| match a {
                    AxisInverse::General(g) => Some(g.clone()),
                    _ => None,
                });
                let numerical = chart.axes().iter().find_map(|a| match a {
                    AxisInverse::Registered {
                        inverse: Inverse::Numerical { solves_for, branch },
                        ..
                    } => Some((*solves_for, *branch)),
                    _ => None,
                });
                let solve = match (general, numerical) {
                    (Some(g), _) => CpuSolve::General(g),
                    (None, Some((solves_for, branch))) => {
                        CpuSolve::Numerical { solves_for, branch }
                    }
                    (None, None) => CpuSolve::Closed,
                };
                Lowering::Cpu {
                    route: Route::Table,
                    solve,
                }
            }
        },
    };
    match &lowering {
        Lowering::Uniform { .. } | Lowering::Baked { .. } => labels.push(Label::Live),
        Lowering::Cpu { solve, .. } => match solve {
            CpuSolve::Closed => labels.push(Label::NotBaked),
            CpuSolve::Numerical { branch, .. } => labels.extend([
                Label::Numerical,
                Label::Branch(branch),
                Label::Seam("where the branch differs across a pixel edge"),
            ]),
            CpuSolve::General(g) => labels.extend([
                Label::GeneralSolve,
                Label::Branch(g.branch),
                Label::Seam(g.seams),
            ]),
        },
    }
    let rows = chart.rows().iter().flatten();
    if rows
        .clone()
        .any(|r| r.class == Class::Conserved(Exactness::Approximate))
    {
        labels.push(Label::Approximate);
    }
    if rows.clone().any(|r| r.scale == Scale::Gauge) {
        labels.push(Label::PaperScaleDiffers);
    }
    Resolution { lowering, labels }
}
