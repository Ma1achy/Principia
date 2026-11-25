//! The well-posedness validator and the validated chart (chart_decoder_contract Part 5, "The check, in one line";
//! lowering contract Part 5; REQ-CHART-006, REQ-CHART-011, REQ-SYS-013).
//!
//! **The check.** Given each axis's block-touch set, conventions, invariant and curve status, and the slice, all 8 DOF
//! are determined exactly once, an annotation axis contributing none. The DOF are counted per group of blocks: a block
//! alone, or the blocks one item spans together (a curve's tied blocks, an oblique latent direction's, the latent
//! plane's complement). In a group a swept axis pins one DOF, a residual convention, a chart convention and a held set
//! their stated count; fewer than the group's DOF leave it under-determined, more double-write it. The blocks a kind-4
//! axis ties are pinned by its embed, every DOF of them (§ "The four axis kinds": "the embed pins config + mass
//! together"): a slice or convention pin there double-writes them, and so does a second kind-4 axis on any of them,
//! whose embed pins them again; a kind-1 or kind-2 axis inside them is a same-block collision, which pins nothing more
//! and which the chart's `coupling` flags. The latent directions the axes and the slice write must be linearly
//! independent, exactly: a direction written twice is double-written though the counts balance.
//!
//! **Kind 3.** An invariant axis sits downstream of every block it depends on, in the decoder's order mass → config →
//! momentum; one that reads its own sector or a later one is refused. Two invariant axes solving into one sector are
//! coupled and solve jointly ([`Solve`]), as `(L_z, E)` does. A `PE` axis is refused: chart_reference §2.1 writes no
//! warp for it, so no chart offers it (R-471 H2, REQ-CHART-070).
//!
//! **The flags.** The declared `annotation`, `has_feasibility_boundary` (invariant charts) and `coupling` agree with the
//! axes, and the view's energy target passes the chart's `forbids_energy_normalisation`
//! ([`crate::chart_validate::validate_view`]). The domain function `validate(u, v)` every chart has by its type
//! (`kernel::chart::Chart`, R-26).
//!
//! **Before resolution.** Only [`validate`] builds a [`ValidatedChart`], and [`resolve`] takes nothing else, so resolve
//! only ever sees a chart that passed. The validated chart is generic over the chart, which stays the type parameter the
//! kernel specialises (design axiom 4).

use std::fmt;

use kernel::chart::energy::{self, Quantity};
use kernel::chart::{family_label, swept, Axis, BlockTouch};
use kernel::decode::Decoded;
use kernel::generated::links::{LinkParams, Selection};

use super::axis::{
    blocks, dof, meets, names, no_blocks, union, within, AxisKind, Block, InvariantQuantity,
    COORDINATES,
};
use super::descriptor::{Authored, ChartDescriptor, ChartKind, Coupling, Frozen};
use super::registry::pair;
use super::registry::{
    AxisSpec, ChartSpec, Class, Constraints, Expr, Hold, Inverse, Primitive, Registry, Row, Scale,
    LATENT_ROWS,
};
use crate::chart_validate::{self, validate_view};
use crate::contract::sim_config::{Latent, LATENT_DIM};

