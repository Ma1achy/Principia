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
//! coupled and solve jointly ([`Solve`]), as `(L_z, E)` does.
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

use kernel::chart::{family_label, swept, Axis, BlockTouch};
use kernel::decode::Decoded;
use kernel::generated::links::Selection;

use super::axis::{blocks, dof, meets, names, no_blocks, union, within, AxisKind, Block};
use super::descriptor::{Authored, ChartDescriptor, ChartKind, Coupling, Frozen};
use crate::chart_validate::{self, validate_view};
use crate::contract::sim_config::{Latent, LATENT_DIM};

/// Why the validator refuses a chart.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// The view's energy target, by the chart's `forbids_energy_normalisation` (R-25).
    View(chart_validate::Refusal),
    /// Neither axis is swept.
    NoSweptAxis,
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
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::View(r) => write!(f, "{r}"),
            Refusal::NoSweptAxis => f.write_str("refused: neither axis is swept"),
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
        self.chart.decode::<S, f64>(u, v)
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
