//! Each registered chart's descriptors (chart_decoder_contract Part 5; chart_reference §5.1's charts; REQ-CHART-014):
//! the axes, slice and conventions, `system_image`, `has_feasibility_boundary`, `coupling` and annotation of every
//! chart in `kernel::chart`, and the Burrau family's two curves, `ν` and the acute angle `θ`, with their tangents.
//!
//! **The slices.** The latent chart holds its plane's orthogonal complement, the six directions `z₀` fixes. The shape
//! sphere holds `z₀`'s mass and momentum blocks (chart_reference §3, "The held values"); the invariant charts their
//! configuration and mass controls (§2). A mixed-axis chart holds its six other controls (§1.3). The ternary mass plot
//! holds the triangle at `ν₀` (§4.5), and the bifurcation strips the invariant they do not sweep, `L_z = 0` or `K`.
//!
//! **The conventions.** A Burrau rest start and the mass plot set every momentum 0; on the invariant charts and the
//! strips the construction's deterministic seed pins the momentum block's two DOF the two invariants leave
//! (chart_reference §2.2, R-82).
//!
//! **`system_image`** (R-27, R-104, R-141, R-157). The shape sphere is n-to-1 with n = 2, its φ hemispheres
//! reflection-equivalent; its descriptors are the `(θ, φ)` chart's (R-117), for either chart map. The `ν`-axis charts
//! are `DoubleCover` where their range reaches both leg orderings and bijective otherwise (`kernel::chart::burrau`); the
//! acute-angle charts fold the swap out and are bijective. Every other chart is bijective: its decode takes distinct
//! points to distinct systems.

use kernel::burrau::System;
use kernel::chart::burrau::embed;
use kernel::chart::burrau_acute::nu_of;
use kernel::chart::mixed::{Coordinate, Quantity};
use kernel::chart::{
    BlockTouch, Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart, EuclidPlane,
    InvariantLE, InvariantLK, Latent, MassSimplex, Mixed, ShapeMap, ShapeSphere, SystemImage,
};

use super::axis::{
    no_blocks, union, AxisDescriptor, Block, CurveMeta, InvariantMeta, InvariantQuantity, Residual,
    COORDINATES,
};
use super::descriptor::{Authored, ChartDescriptor, Convention, Coupling, Frozen};
use crate::contract::sim_config::LATENT_DIM;

// ── The Burrau family's curves ─────────────────────────────────────────────────────────────────────────────────────

/// `γ(ν)`, the `ν` axis's embed: the normalised Burrau system at `a = 1 − ν²`, `b = 2ν`, `c = 1 + ν²`
/// (`kernel::chart::burrau::embed`; chart_reference §4.3).
pub fn nu_embed(nu: f64) -> System<f64> {
    embed(nu)
}

/// `γ'(ν)`, the embed's tangent, each component differentiated in closed form: with `s = a + b + c = 2(1 + ν)`, the
/// masses `(c, b, a)/s` are `((1 + ν²)/(2(1 + ν)), ν/(1 + ν), (1 − ν)/2)`, `a/c = (1 − ν²)/(1 + ν²)`,
/// `b/c = 2ν/(1 + ν²)` and `λ_x = −ab/(c(b + c)) = −2ν(1 − ν)/((1 + ν²)(1 + ν))`.
pub fn nu_tangent(nu: f64) -> System<f64> {
    let c = 1.0 + nu * nu;
    let p = 1.0 + nu;
    let a_c = -4.0 * nu / (c * c);
    let b_c = 2.0 * (1.0 - nu * nu) / (c * c);
    let (n, dn) = (2.0 * nu * (1.0 - nu), 2.0 - 4.0 * nu);
    let (d, dd) = (c * p, 1.0 + 2.0 * nu + 3.0 * nu * nu);
    let lambda_x = -(dn * d - n * dd) / (d * d);
    System {
        m: [
            (nu * nu + 2.0 * nu - 1.0) / (2.0 * p * p),
            1.0 / (p * p),
            -0.5,
        ],
        r: [[0.0, 0.0], [a_c, 0.0], [0.0, b_c]],
        rho: [a_c, 0.0],
        lambda: [lambda_x, b_c],
    }
}