/// Why the validator refuses a chart.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// The view's energy target, by the chart's `forbids_energy_normalisation` (R-25).
    View(chart_validate::Refusal),
    /// Neither axis is swept.
    NoSweptAxis,
    /// The axis `axis` is of a quantity chart_reference §2.1 writes no warp for, `PE`: no chart offers it
    /// (REQ-CHART-070, R-72; R-471 H2).
    NoWarpWritten { axis: &'static str },
    /// The invariant axis `axis` solves into `sector` but reads `depends_on`, not all upstream of it.
    InvariantNotDownstream {
        axis: &'static str,
        sector: Block,
        depends_on: BlockTouch,
    },
    /// The group `blocks` has `dof` DOF and `pinned` fewer.
    UnderDetermined {
        blocks: BlockTouch,
        pinned: u32,
        dof: u32,
    },
    /// The group `blocks` has `dof` DOF and `pinned` more.
    DoubleWritten {
        blocks: BlockTouch,
        pinned: u32,
        dof: u32,
    },
    /// The latent directions written, the axes' and the slice's, are linearly dependent: `what` is written twice.
    DependentDirections { what: String },
    /// The axis `axis`'s latent direction has a non-finite component.
    NonFiniteDirection { axis: &'static str },
    /// The declared flag `flag` is `declared`, the axes imply `implied`.
    Flag {
        flag: &'static str,
        declared: String,
        implied: String,
    },
    /// An axis and a constraint on one quantity (design § 6.1; generalises R-25): `axis` reads `quantity`, which the
    /// chart holds.
    AxisAndHold {
        axis: String,
        quantity: &'static str,
    },
    /// The two axes are rank 1: one quantity (§ 6.1; the pair table, [`SAME_QUANTITY`]).
    SameQuantity { a: String, b: String },
    /// The pair has no inverse: `a` is a registered `none` row picked as an axis (§ 6.1; RQ-816 (a), point (iii)).
    NoJointInverse { a: String, b: String },
    /// The axis `axis` is size-only: `I = 1` at every decode makes it constant (§ 3).
    SizeOnly { axis: String },
    /// The axis `axis` is a trajectory functional, which is never an axis (§ 3.3).
    Functional { axis: String },
    /// The axis `axis` names no registered row.
    UnknownPrimitive { axis: String },
    /// The axis `axis` reads an unclassified row, whose definition is unread: a definition requirement, no axis
    /// (§ 3.3).
    Unclassified { axis: String },
    /// The typed combination `axis` reads the row `part`, which has no forward of its own to evaluate, so no solve
    /// has a residual: a row closed only on its chart's pair is that chart's encode, never a part of an expression.
    NoForward { axis: String, part: &'static str },
    /// The axis `axis` is closed only on its chart's pair, `chart`, and is used off it; its forward is that chart's
    /// encode, so it has none of its own for a solve to read.
    PairOnly {
        axis: &'static str,
        chart: &'static str,
    },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::View(r) => write!(f, "{r}"),
            Refusal::NoSweptAxis => f.write_str("refused: neither axis is swept"),
            Refusal::NoWarpWritten { axis } => write!(
                f,
                "refused: the axis {axis} is of a quantity chart_reference §2.1 writes no warp for"
            ),
            Refusal::InvariantNotDownstream {
                axis,
                sector,
                depends_on,
            } => write!(
                f,
                "refused: the invariant axis {axis} solves into {} but depends on {}, not all upstream of it",
                sector.name(),
                names(*depends_on)
            ),
            Refusal::UnderDetermined {
                blocks,
                pinned,
                dof,
            } => write!(
                f,
                "refused: {} is under-determined, {pinned} of its {dof} DOF pinned",
                names(*blocks)
            ),
            Refusal::DoubleWritten {
                blocks,
                pinned,
                dof,
            } => write!(
                f,
                "refused: {} is double-written, {pinned} pins on its {dof} DOF",
                names(*blocks)
            ),
            Refusal::DependentDirections { what } => {
                write!(f, "refused: a latent direction is double-written: {what}")
            }
            Refusal::NonFiniteDirection { axis } => {
                write!(f, "refused: the axis {axis}'s latent direction is not finite")
            }
            Refusal::Flag {
                flag,
                declared,
                implied,
            } => write!(
                f,
                "refused: the chart declares {flag} = {declared}, its axes imply {implied}"
            ),
            Refusal::AxisAndHold { axis, quantity } => write!(
                f,
                "refused: the axis {axis} reads {quantity}, which the chart holds"
            ),
            Refusal::SameQuantity { a, b } => {
                write!(f, "refused: the axes {a} and {b} are one quantity")
            }
            Refusal::NoJointInverse { a, b } => write!(
                f,
                "refused: the pair ({a}, {b}) has no inverse: {a} is registered with none"
            ),
            Refusal::SizeOnly { axis } => write!(
                f,
                "refused: the axis {axis} is size-only, constant at every decode (I = 1)"
            ),
            Refusal::Functional { axis } => write!(
                f,
                "refused: the axis {axis} is a trajectory functional, never an axis"
            ),
            Refusal::UnknownPrimitive { axis } => {
                write!(f, "refused: the axis {axis} names no registered primitive")
            }
            Refusal::Unclassified { axis } => write!(
                f,
                "refused: the axis {axis} reads an unclassified row, a definition requirement and no axis"
            ),
            Refusal::PairOnly { axis, chart } => write!(
                f,
                "refused: the axis {axis} is closed only on the {chart} chart's pair, and off it has no forward of its own to solve"
            ),
            Refusal::NoForward { axis, part } => write!(
                f,
                "refused: the combination {axis} reads {part}, which has no forward to evaluate"
            ),
        }
    }
}

impl std::error::Error for Refusal {}

/// A sector's invariant solve: the invariant axes solving into it, jointly where there are two.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Solve {
    /// The sector.
    pub sector: Block,
    /// The invariant axes solving into it.
    pub axes: Vec<Axis>,
    /// Whether they solve jointly: two coupled invariants.
    pub joint: bool,
}

/// A chart that passed [`validate`]: only the validator builds one, its fields private (checked by
/// `tests/validated_chart.rs` on `fixtures/validated_chart`).
#[derive(Clone, Debug)]
pub struct ValidatedChart<C> {
    chart: C,
    descriptor: ChartDescriptor,
    solves: Vec<Solve>,
}

impl<C: Authored> ValidatedChart<C> {
    /// The chart.
    pub fn chart(&self) -> &C {
        &self.chart
    }

    /// Its descriptor, as validated.
    pub fn descriptor(&self) -> &ChartDescriptor {
        &self.descriptor
    }

    /// The swept axes: 2, or 1 for a 1-swept family rendered in 2D.
    pub fn swept(&self) -> u32 {
        swept(self.descriptor.axes.map(|a| a.meta()))
    }

    /// The DOF the swept axes leave: 6, or 7 beside an annotation axis.
    pub fn frozen(&self) -> u32 {
        LATENT_DIM as u32 - self.swept()
    }

