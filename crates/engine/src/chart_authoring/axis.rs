//! The axis metadata (chart_decoder_contract § "The four axis kinds — a closed set", Part 5's "Per-axis metadata the
//! validator needs"; REQ-CHART-004, REQ-CHART-012): the decoder's three blocks, the closed set of four axis kinds, and
//! the axis descriptor carrying every field Part 5 lists.

use kernel::chart::mixed::Coordinate;
use kernel::chart::{AxisMeta, BlockTouch};

use crate::contract::sim_config::{Latent, LATENT_DIM};

/// The eight latent coordinates in the decoder's block layout (chart_decoder_contract Part 2): `z[0:2]` configuration,
/// `z[2:6]` momentum, `z[6:8]` mass. A coordinate's index is its discriminant.
pub const COORDINATES: [Coordinate; LATENT_DIM] = [
    Coordinate::ZAlpha,
    Coordinate::ZBeta,
    Coordinate::ZQ0,
    Coordinate::ZQ1,
    Coordinate::ZQ2,
    Coordinate::ZQ3,
    Coordinate::ZMu1,
    Coordinate::ZMu2,
];

/// The unit vector `ê_k` of the coordinate `k`.
pub fn unit(coordinate: Coordinate) -> Latent {
    let mut e = [0.0; LATENT_DIM];
    e[coordinate as usize] = 1.0;
    e
}

/// One of the decoder's three blocks (chart_decoder_contract design axiom 2: `z⁸ → mass × config × momentum`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Block {
    /// The mass block.
    Mass,
    /// The configuration block.
    Config,
    /// The momentum block.
    Momentum,
}

impl Block {
    /// The blocks in the decoder's order, mass → config → momentum (design axiom 2): a block is upstream of every block
    /// after it, which reads it as fixed (§ "The four axis kinds", kind 3).
    pub const ORDER: [Block; 3] = [Block::Mass, Block::Config, Block::Momentum];

    /// The block's position in [`Block::ORDER`].
    pub fn position(self) -> usize {
        match self {
            Block::Mass => 0,
            Block::Config => 1,
            Block::Momentum => 2,
        }
    }

    /// The block the coordinate is in.
    pub fn of(coordinate: Coordinate) -> Block {
        let t = coordinate.touches();
        if t.mass {
            Block::Mass
        } else if t.config {
            Block::Config
        } else {
            Block::Momentum
        }
    }

    /// The block's degrees of freedom: the latent coordinates it holds (2, 2 and 4; 8 in all).
    pub fn dof(self) -> u32 {
        COORDINATES
            .iter()
            .filter(|&&c| Block::of(c) == self)
            .count() as u32
    }

    /// The set holding this block alone.
    pub fn only(self) -> BlockTouch {
        BlockTouch {
            config: self == Block::Config,
            momentum: self == Block::Momentum,
            mass: self == Block::Mass,
        }
    }

    /// Whether `set` holds this block.
    pub fn in_set(self, set: BlockTouch) -> bool {
        match self {
            Block::Mass => set.mass,
            Block::Config => set.config,
            Block::Momentum => set.momentum,
        }
    }

    /// The block's name.
    pub fn name(self) -> &'static str {
        match self {
            Block::Mass => "mass",
            Block::Config => "config",
            Block::Momentum => "momentum",
        }
    }
}

/// The union of two block sets.
pub fn union(a: BlockTouch, b: BlockTouch) -> BlockTouch {
    BlockTouch {
        config: a.config | b.config,
        momentum: a.momentum | b.momentum,
        mass: a.mass | b.mass,
    }
}

/// Whether two block sets share a block.
pub fn meets(a: BlockTouch, b: BlockTouch) -> bool {
    (a.config & b.config) | (a.momentum & b.momentum) | (a.mass & b.mass)
}

/// Whether every block of `a` is in `b`.
pub fn within(a: BlockTouch, b: BlockTouch) -> bool {
    (!a.config | b.config) & (!a.momentum | b.momentum) & (!a.mass | b.mass)
}

/// The blocks a set holds, in [`Block::ORDER`].
pub fn blocks(set: BlockTouch) -> impl Iterator<Item = Block> {
    Block::ORDER.into_iter().filter(move |b| b.in_set(set))
}

/// The degrees of freedom of the blocks a set holds.
pub fn dof(set: BlockTouch) -> u32 {
    blocks(set).map(Block::dof).sum()
}

/// The set's blocks by name, `{mass, config}`.
pub fn names(set: BlockTouch) -> String {
    let names: Vec<_> = blocks(set).map(Block::name).collect();
    format!("{{{}}}", names.join(", "))
}