/// `γ(ν(θ))`, the acute-angle axis's embed, `ν(θ) = cos θ/(1 + sin θ)` (`kernel::chart::burrau_acute::nu_of`).
pub fn theta_embed(theta: f64) -> System<f64> {
    embed(nu_of(theta))
}

/// `γ'(ν(θ))·ν'(θ)`, its tangent: `ν'(θ) = −1/(1 + sin θ)`.
pub fn theta_tangent(theta: f64) -> System<f64> {
    let t = nu_tangent(nu_of(theta));
    let k = -1.0 / (1.0 + theta.sin());
    System {
        m: t.m.map(|x| x * k),
        r: t.r.map(|r| r.map(|x| x * k)),
        rho: t.rho.map(|x| x * k),
        lambda: t.lambda.map(|x| x * k),
    }
}

/// The `ν` curve.
pub const NU_CURVE: CurveMeta = CurveMeta {
    parameter: "ν",
    embed: nu_embed,
    tangent: nu_tangent,
};

/// The acute-angle curve.
pub const THETA_CURVE: CurveMeta = CurveMeta {
    parameter: "θ",
    embed: theta_embed,
    tangent: theta_tangent,
};

// ── Shared pieces ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The blocks a Burrau curve ties and every invariant reads: configuration and mass.
fn config_mass() -> BlockTouch {
    union(Block::Config.only(), Block::Mass.only())
}

/// An invariant axis solving into the momenta, reading the configuration and the masses (chart_reference §2.2: the
/// construction reads the potential `U` and `I` of the decoded configuration and masses).
fn invariant(name: &'static str, quantity: InvariantQuantity) -> AxisDescriptor {
    AxisDescriptor::invariant(
        name,
        InvariantMeta::new(quantity, Block::Momentum, config_mass()),
    )
}

/// The construction's deterministic seed, pinning the two momentum DOF two invariants leave.
fn seed() -> Convention {
    Convention {
        block: Block::Momentum,
        pins: 2,
        statement: "the invariant construction's deterministic seed (chart_reference §2.2, R-82)",
    }
}

/// The rest start: every momentum 0.
fn rest() -> Convention {
    Convention {
        block: Block::Momentum,
        pins: Block::Momentum.dof(),
        statement: "a rest start: every momentum 0",
    }
}

/// The coupling of a Burrau curve with a second axis writing `other`.
fn tied(other: BlockTouch) -> Coupling {
    let ties = config_mass();
    Coupling {
        ties,
        collisions: BlockTouch {
            config: ties.config & other.config,
            momentum: ties.momentum & other.momentum,
            mass: ties.mass & other.mass,
        },
    }
}

/// The coordinates `which` holds, frozen.
fn frozen(which: impl Fn(Coordinate) -> bool) -> Vec<Frozen> {
    COORDINATES
        .into_iter()
        .filter(|&c| which(c))
        .map(Frozen::Coordinate)
        .collect()
}

/// Every block: the latent plane's complement spans them all.
fn all_blocks() -> BlockTouch {
    union(config_mass(), Block::Momentum.only())
}

// ── The charts ─────────────────────────────────────────────────────────────────────────────────────────────────────

impl<P: Copy> Authored for Latent<P>
where
    f64: From<P>,
{
    /// The plane's two latent directions, kind 1, and its complement held: 2 swept + 6 frozen.
    fn descriptor(&self) -> ChartDescriptor {
        let wide = |q: [P; LATENT_DIM]| q.map(<f64 as From<P>>::from);
        ChartDescriptor {
            axes: [
                AxisDescriptor::raw_latent("q₁", wide(self.q1)),
                AxisDescriptor::raw_latent("q₂", wide(self.q2)),
            ],
            slice: vec![Frozen::Complement {
                touches: all_blocks(),
                dof: LATENT_DIM as u32 - 2,
            }],
            conventions: vec![],
            system_image: SystemImage::Bijective,
            has_feasibility_boundary: false,
            coupling: Coupling::none(),
            annotation: false,
        }
    }
}