    /// The label the chart carries for its well-posedness (`kernel::chart::family_label`).
    pub fn label(&self) -> &'static str {
        family_label(self.descriptor.axes.map(|a| a.meta()))
    }

    /// The invariant solves, by sector in the decoder's order.
    pub fn solves(&self) -> &[Solve] {
        &self.solves
    }

    /// Whether the chart is affine or nonlinear.
    pub fn kind(&self) -> ChartKind {
        self.descriptor.kind()
    }

    /// The CPU decode path at `(u, v)` under `S`, as the lock replays it (render_gui_spec §G7): the validated
    /// chart's own map, then the decoder (`Chart::decode`), whatever its kind; an affine chart's map is its own slice
    /// (the latent chart's `z₀ + (2u−1)q₁ + (2v−1)q₂`), so no plane but the one validated is ever decoded.
    pub fn decode<S: Selection>(&self, u: f64, v: f64) -> Decoded<f64>
    where
        f64: From<C::Param>,
    {
        self.decode_params::<S>(u, v, &LinkParams::DEFAULT)
    }

    /// [`ValidatedChart::decode`] at the tunable parameters `p`, the active values (`Chart::decode_params`; chart_reference
    /// §0.1, "every chart reads the active value"; R-481 G4).
    pub fn decode_params<S: Selection>(&self, u: f64, v: f64, p: &LinkParams<f64>) -> Decoded<f64>
    where
        f64: From<C::Param>,
    {
        self.chart.decode_params::<S, f64>(u, v, p)
    }
}

/// What resolve reads of a validated chart (lowering contract Part 5): the chart, the type its compute key hashes,
/// its kind, and whether its kinds demand a curve tangent or invariant targets among the chart uniforms. The keys and
/// the dispatch are TASK-M4-08's.
#[derive(Clone, Copy, Debug)]
pub struct Resolved<'a, C> {
    /// The chart.
    pub chart: &'a C,
    /// Its kind.
    pub kind: ChartKind,
    /// A kind-4 axis: the uniforms carry its tangent.
    pub curve_tangent: bool,
    /// A kind-3 axis: the uniforms carry the invariant targets.
    pub invariant_targets: bool,
}

/// The resolve entry: it takes only a [`ValidatedChart`], so an invalid chart never reaches it.
pub fn resolve<C: Authored>(chart: &ValidatedChart<C>) -> Resolved<'_, C> {
    let has = |kind| chart.descriptor.axes.iter().any(|a| a.kind() == Some(kind));
    Resolved {
        chart: &chart.chart,
        kind: chart.kind(),
        curve_tangent: has(AxisKind::CoupledCurve),
        invariant_targets: has(AxisKind::CrossBlockInvariant),
    }
}

/// The validation pass on `chart` under the view's energy target `e_star` (the module's note): the [`ValidatedChart`]
/// where it passes, else the first [`Refusal`].
pub fn validate<C: Authored>(chart: C, e_star: Option<f64>) -> Result<ValidatedChart<C>, Refusal> {
    validate_view(&chart, e_star).map_err(Refusal::View)?;
    let descriptor = chart.descriptor();
    if swept(descriptor.axes.map(|a| a.meta())) == 0 {
        return Err(Refusal::NoSweptAxis);
    }
    warps(&descriptor)?;
    downstream(&descriptor)?;
    count(&descriptor)?;
    directions(&descriptor)?;
    flags(&descriptor)?;
    let solves = solves(&descriptor);
    Ok(ValidatedChart {
        chart,
        descriptor,
        solves,
    })
}

/// Every invariant axis of a quantity chart_reference §2.1 writes a warp for (`kernel::chart::energy::written`): a `PE`
/// axis is refused (R-471 H2).
fn warps(d: &ChartDescriptor) -> Result<(), Refusal> {
    for axis in &d.axes {
        if let Some(inv) = axis.invariant_meta() {
            if energy::written(quantity(inv.quantity())).is_err() {
                return Err(Refusal::NoWarpWritten { axis: axis.name() });
            }
        }
    }
    Ok(())
}

/// The kernel's quantity for an invariant axis's.
fn quantity(q: InvariantQuantity) -> Quantity {
    match q {
        InvariantQuantity::E => Quantity::E,
        InvariantQuantity::Lz => Quantity::Lz,
        InvariantQuantity::K => Quantity::K,
        InvariantQuantity::KE => Quantity::KE,
        InvariantQuantity::PE => Quantity::PE,
        InvariantQuantity::Q => Quantity::VirialRatio,
    }
}

/// Every invariant axis downstream of each block it depends on.
fn downstream(d: &ChartDescriptor) -> Result<(), Refusal> {
    for axis in &d.axes {
        if let Some(inv) = axis.invariant_meta() {
            let sector = inv.sector();
            if blocks(inv.depends_on()).any(|b| b.position() >= sector.position()) {
                return Err(Refusal::InvariantNotDownstream {
                    axis: axis.name(),
                    sector,
                    depends_on: inv.depends_on(),
                });
            }
        }
    }
    Ok(())
}

/// What a pin is, for the count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// A swept axis: one DOF.
    Swept,
    /// A kind-4 axis: its embed pins its group.
    Curve,
    /// A residual convention, a chart convention or a held set: its count.
    Fixed,
}