/// The four axis kinds, a closed set (chart_decoder_contract § "The four axis kinds — a closed set"): no fifth. They
/// are a CPU authoring layer, never interpreted by the kernel, where each chart is a type parameter (design axiom 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisKind {
    /// Kind 1: a single latent direction, assigned directly.
    RawLatent,
    /// Kind 2: a scalar inside one block, inverted into it; the block's leftover DOF take a residual convention.
    DerivedInBlock,
    /// Kind 3: `E`, `L_z`, `K`, `KE` or `PE`, solved into a downstream sector, the upstream sectors read as fixed.
    CrossBlockInvariant,
    /// Kind 4: one scalar slaving several blocks through a fixed nonlinear embed.
    CoupledCurve,
}

/// A cross-block invariant quantity (kind 3's list).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvariantQuantity {
    /// The energy `E`.
    E,
    /// The angular momentum `L_z`.
    Lz,
    /// The invariant charts' kinetic energy `K`.
    K,
    /// The kinetic energy `KE`.
    KE,
    /// The potential energy `PE`.
    PE,
}

impl InvariantQuantity {
    /// Whether the quantity is a constant of motion (Part 5, `conserved_along_flow`): `E` and `L_z` are; `K` is
    /// invariant-constructed but not conserved, a trace oscillating as `KE ↔ PE` exchange, and `KE` and `PE` are that
    /// exchange's two sides.
    pub fn conserved_along_flow(self) -> bool {
        matches!(self, InvariantQuantity::E | InvariantQuantity::Lz)
    }
}

/// Part 5's "invariant?" metadata: the sector the axis solves into, its dependency set and whether it is
/// `conserved_along_flow`, which the hover trace and any along-trajectory rendering read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvariantMeta {
    quantity: InvariantQuantity,
    sector: Block,
    depends_on: BlockTouch,
    conserved_along_flow: bool,
}

impl InvariantMeta {
    /// `quantity` solved into `sector`, reading `depends_on`; `conserved_along_flow` is the quantity's own. Whether the
    /// sector is downstream of every dependency is the validator's check, not the constructor's.
    pub fn new(quantity: InvariantQuantity, sector: Block, depends_on: BlockTouch) -> Self {
        InvariantMeta {
            quantity,
            sector,
            depends_on,
            conserved_along_flow: quantity.conserved_along_flow(),
        }
    }

    /// The quantity.
    pub fn quantity(&self) -> InvariantQuantity {
        self.quantity
    }

    /// The sector the axis solves into.
    pub fn sector(&self) -> Block {
        self.sector
    }

    /// The blocks the axis reads.
    pub fn depends_on(&self) -> BlockTouch {
        self.depends_on
    }

    /// Whether the quantity is a constant of motion.
    pub fn conserved_along_flow(&self) -> bool {
        self.conserved_along_flow
    }
}

/// A residual convention (Part 3, "Residual conventions (kind 2)"): how the leftover DOF of a block an axis
/// under-determines are pinned, labelled. `pins` is how many it pins itself; 0 where the chart's other axis or the
/// slice pins them, which the statement says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Residual {
    /// The DOF the convention pins.
    pub pins: u32,
    /// The convention, in words: the label the chart carries.
    pub statement: &'static str,
}

/// A point of a kind-4 axis's embed (chart_decoder_contract § "The four axis kinds", row 4: "`ν → (α,β,μ₁,μ₂)`"): the
/// configuration angles `α`, `β` of chart_reference §0.2's canonical-frame decode and the mass logits `μ₁`, `μ₂` of
/// §0.1, `(m₀, m₁, m₂) = softmax(0, μ₁, μ₂)`. These are the decoder's quantities ahead of its links, so the embed and
/// its tangent hold under any link selection; the pullback to the latent `z` through the selected links is the tilt's
/// (TASK-M2-23, REQ-CHART-007).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurvePoint {
    /// `α`, `‖ρ̃‖ = cos α` at `R̃ = 1`.
    pub alpha: f64,
    /// `β`, `λ̃`'s angle from `+x`.
    pub beta: f64,
    /// `μ₁ = ln(m₁/m₀)`.
    pub mu1: f64,
    /// `μ₂ = ln(m₂/m₀)`.
    pub mu2: f64,
}

/// Part 5's "curve?" metadata: a kind-4 axis's embed `γ` from its scalar to `(α, β, μ₁, μ₂)`, and the embed's tangent
/// `γ'` in the same coordinates, which tilt reads at a lock (Part 4).
#[derive(Clone, Copy, Debug)]
pub struct CurveMeta {
    /// The scalar the curve is parametrised by.
    pub parameter: &'static str,
    /// `γ`, the embed.
    pub embed: fn(f64) -> CurvePoint,
    /// `γ'`, its tangent.
    pub tangent: fn(f64) -> CurvePoint,
}

/// Two curves are one where they share their parameter: a function's address is no identity (two may share one, and
/// one may have two).
impl PartialEq for CurveMeta {
    fn eq(&self, other: &Self) -> bool {
        self.parameter == other.parameter
    }
}