impl<P> Authored for ShapeSphere<P>
where
    ShapeSphere<P>: Chart,
{
    /// The shape's two angles, kind 2 in the configuration, each the other's residual; `z₀`'s mass and momentum blocks
    /// held. n-to-1 with n = 2 (R-141), the `(θ, φ)` chart's (R-117).
    fn descriptor(&self) -> ChartDescriptor {
        let (h, v) = match self.map {
            ShapeMap::Spherical => ("θ, the azimuth", "φ, the polar angle"),
            ShapeMap::Exponential => (
                "the exponential map along e₁",
                "the exponential map along e₂",
            ),
        };
        let other = Residual {
            pins: 0,
            statement: "the configuration block's other DOF is the chart's other axis",
        };
        ChartDescriptor {
            axes: [
                AxisDescriptor::derived(h, Block::Config, None, other),
                AxisDescriptor::derived(v, Block::Config, None, other),
            ],
            slice: frozen(|c| Block::of(c) != Block::Config),
            conventions: vec![],
            system_image: SystemImage::NToOne(2),
            has_feasibility_boundary: false,
            coupling: Coupling::none(),
            annotation: false,
        }
    }
}

impl<P> Authored for Burrau<P>
where
    P: Copy,
    f64: From<P>,
{
    /// `ν`, kind 4 over configuration and mass, and `K`, kind 3; `L_z = 0` held and the seed.
    fn descriptor(&self) -> ChartDescriptor {
        let k = invariant("K", InvariantQuantity::K);
        ChartDescriptor {
            axes: [AxisDescriptor::curve("ν", config_mass(), NU_CURVE), k],
            slice: vec![Frozen::Held {
                quantity: "L_z = 0",
                block: Block::Momentum,
                dof: 1,
            }],
            conventions: vec![seed()],
            system_image: self.system_image(),
            has_feasibility_boundary: true,
            coupling: tied(k.touches()),
            annotation: false,
        }
    }
}

impl<P> Authored for EuclidPlane<P>
where
    P: Copy,
    f64: From<P>,
{
    /// `m` annotation-only and `ν`, kind 4; a rest start: 1 swept + 7 frozen.
    fn descriptor(&self) -> ChartDescriptor {
        ChartDescriptor {
            axes: [
                AxisDescriptor::annotation("m"),
                AxisDescriptor::curve("ν", config_mass(), NU_CURVE),
            ],
            slice: vec![],
            conventions: vec![rest()],
            system_image: self.system_image(),
            has_feasibility_boundary: false,
            coupling: tied(no_blocks()),
            annotation: true,
        }
    }
}

impl<P> Authored for BurrauThetaK<P>
where
    BurrauThetaK<P>: Chart,
{
    /// `θ`, kind 4, and `K`, kind 3; `L_z = 0` held and the seed.
    fn descriptor(&self) -> ChartDescriptor {
        let k = invariant("K", InvariantQuantity::K);
        ChartDescriptor {
            axes: [AxisDescriptor::curve("θ", config_mass(), THETA_CURVE), k],
            slice: vec![Frozen::Held {
                quantity: "L_z = 0",
                block: Block::Momentum,
                dof: 1,
            }],
            conventions: vec![seed()],
            system_image: SystemImage::Bijective,
            has_feasibility_boundary: true,
            coupling: tied(k.touches()),
            annotation: false,
        }
    }
}

impl<P> Authored for BurrauThetaLz<P>
where
    BurrauThetaLz<P>: Chart,
{
    /// `θ`, kind 4, and `L_z`, kind 3; `K` held and the seed.
    fn descriptor(&self) -> ChartDescriptor {
        let lz = invariant("L_z", InvariantQuantity::Lz);
        ChartDescriptor {
            axes: [AxisDescriptor::curve("θ", config_mass(), THETA_CURVE), lz],
            slice: vec![Frozen::Held {
                quantity: "K",
                block: Block::Momentum,
                dof: 1,
            }],
            conventions: vec![seed()],
            system_image: SystemImage::Bijective,
            has_feasibility_boundary: true,
            coupling: tied(lz.touches()),
            annotation: false,
        }
    }
}