/// The pins the count reads: the blocks each lies in, its DOF and its role.
fn pins(d: &ChartDescriptor) -> Vec<(BlockTouch, u32, Role)> {
    let mut pins = Vec::new();
    for axis in d.axes.iter().filter(|a| !a.is_annotation()) {
        let role = match axis.kind() {
            Some(AxisKind::CoupledCurve) => Role::Curve,
            _ => Role::Swept,
        };
        pins.push((axis.touches(), 1, role));
        if let Some(r) = axis.residual() {
            pins.push((axis.touches(), r.pins, Role::Fixed));
        }
    }
    pins.extend(d.slice.iter().map(|f| (f.touches(), f.dof(), Role::Fixed)));
    pins.extend(
        d.conventions
            .iter()
            .map(|c| (c.block.only(), c.pins, Role::Fixed)),
    );
    pins
}

/// The groups of blocks the pins span: each block alone, merged where one pin spans several.
fn groups(pins: &[(BlockTouch, u32, Role)]) -> Vec<BlockTouch> {
    let mut groups: Vec<BlockTouch> = Block::ORDER.iter().map(|b| b.only()).collect();
    for &(touches, _, _) in pins.iter().filter(|p| !p.0.is_empty()) {
        let (meeting, mut rest): (Vec<_>, Vec<_>) =
            groups.into_iter().partition(|&g| meets(g, touches));
        rest.push(meeting.into_iter().fold(no_blocks(), union));
        groups = rest;
    }
    groups.sort_by_key(|&g| blocks(g).next().map(Block::position));
    groups
}

/// Every group pinned exactly once (the module's note). In a group with a kind-4 axis, each curve's embed pins every DOF
/// of the blocks it ties, so two curves on overlapping blocks double-write them; a kind-1 or kind-2 axis inside a
/// curve's blocks collides with it and pins nothing more.
fn count(d: &ChartDescriptor) -> Result<(), Refusal> {
    let pins = pins(d);
    for group in groups(&pins) {
        let inside: Vec<_> = pins.iter().filter(|p| meets(p.0, group)).collect();
        let ties = inside
            .iter()
            .filter(|p| p.2 == Role::Curve)
            .fold(no_blocks(), |set, p| union(set, p.0));
        let total = dof(group);
        let pinned: u32 = inside
            .iter()
            .map(|p| match p.2 {
                Role::Curve => dof(p.0),
                Role::Swept if within(p.0, ties) => 0,
                Role::Swept | Role::Fixed => p.1,
            })
            .sum();
        if pinned < total {
            return Err(Refusal::UnderDetermined {
                blocks: group,
                pinned,
                dof: total,
            });
        }
        if pinned > total {
            return Err(Refusal::DoubleWritten {
                blocks: group,
                pinned,
                dof: total,
            });
        }
    }
    Ok(())
}

/// `|x|` as `(m, e)` with `|x| = m·2^e` and `m` odd, or `None` at zero: the exact magnitude of a finite float.
fn exact(x: f64) -> Option<(u128, i32)> {
    if x == 0.0 {
        return None;
    }
    let bits = x.to_bits();
    let biased = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1 << 52) - 1);
    let (m, e) = if biased == 0 {
        (fraction, -1074)
    } else {
        (fraction + (1 << 52), biased - 1075)
    };
    let shift = m.trailing_zeros();
    Some((u128::from(m >> shift), e + shift as i32))
}

/// Whether `a·b = c·d` exactly, for finite floats: the same sign, and the same odd mantissa and exponent.
fn products_equal(a: f64, b: f64, c: f64, d: f64) -> bool {
    let product = |x: f64, y: f64| {
        let ((mx, ex), (my, ey)) = (exact(x)?, exact(y)?);
        Some((
            x.is_sign_negative() ^ y.is_sign_negative(),
            mx * my,
            ex + ey,
        ))
    };
    product(a, b) == product(c, d)
}