/// One axis's metadata (chart_decoder_contract Part 5): its kind, block-touch set, annotation-only flag, invariant,
/// residual convention and curve. Built only by its constructors, one per kind, so the fields agree with the kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisDescriptor {
    name: &'static str,
    kind: Option<AxisKind>,
    touches: BlockTouch,
    annotation: bool,
    direction: Option<Latent>,
    invariant: Option<InvariantMeta>,
    residual: Option<Residual>,
    curve: Option<CurveMeta>,
}

/// The blocks a latent direction has a nonzero component in.
fn touched(direction: &Latent) -> BlockTouch {
    COORDINATES
        .iter()
        .filter(|&&c| direction[c as usize] != 0.0)
        .fold(no_blocks(), |set, &c| union(set, c.touches()))
}

/// The empty block set.
pub fn no_blocks() -> BlockTouch {
    BlockTouch {
        config: false,
        momentum: false,
        mass: false,
    }
}

impl AxisDescriptor {
    fn blank(name: &'static str, kind: Option<AxisKind>, touches: BlockTouch) -> Self {
        AxisDescriptor {
            name,
            kind,
            touches,
            annotation: false,
            direction: None,
            invariant: None,
            residual: None,
            curve: None,
        }
    }

    /// A kind-1 axis along the latent direction `direction`: it writes every block the direction has a component in.
    pub fn raw_latent(name: &'static str, direction: Latent) -> Self {
        AxisDescriptor {
            direction: Some(direction),
            ..Self::blank(name, Some(AxisKind::RawLatent), touched(&direction))
        }
    }

    /// A kind-1 axis on the single coordinate `z_k`.
    pub fn coordinate(name: &'static str, coordinate: Coordinate) -> Self {
        Self::raw_latent(name, unit(coordinate))
    }

    /// A kind-2 axis, a scalar inside `block`, with its residual convention. `control` is the one coordinate the axis
    /// sets through its link's inverse, where it sets one (a mixed-axis chart's derived quantity).
    pub fn derived(
        name: &'static str,
        block: Block,
        control: Option<Coordinate>,
        residual: Residual,
    ) -> Self {
        AxisDescriptor {
            direction: control.map(unit),
            residual: Some(residual),
            ..Self::blank(name, Some(AxisKind::DerivedInBlock), block.only())
        }
    }

    /// A kind-3 axis: it writes the sector it solves into.
    pub fn invariant(name: &'static str, invariant: InvariantMeta) -> Self {
        AxisDescriptor {
            invariant: Some(invariant),
            ..Self::blank(
                name,
                Some(AxisKind::CrossBlockInvariant),
                invariant.sector.only(),
            )
        }
    }

    /// A kind-4 axis tying the blocks `ties` through its embed.
    pub fn curve(name: &'static str, ties: BlockTouch, curve: CurveMeta) -> Self {
        AxisDescriptor {
            curve: Some(curve),
            ..Self::blank(name, Some(AxisKind::CoupledCurve), ties)
        }
    }

    /// An annotation-only axis: it drives no DOF and touches no block, a display and landmark coordinate the decoder
    /// ignores (Part 5). It pins nothing, so it has none of the four kinds, which say how an axis pins DOF.
    pub fn annotation(name: &'static str) -> Self {
        AxisDescriptor {
            annotation: true,
            ..Self::blank(name, None, no_blocks())
        }
    }

    /// The axis's name.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The axis's kind; `None` for an annotation axis.
    pub fn kind(&self) -> Option<AxisKind> {
        self.kind
    }

    /// The block-touch set: the blocks the axis writes, empty for an annotation axis.
    pub fn touches(&self) -> BlockTouch {
        self.touches
    }

    /// Whether the axis is annotation-only.
    pub fn is_annotation(&self) -> bool {
        self.annotation
    }

    /// The latent direction the axis writes, where it writes one: a kind-1 axis's, or a kind-2 axis's control `ê_k`.
    pub fn direction(&self) -> Option<Latent> {
        self.direction
    }

    /// The invariant metadata, for a kind-3 axis.
    pub fn invariant_meta(&self) -> Option<InvariantMeta> {
        self.invariant
    }

    /// The residual convention, for a kind-2 axis.
    pub fn residual(&self) -> Option<Residual> {
        self.residual
    }

    /// The curve metadata, for a kind-4 axis.
    pub fn curve_meta(&self) -> Option<CurveMeta> {
        self.curve
    }

    /// Whether the axis is a physical quantity (render_gui_spec §G7): a derived quantity or an invariant, which the
    /// decoder reaches only through `Φ`.
    pub fn is_physical_quantity(&self) -> bool {
        matches!(
            self.kind,
            Some(AxisKind::DerivedInBlock | AxisKind::CrossBlockInvariant)
        )
    }

    /// The kernel's view of the axis, its block-touch set and annotation flag (`kernel::chart::AxisMeta`), which the
    /// statistics guard reads.
    pub fn meta(&self) -> AxisMeta {
        AxisMeta {
            touches: self.touches,
            annotation: self.annotation,
        }
    }
}
