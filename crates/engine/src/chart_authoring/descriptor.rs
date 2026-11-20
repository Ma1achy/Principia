//! The per-chart descriptors (chart_decoder_contract Part 5, "Per-chart descriptors"; REQ-CHART-014): what a chart
//! declares beside its two axes — its slice, its conventions, `system_image`, `has_feasibility_boundary`, `coupling`
//! and the annotation flag — and the [`Authored`] trait every chart the validator takes implements.

use kernel::chart::mixed::Coordinate;
use kernel::chart::{Axis, BlockTouch, Chart, SystemImage};

use super::axis::{no_blocks, AxisDescriptor, AxisKind, Block};

/// One DOF set the slice holds fixed (Part 5: "swept axes + conventions + slice pin all 8 DOF").
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Frozen {
    /// The coordinate `z_k`, held at `z₀`'s value: one DOF of its block.
    Coordinate(Coordinate),
    /// A quantity that is not a latent coordinate, held at a value (`L_z = 0` on the bifurcation strip): `dof` DOF of
    /// `block`.
    Held {
        /// The held quantity, in words.
        quantity: &'static str,
        /// Its block.
        block: Block,
        /// The DOF it pins.
        dof: u32,
    },
    /// The latent plane's orthogonal complement, held at `z₀` (chart_decoder_contract Part 3, the affine slice): `dof`
    /// DOF across `touches`.
    Complement {
        /// The blocks the complement spans.
        touches: BlockTouch,
        /// Its dimension.
        dof: u32,
    },
}

impl Frozen {
    /// The blocks the held set lies in.
    pub fn touches(&self) -> BlockTouch {
        match *self {
            Frozen::Coordinate(c) => c.touches(),
            Frozen::Held { block, .. } => block.only(),
            Frozen::Complement { touches, .. } => touches,
        }
    }

    /// The DOF it pins.
    pub fn dof(&self) -> u32 {
        match *self {
            Frozen::Coordinate(_) => 1,
            Frozen::Held { dof, .. } | Frozen::Complement { dof, .. } => dof,
        }
    }
}

/// A chart-level convention pinning `pins` DOF of `block` (Part 5's "conventions"): the rest start's zero momenta, or
/// the invariant construction's deterministic seed (chart_reference §2.2, R-82).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Convention {
    /// The block.
    pub block: Block,
    /// The DOF it pins.
    pub pins: u32,
    /// The convention, in words.
    pub statement: &'static str,
}

/// The `coupling` descriptor (Part 5): the blocks a kind-4 axis ties, and the same-block axis collisions there, the
/// blocks both axes write on a chart with a curve. Both are empty on a chart with no kind-4 axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Coupling {
    /// The blocks the chart's curve ties.
    pub ties: BlockTouch,
    /// The blocks both axes write, flagged.
    pub collisions: BlockTouch,
}

impl Coupling {
    /// No coupling: a chart with no kind-4 axis.
    pub fn none() -> Self {
        Coupling {
            ties: no_blocks(),
            collisions: no_blocks(),
        }
    }
}

/// Whether a chart is the affine slice or needs the general `Φ` (chart_decoder_contract Part 3; render_gui_spec §G7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChartKind {
    /// Every swept axis is a latent direction: `z(s, t) = z₀ + (2s−1)q₁ + (2t−1)q₂`.
    Affine,
    /// A swept axis is a physical quantity or a curve: pixels map through `Φ`, then the decoder.
    Nonlinear,
}

/// A chart's declarations (chart_decoder_contract Part 5): its axes `[horizontal, vertical]`, its slice and
/// conventions, and the per-chart descriptors. The validator checks the declarations against the axes.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartDescriptor {
    /// The axes, horizontal then vertical.
    pub axes: [AxisDescriptor; 2],
    /// The DOF the slice holds.
    pub slice: Vec<Frozen>,
    /// The chart-level conventions.
    pub conventions: Vec<Convention>,
    /// How the address map folds onto distinct systems.
    pub system_image: SystemImage,
    /// Whether the chart has a feasibility boundary, its infeasible pixels tagged, never dropped: invariant charts.
    pub has_feasibility_boundary: bool,
    /// The coupling.
    pub coupling: Coupling,
    /// Whether the chart has an annotation axis: a 1-swept family rendered in 2D.
    pub annotation: bool,
}

impl ChartDescriptor {
    /// The axis `axis`.
    pub fn axis(&self, axis: Axis) -> &AxisDescriptor {
        match axis {
            Axis::Horizontal => &self.axes[0],
            Axis::Vertical => &self.axes[1],
        }
    }

    /// The chart's kind (render_gui_spec §G7: "a physical-quantity axis makes the chart nonlinear"): affine where every
    /// swept axis is a latent direction, else nonlinear.
    pub fn kind(&self) -> ChartKind {
        let affine = self
            .axes
            .iter()
            .all(|a| a.is_annotation() || a.kind() == Some(AxisKind::RawLatent));
        if affine {
            ChartKind::Affine
        } else {
            ChartKind::Nonlinear
        }
    }
}

/// A chart the authoring layer describes: every chart the validator takes declares its descriptors.
pub trait Authored: Chart {
    /// The chart's declarations.
    fn descriptor(&self) -> ChartDescriptor;
}