/// The latent directions written, linearly independent, exactly (the module's note): the coordinates the slice holds
/// and the axes' directions. A direction along one coordinate is that coordinate; a coordinate written twice, a
/// direction inside the span of the coordinates written, and two directions parallel there are each refused.
fn directions(d: &ChartDescriptor) -> Result<(), Refusal> {
    let mut written = [false; LATENT_DIM];
    let mut write = |k: usize, by: &str| {
        if written[k] {
            return Err(Refusal::DependentDirections {
                what: format!("z[{k}] by {by}"),
            });
        }
        written[k] = true;
        Ok(())
    };
    for f in &d.slice {
        if let Frozen::Coordinate(c) = f {
            write(*c as usize, "the slice")?;
        }
    }
    let mut general: Vec<(&'static str, Latent)> = Vec::new();
    for axis in &d.axes {
        let Some(q) = axis.direction() else { continue };
        if !q.iter().all(|x| x.is_finite()) {
            return Err(Refusal::NonFiniteDirection { axis: axis.name() });
        }
        let support: Vec<usize> = (0..LATENT_DIM).filter(|&k| q[k] != 0.0).collect();
        match support[..] {
            [k] => write(k, axis.name())?,
            _ => general.push((axis.name(), q)),
        }
    }
    let projected: Vec<(&str, Latent)> = general
        .into_iter()
        .map(|(name, q)| {
            (
                name,
                std::array::from_fn(|k| if written[k] { 0.0 } else { q[k] }),
            )
        })
        .collect();
    for (name, q) in &projected {
        if q.iter().all(|&x| x == 0.0) {
            return Err(Refusal::DependentDirections {
                what: format!("{name}, inside the coordinates written"),
            });
        }
    }
    if let [(a, x), (b, y)] = &projected[..] {
        let parallel = (0..LATENT_DIM)
            .all(|i| (0..LATENT_DIM).all(|j| products_equal(x[i], y[j], x[j], y[i])));
        if parallel {
            return Err(Refusal::DependentDirections {
                what: format!("{a} and {b}, parallel"),
            });
        }
    }
    Ok(())
}

/// The coupling the axes imply: the blocks the curves tie, and there the blocks both swept axes write.
fn implied_coupling(d: &ChartDescriptor) -> Coupling {
    let ties = d
        .axes
        .iter()
        .filter(|a| a.kind() == Some(AxisKind::CoupledCurve))
        .fold(no_blocks(), |set, a| union(set, a.touches()));
    let [h, v] = d.axes.map(|a| a.touches());
    let both = BlockTouch {
        config: h.config & v.config,
        momentum: h.momentum & v.momentum,
        mass: h.mass & v.mass,
    };
    let collisions = if ties.is_empty() { no_blocks() } else { both };
    Coupling { ties, collisions }
}

/// The declared flags agree with the axes.
fn flags(d: &ChartDescriptor) -> Result<(), Refusal> {
    let mismatch = |flag, declared: &dyn fmt::Debug, implied: &dyn fmt::Debug| Refusal::Flag {
        flag,
        declared: format!("{declared:?}"),
        implied: format!("{implied:?}"),
    };
    let annotation = d.axes.iter().any(|a| a.is_annotation());
    if d.annotation != annotation {
        return Err(mismatch("annotation", &d.annotation, &annotation));
    }
    let feasibility = d.axes.iter().any(|a| a.invariant_meta().is_some());
    if d.has_feasibility_boundary != feasibility {
        return Err(mismatch(
            "has_feasibility_boundary",
            &d.has_feasibility_boundary,
            &feasibility,
        ));
    }
    let coupling = implied_coupling(d);
    if d.coupling != coupling {
        return Err(mismatch("coupling", &d.coupling, &coupling));
    }
    Ok(())
}

/// The invariant solves, by sector in the decoder's order.
fn solves(d: &ChartDescriptor) -> Vec<Solve> {
    Block::ORDER
        .iter()
        .filter_map(|&sector| {
            let axes: Vec<Axis> = [Axis::Horizontal, Axis::Vertical]
                .into_iter()
                .filter(|&a| {
                    d.axis(a)
                        .invariant_meta()
                        .is_some_and(|i| i.sector() == sector)
                })
                .collect();
            (!axes.is_empty()).then_some(Solve {
                sector,
                joint: axes.len() > 1,
                axes,
            })
        })
        .collect()
}

// ── The registry's charts: the pair table and the spec validator (design § 6.1; TASK-M2-66) ──────────────────────

/// The pair table (§ 6.1, § 5.5): every pair of registered rows that is rank 1 over the coordinates the two axes
/// write, the other blocks frozen by the slice, so one quantity: refused as `SameQuantity`. `pair_table_complete`
/// estimates every registered pair's rank on a grid and holds this table to it in both directions. Each entry says why.
pub const SAME_QUANTITY: [(&str, &str, &str); 19] = [
    ("z_α", "α", "α is the α link of z_α"),
    (
        "z_α",
        "‖λ‖/‖ρ‖",
        "‖λ‖/‖ρ‖ = tan α·√(μ_ρ/μ_λ) at the frozen masses",
    ),
    ("z_α", "φ", "φ = 2α in the shape sphere's home frame"),
    (
        "α",
        "‖λ‖/‖ρ‖",
        "‖λ‖/‖ρ‖ = tan α·√(μ_ρ/μ_λ) at the frozen masses",
    ),
    ("α", "φ", "φ = 2α in the shape sphere's home frame"),
    ("‖λ‖/‖ρ‖", "φ", "both functions of α at the frozen masses"),
    ("z_β", "β", "β is the β link of z_β"),
    ("z_β", "θ", "θ = β in the shape sphere's home frame"),
    ("β", "θ", "θ = β in the shape sphere's home frame"),
    ("m₁/m₀", "m₀/m₁", "one the reciprocal of the other"),
    (
        "z_μ1",
        "m₁/m₀",
        "the mass link gives ln(m₁/m₀) = μ_max·tanh z_μ1",
    ),
    (
        "z_μ1",
        "m₀/m₁",
        "the mass link gives ln(m₁/m₀) = μ_max·tanh z_μ1",
    ),
    (
        "z_μ2",
        "m₂/m₀",
        "the mass link gives ln(m₂/m₀) = μ_max·tanh z_μ2",
    ),
    ("E", "K", "E = K + U, U frozen with the configuration"),
    ("E", "KE", "E = KE + U, U frozen with the configuration"),
    (
        "E",
        "Q",
        "Q = 2(E − U)/|U|, U frozen with the configuration",
    ),
    ("K", "KE", "one quantity, the kinetic energy"),
    ("K", "Q", "Q = 2K/|U|, U frozen with the configuration"),
    ("KE", "Q", "Q = 2·KE/|U|, U frozen with the configuration"),
];

/// Whether the pair table refuses `(a, b)`, in either order, or `a` is `b`.
pub fn same_quantity(a: &str, b: &str) -> bool {
    a == b
        || SAME_QUANTITY
            .iter()
            .any(|&(x, y, _)| (x == a && y == b) || (x == b && y == a))
}

/// The pair table without `(K, Q)`, for `pair_table_complete`'s control.
#[cfg(feature = "controls")]
pub fn same_quantity_without_k_q(a: &str, b: &str) -> bool {
    same_quantity(a, b) && !matches!((a, b), ("K", "Q") | ("Q", "K"))
}

/// R-849's general numerical solve, as a typed combination with no registered inverse reports it (§ 3, § 7.3;
/// RQ-816 (a), point (i)): the coordinates it solves for, its branch rule, its seams and its convergence per pixel.
/// TASK-M2-73 builds the solve; this is what it is held to, and it is never a registered numerical inverse.
#[derive(Clone, Debug, PartialEq)]
pub struct GeneralSolve {
    /// The combination.
    pub expression: Expr,
    /// The sector it solves into: the most downstream block its parts write, in the decoder's order.
    pub sector: Block,
    /// The latent coordinates it solves for, the sector's.
    pub solves_for: Vec<usize>,
    /// Its branch rule, a pure function of the pixel (§ 7.3).
    pub branch: &'static str,
    /// Where its seams are (§ 7.3).
    pub seams: &'static str,
    /// How it reports convergence.
    pub convergence: Convergence,
}

/// How a solve reports convergence (REQ-CHART-145).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Convergence {
    /// Per pixel: each sample's iterations and residual, a failure tagged `decode_failed` and never dropped.
    PerPixel,
}