impl<P> Authored for BurrauThetaDeltaM<P>
where
    BurrauThetaDeltaM<P>: Chart,
{
    /// `θ`, kind 4, and δm, kind 2 in the masses, which the curve ties too: a same-block collision, flagged. A rest
    /// start.
    fn descriptor(&self) -> ChartDescriptor {
        let dm = AxisDescriptor::derived(
            "δm",
            Block::Mass,
            None,
            Residual {
                pins: 0,
                statement: "the blend from the curve's Burrau masses toward m_target",
            },
        );
        ChartDescriptor {
            axes: [AxisDescriptor::curve("θ", config_mass(), THETA_CURVE), dm],
            slice: vec![],
            conventions: vec![rest()],
            system_image: SystemImage::Bijective,
            has_feasibility_boundary: false,
            coupling: tied(dm.touches()),
            annotation: false,
        }
    }
}

impl<P> Authored for InvariantLE<P>
where
    InvariantLE<P>: Chart,
{
    /// `L_z` and `E`, kind 3, solving jointly into the momenta; the configuration and mass controls held, the seed.
    fn descriptor(&self) -> ChartDescriptor {
        invariant_chart(invariant("E", InvariantQuantity::E))
    }
}

impl<P> Authored for InvariantLK<P>
where
    InvariantLK<P>: Chart,
{
    /// `L_z` and `K`, kind 3, as on `(L_z, E)`.
    fn descriptor(&self) -> ChartDescriptor {
        invariant_chart(invariant("K", InvariantQuantity::K))
    }
}

/// An invariant chart's descriptor: `L_z` horizontal, `vertical` vertical (chart_reference §2).
fn invariant_chart(vertical: AxisDescriptor) -> ChartDescriptor {
    ChartDescriptor {
        axes: [invariant("L_z", InvariantQuantity::Lz), vertical],
        slice: frozen(|c| Block::of(c) != Block::Momentum),
        conventions: vec![seed()],
        system_image: SystemImage::Bijective,
        has_feasibility_boundary: true,
        coupling: Coupling::none(),
        annotation: false,
    }
}

impl<P> Authored for MassSimplex<P>
where
    MassSimplex<P>: Chart,
{
    /// The two simplex coordinates, kind 2 in the masses, each the other's residual; the triangle at `ν₀` held, a
    /// rest start.
    fn descriptor(&self) -> ChartDescriptor {
        let other = Residual {
            pins: 0,
            statement: "the mass block's other DOF is the chart's other axis",
        };
        ChartDescriptor {
            axes: [
                AxisDescriptor::derived("m₁ = x", Block::Mass, None, other),
                AxisDescriptor::derived("y = (1 − x)·v", Block::Mass, None, other),
            ],
            slice: vec![Frozen::Held {
                quantity: "the Burrau triangle at ν₀",
                block: Block::Config,
                dof: Block::Config.dof(),
            }],
            conventions: vec![rest()],
            system_image: SystemImage::Bijective,
            has_feasibility_boundary: false,
            coupling: Coupling::none(),
            annotation: false,
        }
    }
}

/// A mixed chart's axis `Q`: its latent coordinate, kind 1, or its derived quantity, kind 2, setting its control.
fn mixed_axis<Q: Quantity>() -> AxisDescriptor {
    if Q::DERIVED {
        AxisDescriptor::derived(
            Q::NAME,
            Block::of(Q::COORDINATE),
            Some(Q::COORDINATE),
            Residual {
                pins: 0,
                statement: "the block's other controls: the slice's, or the chart's other axis",
            },
        )
    } else {
        AxisDescriptor::coordinate(Q::NAME, Q::COORDINATE)
    }
}

impl<H: Quantity, V: Quantity, P> Authored for Mixed<H, V, P>
where
    Mixed<H, V, P>: Chart,
{
    /// The two axes' coordinates swept, the other six controls held.
    fn descriptor(&self) -> ChartDescriptor {
        ChartDescriptor {
            axes: [mixed_axis::<H>(), mixed_axis::<V>()],
            slice: frozen(|c| c != H::COORDINATE && c != V::COORDINATE),
            conventions: vec![],
            system_image: SystemImage::Bijective,
            has_feasibility_boundary: false,
            coupling: Coupling::none(),
            annotation: false,
        }
    }
}