/// The general solve's branch rule (§ 7.3).
pub const GENERAL_BRANCH: &str = "Newton in f64 from a fixed list of starts, the pixel's own coordinates the only \
     input; the first start that converges gives the root, and within a fixed δ of a tie, the root nearest the slice \
     centre";

/// The general solve's seams (§ 7.3).
pub const GENERAL_SEAMS: &str =
    "where the solve's Jacobian changes sign, or where the branch differs across a \
     pixel edge";

/// How an axis of a validated spec inverts.
#[derive(Clone, Debug, PartialEq)]
pub enum AxisInverse {
    /// A latent direction, assigned (kind 1).
    Latent(Latent),
    /// A registered row, with its registered inverse.
    Registered {
        /// The row.
        row: &'static str,
        /// Its inverse, closed or numerical.
        inverse: Inverse,
    },
    /// A typed combination with no registered inverse: the general solve.
    General(GeneralSolve),
}

/// A [`ChartSpec`] that passed [`validate_spec`].
#[derive(Clone, Debug)]
pub struct ValidatedSpec {
    spec: ChartSpec,
    axes: [AxisInverse; 2],
    rows: [Option<Row>; 2],
}

impl ValidatedSpec {
    /// The spec.
    pub fn spec(&self) -> &ChartSpec {
        &self.spec
    }

    /// How each axis inverts, horizontal then vertical.
    pub fn axes(&self) -> &[AxisInverse; 2] {
        &self.axes
    }

    /// Each axis's registered row, where it has one.
    pub fn rows(&self) -> &[Option<Row>; 2] {
        &self.rows
    }
}

/// An axis's name, as a refusal names it.
pub fn axis_name(axis: &AxisSpec) -> String {
    match &axis.primitive {
        Primitive::Direction(q) => match single(q) {
            Some(k) => LATENT_ROWS[k].to_owned(),
            None => "a latent direction".to_owned(),
        },
        Primitive::Pick(name) => name.clone(),
        Primitive::Typed(e) => e.to_string(),
    }
}

/// The one coordinate `q` lies along, if it lies along one.
fn single(q: &Latent) -> Option<usize> {
    let support: Vec<usize> = (0..LATENT_DIM).filter(|&k| q[k] != 0.0).collect();
    match support[..] {
        [k] => Some(k),
        _ => None,
    }
}

/// The single registered quantity an axis is a function of, for the pair table: a picked row, a direction along one
/// coordinate (that coordinate's row), or a typed combination whose parts the table makes one quantity.
fn quantity_of(axis: &AxisSpec) -> Option<String> {
    match &axis.primitive {
        Primitive::Direction(q) => single(q).map(|k| LATENT_ROWS[k].to_owned()),
        Primitive::Pick(name) => Some(name.clone()),
        Primitive::Typed(e) => {
            let parts = e.parts();
            let first = parts.first()?.clone();
            parts
                .iter()
                .all(|p| same_quantity(&first, p))
                .then_some(first)
        }
    }
}

/// The holds on in `c` that fix `row`.
fn held(c: &Constraints, row: &Row) -> Option<Hold> {
    row.fixed_by.filter(|&h| c.holds(h))
}

/// A registered row as an axis named `name`, beside `other`: refused as a functional, unclassified, size-only, with no
/// warp written, with no inverse (`NoJointInverse`, RQ-816 (a) point (iii)) or held by a constraint, in that order; a
/// latent coordinate's row is its unit direction.
fn picked(
    row: Row,
    name: String,
    other: &str,
    c: &Constraints,
) -> Result<(AxisInverse, Option<Row>), Refusal> {
    if row.class == Class::Functional {
        return Err(Refusal::Functional { axis: name });
    }
    if row.class == Class::Unclassified {
        return Err(Refusal::Unclassified { axis: name });
    }
    if row.scale == Scale::SizeOnly {
        return Err(Refusal::SizeOnly { axis: name });
    }
    if !row.warp_written {
        return Err(Refusal::NoWarpWritten { axis: row.name });
    }
    if !row.inverse.exists() {
        return Err(Refusal::NoJointInverse {
            a: name,
            b: other.to_owned(),
        });
    }
    if let Some(h) = held(c, &row) {
        return Err(Refusal::AxisAndHold {
            axis: name,
            quantity: h.name(),
        });
    }
    // A latent coordinate's row is that coordinate's unit direction, assigned (kind 1), however it was entered, so a
    // pair of them lowers uniform as the latent chart does (§ 7).
    if row.kind == Some(AxisKind::RawLatent) {
        if let Some(k) = LATENT_ROWS.iter().position(|&n| n == row.name) {
            let mut q = [0.0; LATENT_DIM];
            q[k] = 1.0;
            return Ok((AxisInverse::Latent(q), Some(row)));
        }
    }
    Ok((
        AxisInverse::Registered {
            row: row.name,
            inverse: row.inverse,
        },
        Some(row),
    ))
}

/// One axis classed (the order is § 6.1's list: a functional, size-only, no warp written, no joint inverse, axis and
/// hold): a registered row picked, a typed combination (RQ-816 (a): the `SizeOnly`, `NoWarpWritten`, `AxisAndHold`
/// and `SameQuantity` refusals still apply, and with no registered inverse it takes the general solve, a registered
/// `none` row typed as an expression included), or a latent direction.
fn classify(
    registry: &Registry,
    c: &Constraints,
    axis: &AxisSpec,
    other: &str,
    rules: Rules,
) -> Result<(AxisInverse, Option<Row>), Refusal> {
    let name = axis_name(axis);
    let lookup = |part: &str| {
        registry
            .row(part)
            .copied()
            .ok_or_else(|| Refusal::UnknownPrimitive {
                axis: part.to_owned(),
            })
    };
    match &axis.primitive {
        Primitive::Direction(q) => {
            if !q.iter().all(|x| x.is_finite()) {
                return Err(Refusal::NonFiniteDirection { axis: "q" });
            }
            if q.iter().all(|&x| x == 0.0) {
                return Err(Refusal::DependentDirections {
                    what: "a zero latent direction".to_owned(),
                });
            }
            let row = single(q).and_then(|k| registry.row(LATENT_ROWS[k]).copied());
            Ok((AxisInverse::Latent(*q), row))
        }
        Primitive::Pick(pick) => picked(lookup(pick)?, name, other, c),
        Primitive::Typed(e) => {
            let parts = e
                .parts()
                .iter()
                .map(|p| lookup(p))
                .collect::<Result<Vec<_>, _>>()?;
            // A combination matching a registered row's own inverse is that row (§ 3: "`closed` only with a
            // registered closed inverse, `numerical` only with a registered numerical one").
            let registered = registry.rows().iter().find(|r| {
                r.inverse.exists()
                    && (*e == Expr::row(r.name) || r.definition.is_some_and(|d| d() == *e))
            });
            if let Some(row) = registered {
                return picked(*row, name, other, c);
            }
            if parts.iter().any(|r| r.class == Class::Functional) {
                return Err(Refusal::Functional { axis: name });
            }
            if rules.forward_check {
                if parts.iter().any(|r| r.class == Class::Unclassified) {
                    return Err(Refusal::Unclassified { axis: name });
                }
                if let Some(r) = parts.iter().find(|r| r.forward.is_none()) {
                    return Err(Refusal::NoForward {
                        axis: name,
                        part: r.name,
                    });
                }
            }
            // Its scale is computed, not declared: a combination of size-only rows and constants is constant.
            if parts.iter().all(|r| r.scale == Scale::SizeOnly) {
                return Err(Refusal::SizeOnly { axis: name });
            }
            if let Some(r) = parts.iter().find(|r| !r.warp_written) {
                return Err(Refusal::NoWarpWritten { axis: r.name });
            }
            // The plan is unclear here; chose to refuse a combination every part of which a hold fixes, because it
            // is then that hold's quantity, as a single row is; one with a free part varies through it.
            let holds: Vec<Option<Hold>> = parts.iter().map(|r| held(c, r)).collect();
            if let Some(Some(h)) = holds.first() {
                if holds.iter().all(|x| x.is_some()) {
                    return Err(Refusal::AxisAndHold {
                        axis: name,
                        quantity: h.name(),
                    });
                }
            }
            Ok((AxisInverse::General(general(e, &parts)), None))
        }
    }
}

/// R-849's general solve of `e`, whose parts are `parts`: it solves into the most downstream block they write.
fn general(e: &Expr, parts: &[Row]) -> GeneralSolve {
    let sector = Block::ORDER
        .into_iter()
        .rev()
        .find(|b| parts.iter().any(|r| b.in_set(r.writes)))
        .unwrap_or(Block::Momentum);
    let solves_for = COORDINATES
        .iter()
        .filter(|&&k| Block::of(k) == sector)
        .map(|&k| k as usize)
        .collect();
    GeneralSolve {
        expression: e.clone(),
        sector,
        solves_for,
        branch: GENERAL_BRANCH,
        seams: GENERAL_SEAMS,
        convergence: Convergence::PerPixel,
    }
}

/// The validation pass on a registry chart (§ 6.1): each axis classed against `registry` and the constraints, then the
/// pair against the pair table; the [`ValidatedSpec`] where it passes, else the first [`Refusal`]. Kernel charts keep
/// [`validate`], their descriptors' check.
pub fn validate_spec(registry: &Registry, spec: &ChartSpec) -> Result<ValidatedSpec, Refusal> {
    validate_spec_with(registry, spec, same_quantity)
}

/// The rules [`validate_spec`] applies, which its controls swap out.
#[derive(Clone, Copy, Debug)]
pub struct Rules {
    /// The pair table.
    pub table: fn(&str, &str) -> bool,
    /// Whether a typed combination's parts must each be classified and have a forward.
    pub forward_check: bool,
    /// Whether a row closed only on its chart's pair is refused `NoJointInverse` off it (the control's old rule).
    pub pair_only_refused: bool,
}

impl Rules {
    /// The rules as built.
    pub const BUILT: Rules = Rules {
        table: same_quantity,
        forward_check: true,
        pair_only_refused: false,
    };
}

/// [`validate_spec`] under the pair table `table` (the controls swap it).
pub fn validate_spec_with(
    registry: &Registry,
    spec: &ChartSpec,
    table: fn(&str, &str) -> bool,
) -> Result<ValidatedSpec, Refusal> {
    validate_spec_under(
        registry,
        spec,
        Rules {
            table,
            ..Rules::BUILT
        },
    )
}

/// [`validate_spec`] under `rules` (the controls swap them).
pub fn validate_spec_under(
    registry: &Registry,
    spec: &ChartSpec,
    rules: Rules,
) -> Result<ValidatedSpec, Refusal> {
    let table = rules.table;
    let names = spec.axes.clone().map(|a| axis_name(&a));
    let (h, hr) = classify(registry, &spec.constraints, &spec.axes[0], &names[1], rules)?;
    let (v, vr) = classify(registry, &spec.constraints, &spec.axes[1], &names[0], rules)?;
    if let (AxisInverse::Latent(x), AxisInverse::Latent(y)) = (&h, &v) {
        let parallel = (0..LATENT_DIM)
            .all(|i| (0..LATENT_DIM).all(|j| products_equal(x[i], y[j], x[j], y[i])));
        if parallel {
            return Err(Refusal::DependentDirections {
                what: format!("{} and {}, parallel", names[0], names[1]),
            });
        }
    }
    if let (Some(a), Some(b)) = (quantity_of(&spec.axes[0]), quantity_of(&spec.axes[1])) {
        if table(&a, &b) {
            let [a, b] = names;
            return Err(Refusal::SameQuantity { a, b });
        }
    }
    // A row closed only on its chart's pair has its closed inverse on that pair alone (§ 3: "a closed inverse may need
    // a partner: it is then registered on the pair"). Off it there is no contradiction, so no refusal of § 6.1: a row
    // with a forward of its own (`θ`, `φ`) takes the general solve on the CPU, as a combination with no registered
    // inverse does (R-849, R-925); one with none (its forward is its chart's encode) has nothing a solve could read,
    // and is refused for that, by name. Transposed, a registered pair is still closed: it lowers to the CPU.
    let mut axes = [h, v];
    let rows = [hr, vr];
    let on_pair = pair(&names[0], &names[1]).is_some() || pair(&names[1], &names[0]).is_some();
    for i in 0..2 {
        let Some(r) = rows[i] else { continue };
        let Inverse::Closed {
            on_pair: Some(chart),
        } = r.inverse
        else {
            continue;
        };
        if on_pair {
            continue;
        }
        if rules.pair_only_refused {
            return Err(Refusal::NoJointInverse {
                a: r.name.to_owned(),
                b: names[1 - i].clone(),
            });
        }
        if r.forward.is_none() {
            return Err(Refusal::PairOnly {
                axis: r.name,
                chart,
            });
        }
        axes[i] = AxisInverse::General(general(&Expr::row(r.name), &[r]));
    }
    Ok(ValidatedSpec {
        spec: spec.clone(),
        axes,
        rows,
    })
}
