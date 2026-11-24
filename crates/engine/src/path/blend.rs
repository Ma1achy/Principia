//! The blend chart (TASK-M2-90; design § 17.2, § 17.4, § 17.11; the chart-space model's § 0.1): the state blend of two
//! views at a share `s`, a chart in its own right (a morph frame is a chart, area statistics its one exception).
//!
//! **The default path, block by block** ([`BlendPath::BlockLine`]; § 17.2). Each end's pixel gives its signed state
//! `(m, r, p)`, its own decode. The blend reads it as three blocks, each blended in its own geometry:
//!
//! ```text
//! mass      μ_k = ln(m_k/m₀), k = 1, 2      a + s(b − a)
//! shape     n̂, the signed shape-sphere point   the great circle from n̂_A to n̂_B (§ 17.2's two degenerate cases)
//! momentum  (p_ρ, p_λ), the body-frame Jacobi momenta   a + s(b − a)
//! ```
//!
//! and builds the state from them with the kernel's own pieces, no new physics: the masses `softmax(0, μ₁, μ₂)` through
//! the shared entry for a chart that sets masses ([`mass::from_masses`], whose `M01_TINY` test applies); the shape
//! sphere's [`place`] at `I = 1` and phase 0, so `ρ̃` lies on `+x` and the momenta are already in the decode's gauge;
//! [`momentum::to_particles`]; then the one shared `C` ([`canon::canonicalise`]) and the shared descriptor. Each end's
//! blocks are read off its canonical state, where `ρ̃` lies on `+x` and `I = 1`: `n̂` by [`shape`], `p_λ = p₂` and
//! `p_ρ = p₁ + (m₁/M₀₁)·p₂`, the inverse of the decoder's Jacobi-to-particle map. So a block line between two equal
//! states gives that state again, to rounding.
//!
//! **The ends are exact** (§ 17.4): `s` is compared exactly, `0.0` evaluating `A`'s own pixel and `1.0` `B`'s, never
//! the block line. An end may itself be a blend ([`super::Map::Blend`]): its pixel is that blend's, at its own share.
//!
//! **The mass hold** (R-975; § 17.11, "Out of feasible", item 2). The masses blend linearly in `μ`, so `M₀₁ = g/(1 + g)`,
//! `g(s) = e^{−μ₂(s)} + e^{μ₁(s) − μ₂(s)}`, is convex in `s` and can dip below `ε_M01` between two valid ends. Such a
//! pixel keeps the blended `m₁ : m₀` and lifts the light pair to the allowed minimum, the rest of the blend unchanged
//! ([`hold_m01`]). The guard fires strictly below `ε_M01` on the one rounded sum `m₀ + m₁` (`kernel::decode::mass`'s
//! note), so the lifted state is defined against that sum: `M₀₁` is set to `ε_M01` and stepped up an ulp at a time,
//! at most a few, until the decoder's own test passes. The held pixel is flagged [`Flags::m01_held`], never tagged.
//! Where the plain blend already passes the guard nothing is changed, so the hold is continuous in `s` and across
//! pixels, equal to the plain blend where the dip begins and ends, to an ulp of `ε_M01`.
//!
//! **An end with no state** (§ 17.11, "Blank or forbidden at an end", "Tagged at an end"): a pixel is in a mid frame
//! only if each end has a state there. An end's decode that is not finite, or tagged `M01_TINY` or
//! `NO_QUALIFYING_SEED`, has none, and the pixel is [`Pixel::Blank`] for every `0 < s < 1`. `ENERGY_UNSCALABLE` and
//! `ENERGY_INFEASIBLE` carry a real state and blend. § 17.11 blends `INVARIANT_INFEASIBLE`'s arm `K* < L_z²/(2I)` and
//! not its arm "`2K*` has no finite float", and the tag alone does not say which fired. The state does: the overflow
//! arm sets every momentum to 0 (`kernel::chart::invariant`'s "The overflow edge": "not a system"), while the other is
//! the rigid rotation at the pixel's `L_z`, nonzero since `K* ≥ 0 > K* − L_z²/(2I)` needs `L_z ≠ 0`. So an end tagged
//! `INVARIANT_INFEASIBLE` with every momentum exactly 0 has no state, and one with any momentum blends.
//!
//! **Navigation reaches both ends** (R-926: "navigated by one pixel-space edit on both ends"). A pan or zoom of a blend
//! is that pan or zoom of each end, through the chart-path object's own operation ([`Segment::pan`],
//! [`Segment::zoom_factor`]), recursively into an end that is a blend. A turn is held by the blend as an angle about the
//! window's centre and applied to the address before either end reads it, and a later pan is turned by it, so the pan
//! is a pan of the screen; a rigid turn of the picture on a square window only ([`Blend::address`]). The plan is unclear here (no window shape turns today); chose to hold the turn in the blend,
//! because neither the latent plane's nor a kernel map's window can carry one without moving every view's sim key. At a
//! turn of 0 the address is passed through untouched, bit for bit.
//!
//! **Exposed** to placement and to `computeIC`: [`Blend::forward`] is the f64 forward map, a pixel to its blocks;
//! [`Blend::decode`] is the pixel's decode, a kernel [`Decoded`] the integrator takes, with the blend's flags.
//!
//! **The signed fold indicator** ([`fold_indicator`]; § 17.11): defined there.

use kernel::canon;
use kernel::chart::shape_sphere::place;
use kernel::chart::Chart;
use kernel::decode::degenerate::Degenerate;
use kernel::decode::descriptor;
use kernel::decode::mass::{self, Masses};
use kernel::decode::momentum::{self, Momentum};
use kernel::decode::{Decoded, Ic};
use kernel::generated::links::Selection;
use kernel::shape::shape;

use super::anchor::{self, Anchor, OffsetForm};
use super::{BlendPath, KernelMap, Map, PathError, Segment, View};
use crate::contract::sim_config::Plane;

// ── The leaves ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// The decode of a leaf view, a view that is not a blend, at an address of its window. A view names its kernel map by
/// registry index alone (`KernelMap`), so the chart's own parameters come from the caller, as a lock's do.
pub trait Leaves {
    /// The decode of `view` at the address `address` of its window's `[0,1]²`; [`PathError::WrongMap`] for a map this
    /// resolver does not evaluate.
    fn decode(&self, view: &View, address: [f64; 2]) -> Result<Decoded<f64>, PathError>;
}

/// The latent map's leaves under the link selection `S`: `D ∘ Φ` of the plane at the address.
pub struct LatentLeaves<S>(core::marker::PhantomData<S>);

impl<S> LatentLeaves<S> {
    /// The resolver.
    pub fn new() -> Self {
        LatentLeaves(core::marker::PhantomData)
    }
}

impl<S> Default for LatentLeaves<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// The latent chart of `plane`.
fn latent(plane: &Plane) -> kernel::chart::Latent<f64> {
    kernel::chart::Latent {
        z0: plane.z0,
        q1: plane.q1,
        q2: plane.q2,
    }
}

impl<S: Selection> Leaves for LatentLeaves<S> {
    fn decode(&self, view: &View, [s, t]: [f64; 2]) -> Result<Decoded<f64>, PathError> {
        match view.map {
            Map::Latent => Ok(latent(&view.sim.plane).decode::<S, f64>(s, t)),
            _ => Err(PathError::WrongMap),
        }
    }
}

/// A registry chart's leaves under `S`: the chart's decode at its window's point of the address. An untilted view
/// only: a curved view's own tilt as a blend's end is not built.
pub struct ChartLeaves<C, S> {
    chart: C,
    map: KernelMap,
    selection: core::marker::PhantomData<S>,
}

impl<C: Chart, S> ChartLeaves<C, S> {
    /// The resolver of `chart`; [`PathError::WrongMap`] for a chart the registry does not name.
    pub fn new(chart: C) -> Result<Self, PathError> {
        let map = KernelMap::of(&chart)?;
        Ok(ChartLeaves {
            chart,
            map,
            selection: core::marker::PhantomData,
        })
    }
}

impl<C, S> Leaves for ChartLeaves<C, S>
where
    C: Chart,
    f64: From<C::Param>,
    S: Selection,
{
    fn decode(&self, view: &View, [s, t]: [f64; 2]) -> Result<Decoded<f64>, PathError> {
        if view.map != Map::Kernel(self.map) {
            return Err(PathError::WrongMap);
        }
        if view.tilt.is_some_and(|tilt| !tilt.spec().untilted()) {
            return Err(PathError::NotBuilt("a tilted curved view as a blend's end"));
        }
        let [u, v] = view.sim.chart.window.at(s, t);
        Ok(self.chart.decode::<S, f64>(u, v))
    }
}

/// Two resolvers: the first's decode, or the second's where the first does not evaluate the map.
impl<L1: Leaves, L2: Leaves> Leaves for (L1, L2) {
    fn decode(&self, view: &View, address: [f64; 2]) -> Result<Decoded<f64>, PathError> {
        match self.0.decode(view, address) {
            Err(PathError::WrongMap) => self.1.decode(view, address),
            other => other,
        }
    }
}

// ── The blocks ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// A state's blocks, the coordinates the default path blends (the module's note).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blocks {
    /// `(μ₁, μ₂)`, `μ_k = ln(m_k/m₀)`.
    pub mu: [f64; 2],
    /// The signed shape-sphere point `n̂`.
    pub n: [f64; 3],
    /// The inner pair's body-frame Jacobi momentum `p_ρ`.
    pub p_rho: [f64; 2],
    /// The outer body-frame Jacobi momentum `p_λ`.
    pub p_lambda: [f64; 2],
}

impl Blocks {
    /// The blocks of a canonical state `ic` (CoM at rest at the origin, `I = 1`, `ρ̃` on `+x`).
    pub fn of(ic: &Ic<f64>) -> Self {
        let [m0, m1, m2] = ic.m;
        let w1 = m1 / mass::from_masses(ic.m).m01;
        let p2 = ic.p[2];
        Blocks {
            mu: [libm::log(m1) - libm::log(m0), libm::log(m2) - libm::log(m0)],
            n: shape(ic.r, ic.m),
            p_rho: [ic.p[1][0] + w1 * p2[0], ic.p[1][1] + w1 * p2[1]],
            p_lambda: p2,
        }
    }

    /// The coordinates as one vector, `(μ₁, μ₂, n̂, p_ρ, p_λ)`, in the blocks' own metric (the fold indicator's).
    pub fn vector(&self) -> [f64; 9] {
        [
            self.mu[0],
            self.mu[1],
            self.n[0],
            self.n[1],
            self.n[2],
            self.p_rho[0],
            self.p_rho[1],
            self.p_lambda[0],
            self.p_lambda[1],
        ]
    }

    /// Whether every coordinate is finite.
    fn finite(&self) -> bool {
        self.vector().iter().all(|x| x.is_finite())
    }
}

/// `a + s(b − a)`.
fn lerp(a: f64, b: f64, s: f64) -> f64 {
    a + s * (b - a)
}

/// `‖w‖`, scaled by its largest component (as `hypot`), so it underflows to 0 only where `w` is 0.
fn scaled_norm(w: [f64; 3]) -> f64 {
    let big = w.iter().fold(0.0_f64, |a, x| a.max(x.abs()));
    if big == 0.0 {
        return 0.0;
    }
    let [x, y, z] = w.map(|c| c / big);
    big * (x * x + y * y + z * z).sqrt()
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The great circle from `a` to `b` at `0 < s < 1` (§ 17.2, "The great circle's two degenerate cases"), and whether the
/// pixel takes the antipodal tie, a declared seam. With `c = a·b` and `w = b − c·a`: the general circle
/// `cos(sθ)·a + sin(sθ)·w/‖w‖`, `θ = atan2(‖w‖, c)`, where `c ≥ 0` and `‖w‖ > 0`, or `c < 0` and `‖w‖ > √ε_64`; `a`
/// itself where `w = 0` and `c ≥ 0`; else the tie, about `k̂ = (a × ê)/‖a × ê‖`, `ê` the coordinate axis with the
/// smallest `|a·ê|` (the lowest index on a tie): `cos(πs)·a + sin(πs)·(k̂ × a)`.
pub fn great_circle(a: [f64; 3], b: [f64; 3], s: f64) -> ([f64; 3], bool) {
    let c = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let w = [b[0] - c * a[0], b[1] - c * a[1], b[2] - c * a[2]];
    let norm = scaled_norm(w);
    let general = match c >= 0.0 {
        true => norm > 0.0,
        false => norm > f64::EPSILON.sqrt(),
    };
    if general {
        let theta = libm::atan2(norm, c);
        let (sin, cos) = (libm::sin(s * theta), libm::cos(s * theta));
        return (
            std::array::from_fn(|i| cos * a[i] + sin * (w[i] / norm)),
            false,
        );
    }
    if c >= 0.0 {
        return (a, false);
    }
    let mut axis = 0;
    for i in 1..3 {
        if a[i].abs() < a[axis].abs() {
            axis = i;
        }
    }
    let mut e = [0.0; 3];
    e[axis] = 1.0;
    let k = cross(a, e);
    let k_norm = scaled_norm(k);
    let k = k.map(|x| x / k_norm);
    let turn = cross(k, a);
    let angle = core::f64::consts::PI * s;
    let (sin, cos) = (libm::sin(angle), libm::cos(angle));
    (std::array::from_fn(|i| cos * a[i] + sin * turn[i]), true)
}

/// The default path between two ends' blocks at `0 < s < 1`, and whether the shape took the antipodal tie.
pub fn block_line(a: &Blocks, b: &Blocks, s: f64) -> (Blocks, bool) {
    let (n, tie) = great_circle(a.n, b.n, s);
    let blocks = Blocks {
        mu: std::array::from_fn(|k| lerp(a.mu[k], b.mu[k], s)),
        n,
        p_rho: std::array::from_fn(|k| lerp(a.p_rho[k], b.p_rho[k], s)),
        p_lambda: std::array::from_fn(|k| lerp(a.p_lambda[k], b.p_lambda[k], s)),
    };
    (blocks, tie)
}

/// `block_line(a, b, s) − a` as one vector in [`Blocks::vector`]'s order, formed without subtracting `a` back off: `s(b − a)`
/// on `μ` and the momenta, and on `n̂` the great circle's own offset, `(cos sθ − 1)·a + sin sθ·ŵ` with
/// `cos sθ − 1 = −2 sin²(sθ/2)`, on [`great_circle`]'s branches (its tie included). The same function as
/// `block_line(a, b, s).vector() − a.vector()` in exact arithmetic; in floating point that difference carries an
/// absolute rounding of `ε|a|` against a value of size `s|b − a|`, which grows as `1/s` relative to it, and this does
/// not. The lock's slide solves on it (`anchor`'s `Slide`).
pub fn line_offset(a: &Blocks, b: &Blocks, s: f64) -> [f64; 9] {
    let n = {
        let (a, b) = (a.n, b.n);
        let c = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let w = [b[0] - c * a[0], b[1] - c * a[1], b[2] - c * a[2]];
        let norm = scaled_norm(w);
        let general = match c >= 0.0 {
            true => norm > 0.0,
            false => norm > f64::EPSILON.sqrt(),
        };
        let turn = |angle: f64, dir: [f64; 3]| -> [f64; 3] {
            let half = libm::sin(angle / 2.0);
            let (cosm1, sin) = (-2.0 * half * half, libm::sin(angle));
            std::array::from_fn(|i| cosm1 * a[i] + sin * dir[i])
        };
        if general {
            let theta = libm::atan2(norm, c);
            turn(s * theta, w.map(|x| x / norm))
        } else if c >= 0.0 {
            [0.0; 3]
        } else {
            let mut axis = 0;
            for i in 1..3 {
                if a[i].abs() < a[axis].abs() {
                    axis = i;
                }
            }
            let mut e = [0.0; 3];
            e[axis] = 1.0;
            let k = cross(a, e);
            let k_norm = scaled_norm(k);
            let k = k.map(|x| x / k_norm);
            turn(core::f64::consts::PI * s, cross(k, a))
        }
    };
    let d = |x: f64, y: f64| s * (y - x);
    [
        d(a.mu[0], b.mu[0]),
        d(a.mu[1], b.mu[1]),
        n[0],
        n[1],
        n[2],
        d(a.p_rho[0], b.p_rho[0]),
        d(a.p_rho[1], b.p_rho[1]),
        d(a.p_lambda[0], b.p_lambda[0]),
        d(a.p_lambda[1], b.p_lambda[1]),
    ]
}

/// The masses `softmax(0, μ₁, μ₂)`, by the shared entry for a chart that sets masses, so the guard's own test applies.
pub fn masses_of(mu: [f64; 2]) -> Masses<f64> {
    let top = 0.0_f64.max(mu[0]).max(mu[1]);
    let e = [
        libm::exp(-top),
        libm::exp(mu[0] - top),
        libm::exp(mu[1] - top),
    ];
    let sum = e[0] + e[1] + e[2];
    mass::from_masses([e[0] / sum, e[1] / sum, e[2] / sum])
}

/// The most ulps [`hold_m01`] steps `M₀₁` up from `ε_M01` before it stops: the guard's sum rounds once, so the lifted
/// pair passes within a couple; a bound so the loop is finite, not a tolerance.
const HOLD_STEPS: u32 = ledger::constants::BLEND_HOLD_STEPS.number() as u32;

/// R-975's smooth hold (the module's note): `masses` where the guard passes; else the light pair lifted to the allowed
/// minimum with its ratio `m₁ : m₀` kept, `m₂ = 1 − M₀₁`, `M₀₁` stepped up from `ε_M01` an ulp at a time until
/// [`mass::from_masses`] raises no tag. The bool says whether it held.
pub fn hold_m01(masses: Masses<f64>, ratio: f64) -> (Masses<f64>, bool) {
    if masses.degenerate.is_none() {
        return (masses, false);
    }
    let mut pair = mass::m01_epsilon::<f64>();
    for _ in 0..HOLD_STEPS {
        let m0 = pair / (1.0 + ratio);
        let m1 = pair - m0;
        let held = mass::from_masses([m0, m1, 1.0 - pair]);
        if held.degenerate.is_none() {
            return (held, true);
        }
        pair = pair.next_up();
    }
    (masses, false)
}

// ── The pixel ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// Which end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// `A`, at `s = 0`.
    A,
    /// `B`, at `s = 1`.
    B,
}

/// Why an end has no state at a pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoState {
    /// The end's decode is not finite.
    NonFinite,
    /// The end's decode is tagged with a reason that carries no state (§ 17.11's table).
    Tag(Degenerate),
    /// The end is a blend whose own pixel is blank.
    Blank,
}

/// A blank pixel of a mid frame: an end with no state there (§ 17.11), held for every `0 < s < 1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blank {
    /// The first end found with no state.
    pub end: End,
    /// Why.
    pub why: NoState,
}

/// What the blend declares at a pixel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    /// The shape took the great circle's antipodal tie, a declared seam (§ 17.2).
    pub antipodal_tie: bool,
    /// The masses were held at the guard (R-975).
    pub m01_held: bool,
}

/// A pixel of a blend chart: a system, or blank.
#[derive(Clone, Debug, PartialEq)]
pub enum Pixel {
    /// A system: its decode, the integrator's input, and the blend's flags.
    State {
        /// The decode.
        decoded: Box<Decoded<f64>>,
        /// The flags.
        flags: Flags,
    },
    /// No system: an end has none.
    Blank(Blank),
}

/// The forward map's value at a pixel: the blocks, or blank.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Forward {
    /// The pixel's blocks.
    Point {
        /// The blocks.
        blocks: Blocks,
        /// The shape took the antipodal tie.
        antipodal_tie: bool,
    },
    /// No system.
    Blank(Blank),
}

/// A locked frame's pixel, before the state is built.
enum Locked {
    /// An end has no state.
    Blank(Blank),
    /// The select: the stored words.
    Select(Box<Decoded<f64>>),
    /// The blocks of the offset form or the slide's pin state, and the great circle's tie.
    Blocks(Blocks, bool),
}

/// Whether a tagged end's state blends (§ 17.11's table): `M01_TINY` and `NO_QUALIFYING_SEED` never;
/// `INVARIANT_INFEASIBLE` only off its overflow arm, where some momentum is not 0 (the module's note).
fn blends(tag: Option<Degenerate>, ic: &Ic<f64>) -> bool {
    match tag {
        Some(Degenerate::M01Tiny) | Some(Degenerate::NoQualifyingSeed) => false,
        Some(Degenerate::InvariantInfeasible) => ic.p.iter().flatten().any(|x| *x != 0.0),
        _ => true,
    }
}

/// Whether every number of `ic` is finite.
fn finite(ic: &Ic<f64>) -> bool {
    ic.m.iter()
        .chain(ic.r.iter().flatten())
        .chain(ic.p.iter().flatten())
        .all(|x| x.is_finite())
}

/// The state an end carries into the blend at a pixel with its flags, or why it carries none.
fn carried(end: End, pixel: &Pixel) -> Result<(Ic<f64>, Flags), Blank> {
    match pixel {
        Pixel::Blank(_) => Err(Blank {
            end,
            why: NoState::Blank,
        }),
        Pixel::State { decoded, flags } => {
            if !finite(&decoded.ic) {
                return Err(Blank {
                    end,
                    why: NoState::NonFinite,
                });
            }
            match blends(decoded.degenerate, &decoded.ic) {
                true => Ok((decoded.ic, *flags)),
                false => Err(Blank {
                    end,
                    why: NoState::Tag(decoded.degenerate.expect("a tag that does not blend")),
                }),
            }
        }
    }
}

/// The decode of `blocks`: the masses (held at the guard where they dip), [`place`] at `I = 1` and phase 0, the
/// momenta, `C` and the descriptor (the module's note).
pub fn state_of(blocks: &Blocks) -> (Decoded<f64>, bool) {
    let (masses, held) = hold_m01(masses_of(blocks.mu), libm::exp(blocks.mu[0]));
    let config = place(blocks.n, 1.0, 0.0, &masses);
    let p = momentum::to_particles(&masses, blocks.p_rho, blocks.p_lambda);
    // A blend has no momentum controls: `q` holds the Jacobi momenta, as `D_mom` sets `p_ρ = (q₀, q₁)`,
    // `p_λ = (q₂, q₃)`, and is not bounded by `q_max`.
    let momentum = Momentum {
        q: [
            blocks.p_rho[0],
            blocks.p_rho[1],
            blocks.p_lambda[0],
            blocks.p_lambda[1],
        ],
        p_rho: blocks.p_rho,
        p_lambda: blocks.p_lambda,
        p,
    };
    let ic = canon::canonicalise(&Ic {
        m: masses.m,
        r: config.r,
        p,
    })
    .ic;
    let decoded = Decoded {
        ic,
        masses,
        config,
        momentum,
        descriptor: descriptor::describe_decoded(&ic, &config),
        degenerate: masses.degenerate,
    };
    (decoded, held)
}

// ── The blend chart ────────────────────────────────────────────────────────────────────────────────────────────────

/// The blend chart: the state blend of two views at the share `s` on a named path, turned by `turn` about the window's
/// centre. An end may itself be a blend. Two blends are equal by their ends, path, share, turn, lock and re-hold, not by
/// the re-held `B` cached from a decode.
#[derive(Clone, Debug)]
pub struct Blend {
    /// The view at `s = 0`.
    pub a: View,
    /// The view at `s = 1`.
    pub b: View,
    /// The path, by name: nothing outside this evaluator assumes the default (R-943).
    pub path: BlendPath,
    s: f64,
    turn: f64,
    lock: Option<BlendLock>,
    /// `B` is re-placed, at each decode, to hold the state `A` shows at the pin: a mid-morph slice under a lock moves
    /// `A`'s pinned state, and `B` follows it ([`super::Segment::locked_slice`]).
    rehold: bool,
    /// `B` as re-placed, once per blend: computed at the first decode, with that decode's leaves, and kept for the
    /// blend's later decodes, which are expected to pass the same leaves (a chart's parameters, from the caller).
    reheld: std::sync::OnceLock<Result<(View, bool), PathError>>,
}

impl PartialEq for Blend {
    fn eq(&self, other: &Self) -> bool {
        self.a == other.a
            && self.b == other.b
            && self.path == other.path
            && self.s.to_bits() == other.s.to_bits()
            && self.turn.to_bits() == other.turn.to_bits()
            && self.lock == other.lock
            && self.rehold == other.rehold
    }
}

/// A blend under a lock (§ 17.8): read from its ends, never stored apart from them. `A` is locked with a recorded anchor;
/// `B` holds it where it carries the same anchor (re-centred on it), and otherwise is the slide's placement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlendLock {
    /// The pin's address.
    pub pin: [f64; 2],
    /// `A`'s anchor; `None` where `B` is re-held ([`Blend::reheld`]) and `A`'s pinned state moves with the frame: then the
    /// state `A` shows at the pin.
    pub anchor: Option<Anchor>,
    /// Both ends hold the anchor: the pin takes the stored words.
    pub hold: bool,
    /// The offset form and the select.
    pub form: OffsetForm,
}

impl BlendLock {
    /// The lock of a blend from `a` to `b`: `None` unless `a` is locked with a recorded anchor, or locked with `B`
    /// re-held to it (`rehold`).
    fn of(a: &View, b: &View, rehold: bool) -> Option<Self> {
        if !a.sim.lock.locked || (a.anchor.is_none() && !rehold) {
            return None;
        }
        let (pin, _) = super::anchor::pin_address(a)?;
        // A re-held `B`'s hold is decided at its decode ([`Blend::holds`]), not here.
        let hold = !rehold
            && a.anchor.is_some_and(|anchor| {
                b.anchor.is_some_and(|x| x.same_state(&anchor) && !x.slid) && b.sim.lock.locked
            });
        Some(BlendLock {
            pin,
            anchor: a.anchor.filter(|_| !rehold),
            hold,
            form: OffsetForm::default(),
        })
    }
}

impl Blend {
    /// The blend of `a` and `b` at `s` on `path`, untilted. `s` is canonicalised (`−0` is `+0`); NaN or outside
    /// `[0, 1]` is refused ([`PathError::Parameter`]). Where `a` is locked with a recorded anchor the blend is locked
    /// ([`BlendLock`]): its pixels take the offset form about the pin.
    pub fn new(a: View, b: View, path: BlendPath, s: f64) -> Result<Self, PathError> {
        Ok(Blend {
            lock: BlendLock::of(&a, &b, false),
            a,
            b,
            path,
            s: super::parameter(s)?,
            turn: 0.0,
            rehold: false,
            reheld: Default::default(),
        })
    }

    /// This blend with its ends `a` and `b` in place of its own, its share, path and turn kept, its lock read again.
    pub fn with_ends(&self, a: View, b: View) -> Self {
        Blend {
            a,
            b,
            reheld: Default::default(),
            ..self.clone()
        }
        .relocked()
    }

    /// This blend with `B` re-held to the state `A` shows at the pin (`true`), or not.
    pub fn reholding(mut self, rehold: bool) -> Self {
        self.rehold = rehold;
        self.reheld = Default::default();
        self.relocked()
    }

    /// Whether the frame holds the pinned state: its pin shows the stored words. A re-held `B`'s hold is its re-solve's
    /// (`anchor::rehold`), read through `leaves`; `false` on a free blend.
    pub fn holds<L: Leaves>(&self, leaves: &L) -> Result<bool, PathError> {
        let Some(lock) = &self.lock else {
            return Ok(false);
        };
        let held = self.end_b(leaves)?.1;
        #[cfg(feature = "controls")]
        let held = held || lock.form.forced_hold;
        let _ = lock;
        Ok(held)
    }

    /// Whether `B` is re-held to `A`'s pinned state.
    pub fn rehold(&self) -> bool {
        self.rehold
    }

    /// `B` as the frame reads it, and whether it holds the pinned state: re-held to `A`'s pinned state where
    /// [`Blend::rehold`] ([`anchor::rehold`]), else `B` and the lock's own hold.
    fn end_b<L: Leaves>(&self, leaves: &L) -> Result<(View, bool), PathError> {
        if !self.rehold {
            return Ok((self.b.clone(), self.lock.is_some_and(|l| l.hold)));
        }
        self.reheld
            // The plan is unclear here (the hold is decided in "lookup's `‖·‖_phys`", which a caller passes; a frame's
            // decode has no caller's norm in reach); chose the engine's own copy of that norm, `lookup::phys`, the
            // function every caller passes, because a frame built by `Segment::frame` or replayed from its record
            // must re-hold as the slice that made it did.
            .get_or_init(|| anchor::rehold(&self.a, &self.b, leaves, crate::lookup::phys, self.s))
            .clone()
    }

    /// This blend's lock read again from its ends, after an edit of them moved the pin; its form kept.
    fn relocked(mut self) -> Self {
        self.reheld = Default::default();
        let form = self.lock.map(|l| l.form);
        self.lock = BlendLock::of(&self.a, &self.b, self.rehold).map(|mut l| {
            l.form = form.unwrap_or_default();
            l
        });
        self
    }

    /// The lock, `None` on a free blend.
    pub fn lock(&self) -> Option<&BlendLock> {
        self.lock.as_ref()
    }

    /// This blend with its lock's offset form set to `form`: a control's switch (the select removed, offsets blended
    /// linearly).
    #[cfg(feature = "controls")]
    pub fn with_form(mut self, form: OffsetForm) -> Self {
        if let Some(lock) = &mut self.lock {
            lock.form = form;
        }
        self
    }

    /// The share `s`.
    pub fn s(&self) -> f64 {
        self.s
    }

    /// The turn about the window's centre, in radians.
    pub fn turn(&self) -> f64 {
        self.turn
    }

    /// This blend at another share.
    ///
    /// A re-held `B` (a frame inside a slice mid-morph) depends on the share on a sliding pair, so its cached re-solve is
    /// dropped: the blend at `s` re-solves at `s`, whether or not this blend was decoded first.
    pub fn at(&self, s: f64) -> Result<Self, PathError> {
        Ok(Blend {
            s: super::parameter(s)?,
            reheld: Default::default(),
            ..self.clone()
        })
    }

    /// A control's switch: the blend at `s` keeping this blend's cached re-solve, as `at` did before it dropped it.
    #[cfg(feature = "controls")]
    pub fn at_keeping_cache(&self, s: f64) -> Result<Self, PathError> {
        Ok(Blend {
            s: super::parameter(s)?,
            ..self.clone()
        })
    }

    /// The end `end`.
    pub fn end(&self, end: End) -> &View {
        match end {
            End::A => &self.a,
            End::B => &self.b,
        }
    }

    /// The address the ends read for the frame's address `p`: `p` turned by `turn` about `(½, ½)`, or `p` itself at a
    /// turn of 0. The turn is of the normalised address square, a rigid turn on screen only where the window's two
    /// half-widths are equal (on the latent map, `‖q₁‖ = ‖q₂‖`); on any other window the turned picture is sheared.
    pub fn address(&self, [s, t]: [f64; 2]) -> [f64; 2] {
        if self.turn == 0.0 {
            return [s, t];
        }
        let (sin, cos) = (libm::sin(self.turn), libm::cos(self.turn));
        let (x, y) = (s - 0.5, t - 0.5);
        [0.5 + (cos * x - sin * y), 0.5 + (sin * x + cos * y)]
    }

    /// The pixel of `view` at the window address `address`: a blend's own, or a leaf's decode.
    fn end_pixel<L: Leaves>(
        view: &View,
        leaves: &L,
        address: [f64; 2],
    ) -> Result<Pixel, PathError> {
        anchor::state_at(view, leaves, address)
    }

    /// Both ends' states at the frame's address, and whether either end's own pixel lies on an antipodal tie (a
    /// nested end's seam shows in the frame at every `s`, § 17.2); or the first end with none.
    #[allow(clippy::type_complexity)]
    fn carried<L: Leaves>(
        &self,
        leaves: &L,
        address: [f64; 2],
    ) -> Result<Result<([Ic<f64>; 2], bool), Blank>, PathError> {
        let at = self.address(address);
        let a = carried(End::A, &Self::end_pixel(&self.a, leaves, at)?);
        let b = carried(End::B, &Self::end_pixel(&self.b, leaves, at)?);
        Ok(match (a, b) {
            (Ok((a, fa)), Ok((b, fb))) => Ok(([a, b], fa.antipodal_tie || fb.antipodal_tie)),
            (Err(blank), _) | (_, Err(blank)) => Err(blank),
        })
    }

    /// The f64 forward map at the frame's address: the pixel's blocks. At `s = 0` and `s = 1` the end's own state's
    /// blocks; between them the path's.
    pub fn forward<L: Leaves>(&self, leaves: &L, address: [f64; 2]) -> Result<Forward, PathError> {
        let end = match self.s {
            0.0 => Some(End::A),
            1.0 => Some(End::B),
            _ => None,
        };
        if let Some(end) = end {
            let pixel = Self::end_pixel(self.end(end), leaves, self.address(address))?;
            return Ok(match pixel {
                Pixel::State { decoded, flags } if finite(&decoded.ic) => Forward::Point {
                    blocks: Blocks::of(&decoded.ic),
                    antipodal_tie: flags.antipodal_tie,
                },
                other => Forward::Blank(match carried(end, &other) {
                    Err(blank) => blank,
                    Ok(_) => unreachable!("a non-finite state is not carried"),
                }),
            });
        }
        if let Some(lock) = &self.lock {
            return Ok(match self.locked(lock, leaves, address)? {
                Locked::Blank(blank) => Forward::Blank(blank),
                Locked::Select(d) => Forward::Point {
                    blocks: Blocks::of(&d.ic),
                    antipodal_tie: false,
                },
                Locked::Blocks(blocks, antipodal_tie) => Forward::Point {
                    blocks,
                    antipodal_tie,
                },
            });
        }
        let ([a, b], ends_tie) = match self.carried(leaves, address)? {
            Ok(states) => states,
            Err(blank) => return Ok(Forward::Blank(blank)),
        };
        let (a, b) = (Blocks::of(&a), Blocks::of(&b));
        if !(a.finite() && b.finite()) {
            let end = if a.finite() { End::B } else { End::A };
            return Ok(Forward::Blank(Blank {
                end,
                why: NoState::NonFinite,
            }));
        }
        let BlendPath::BlockLine = self.path;
        let (blocks, antipodal_tie) = block_line(&a, &b, self.s);
        Ok(Forward::Point {
            blocks,
            antipodal_tie: antipodal_tie || ends_tie,
        })
    }

    /// The pixel at the frame's address: at `s = 0` and `s = 1` the end's own pixel, bit for bit, tag and all (§ 17.4);
    /// between them the decode of the forward map's blocks, or blank.
    pub fn decode<L: Leaves>(&self, leaves: &L, address: [f64; 2]) -> Result<Pixel, PathError> {
        match self.s {
            0.0 => return Self::end_pixel(&self.a, leaves, self.address(address)),
            1.0 => return Self::end_pixel(&self.b, leaves, self.address(address)),
            _ => {}
        }
        if let Some(lock) = &self.lock {
            if let Locked::Select(decoded) = self.locked(lock, leaves, address)? {
                return Ok(Pixel::State {
                    decoded,
                    flags: Flags::default(),
                });
            }
        }
        Ok(match self.forward(leaves, address)? {
            Forward::Blank(blank) => Pixel::Blank(blank),
            Forward::Point {
                blocks,
                antipodal_tie,
            } => {
                let (decoded, m01_held) = state_of(&blocks);
                Pixel::State {
                    decoded: Box::new(decoded),
                    flags: Flags {
                        antipodal_tie,
                        m01_held,
                    },
                }
            }
        })
    }

    /// The locked frame's pixel at `address`, `0 < s < 1` (§ 17.8): at the pin, the stored words where both ends hold
    /// the anchor (the select), else the frame's pin state, `block_line(a, b, s)` of the ends' pin states; elsewhere the
    /// offset form about the frame's pin state ([`anchor::offset_pixel`]). Blank where an end has no state at the
    /// pixel, at its pin or at its slope's probes.
    fn locked<L: Leaves>(
        &self,
        lock: &BlendLock,
        leaves: &L,
        address: [f64; 2],
    ) -> Result<Locked, PathError> {
        let at = self.address(address);
        let pin = lock.pin;
        let blank = |end: End| {
            Locked::Blank(Blank {
                end,
                why: NoState::Blank,
            })
        };
        let a_pin = match anchor::state_at(&self.a, leaves, pin)? {
            Pixel::State { decoded, .. } => *decoded,
            Pixel::Blank(_) => return Ok(blank(End::A)),
        };
        let pinned = lock.anchor.unwrap_or_else(|| Anchor::of(&a_pin.ic, false));
        let (end_b, holds) = self.end_b(leaves)?;
        #[cfg(feature = "controls")]
        let holds = holds || lock.form.forced_hold;
        let xa = pinned.blocks();
        let xb = match holds {
            true => xa,
            false => match anchor::state_at(&end_b, leaves, pin)? {
                Pixel::State { decoded, .. } => Blocks::of(&decoded.ic),
                Pixel::Blank(_) => return Ok(blank(End::B)),
            },
        };
        let (base, tie) = match holds {
            true => (xa, false),
            false => anchor::slid(&xa, &xb, self.s),
        };
        if at.map(f64::to_bits) == pin.map(f64::to_bits) {
            return Ok(match holds && lock.form.select {
                true => Locked::Select(Box::new(anchor::selected(&pinned, &a_pin))),
                false => Locked::Blocks(base, tie),
            });
        }
        let Some(oa) = anchor::end_offsets(&self.a, leaves, pin, xa, at)? else {
            return Ok(blank(End::A));
        };
        let Some(ob) = anchor::end_offsets(&end_b, leaves, pin, xb, at)? else {
            return Ok(blank(End::B));
        };
        let (blocks, _) = anchor::offset_pixel(&oa, &ob, &base, pin, at, self.s, lock.form);
        Ok(Locked::Blocks(blocks, tie))
    }

    // ── Navigation: one pixel-space edit on both ends ──────────────────────────────────────────────────────────────

    /// The pan by `(ds, dt)` of the screen: each end panned by the pan turned by `turn` (the module's note).
    pub fn pan(&self, ds: f64, dt: f64) -> Result<Self, PathError> {
        let (ds, dt) = match self.turn {
            0.0 => (ds, dt),
            turn => {
                let (sin, cos) = (libm::sin(turn), libm::cos(turn));
                (cos * ds - sin * dt, sin * ds + cos * dt)
            }
        };
        Ok(Blend {
            a: pan_end(&self.a, ds, dt)?,
            b: pan_end(&self.b, ds, dt)?,
            ..self.clone()
        }
        .relocked())
    }

    /// The zoom by the factor `k` about the centre: each end's ([`Segment::zoom_factor`]).
    pub fn zoom(&self, k: f64) -> Result<Self, PathError> {
        Ok(Blend {
            a: zoom_end(&self.a, k)?,
            b: zoom_end(&self.b, k)?,
            ..self.clone()
        }
        .relocked())
    }

    /// The turn by `radians` about the centre, added to the blend's own.
    pub fn turned(&self, radians: f64) -> Result<Self, PathError> {
        crate::nav::finite(&[radians])?;
        Ok(Blend {
            turn: self.turn + radians,
            ..self.clone()
        })
    }
}

/// The pan of an end: a blend's own, or the chart-path object's pan of the view.
fn pan_end(view: &View, ds: f64, dt: f64) -> Result<View, PathError> {
    match &view.map {
        Map::Blend(blend) => Ok(View::blend(blend.pan(ds, dt)?)),
        _ => Ok(Segment::pan(view, ds, dt)?.b().clone()),
    }
}

/// The zoom of an end: a blend's own, or the chart-path object's zoom of the view by the factor.
fn zoom_end(view: &View, k: f64) -> Result<View, PathError> {
    match &view.map {
        Map::Blend(blend) => Ok(View::blend(blend.zoom(k)?)),
        _ => Ok(Segment::zoom_factor(view, k)?.b().clone()),
    }
}

// ── The signed fold indicator (§ 17.11) ────────────────────────────────────────────────────────────────────────────

/// The signed fold indicator of a frame on an `n × n` probe grid (§ 17.11's definition): per probe its sign, `+1`, `−1`,
/// or `0` where it is not defined; the reference probe; and the shares.
#[derive(Clone, Debug, PartialEq)]
pub struct Folds {
    /// The grid's side.
    pub n: usize,
    /// Per probe, row-major from the bottom-left: `σ ∈ {−1, 0, +1}`.
    pub sigma: Vec<i8>,
    /// The reference probe's index.
    pub reference: usize,
    /// The share of the defined probes on the minority side, the sign fewer defined probes carry: the folded share,
    /// at most ½, whichever side the reference falls on.
    pub folded: f64,
    /// The share of all probes where `σ` is not defined (a blank in the stencil, or below the area floor).
    pub undefined: f64,
}

/// The area floor: `σ` is not defined where the frame's tangent plane, projected onto the reference's, keeps less
/// than this share of its area (§ 17.11's definition).
pub const FOLD_FLOOR: f64 = ledger::constants::FOLD_AREA_FLOOR.number();

/// The 2×2 Gram matrix's determinant of two 9-vectors, `‖x‖²‖y‖² − (x·y)²`.
fn gram(x: &[f64; 9], y: &[f64; 9]) -> f64 {
    let dot = |a: &[f64; 9], b: &[f64; 9]| a.iter().zip(b).map(|(p, q)| p * q).sum::<f64>();
    dot(x, x) * dot(y, y) - dot(x, y) * dot(x, y)
}

/// § 17.11's signed fold indicator of `blend` on the `n × n` probe grid of pixel centres (`n ≥ 3`): `J = ∂Y/∂(u, v)`
/// by central differences on the grid (one-sided at its edges), `Y` the forward map's blocks ([`Blocks::vector`]);
/// the reference the probe of largest area `√det(JᵀJ)`, the lowest index on a tie; `E` its tangent plane,
/// orthonormalised; `σ = sign det(EᵀJ)`, not defined where `|det(EᵀJ)| < FOLD_FLOOR·√det(JᵀJ)` or a stencil probe is
/// blank.
pub fn fold_indicator<L: Leaves>(blend: &Blend, leaves: &L, n: usize) -> Result<Folds, PathError> {
    assert!(n >= 3, "the probe grid needs three probes a side");
    let centre = |i: usize| (i as f64 + 0.5) / n as f64;
    let mut y: Vec<Option<[f64; 9]>> = Vec::with_capacity(n * n);
    for j in 0..n {
        for i in 0..n {
            y.push(match blend.forward(leaves, [centre(i), centre(j)])? {
                Forward::Point { blocks, .. } => Some(blocks.vector()),
                Forward::Blank(_) => None,
            });
        }
    }
    let step = 1.0 / n as f64;
    let diff = |lo: usize, hi: usize, span: f64| -> Option<[f64; 9]> {
        let (a, b) = (y[lo]?, y[hi]?);
        Some(std::array::from_fn(|k| (b[k] - a[k]) / span))
    };
    let jacobian = |i: usize, j: usize| -> Option<([f64; 9], [f64; 9])> {
        let at = |i: usize, j: usize| j * n + i;
        let (il, ih) = (i.saturating_sub(1), (i + 1).min(n - 1));
        let (jl, jh) = (j.saturating_sub(1), (j + 1).min(n - 1));
        let ju = diff(at(il, j), at(ih, j), (ih - il) as f64 * step)?;
        let jv = diff(at(i, jl), at(i, jh), (jh - jl) as f64 * step)?;
        Some((ju, jv))
    };
    let jacobians: Vec<_> = (0..n * n).map(|k| jacobian(k % n, k / n)).collect();
    let area = |k: usize| jacobians[k].map_or(-1.0, |(u, v)| gram(&u, &v).max(0.0).sqrt());
    let reference = (0..n * n).fold(0, |best, k| if area(k) > area(best) { k } else { best });
    let (e1, e2) = match jacobians[reference] {
        Some((u, v)) if area(reference) > 0.0 => {
            let norm = |x: &[f64; 9]| x.iter().map(|c| c * c).sum::<f64>().sqrt();
            let e1 = u.map(|c| c / norm(&u));
            let along = v.iter().zip(&e1).map(|(a, b)| a * b).sum::<f64>();
            let rest: [f64; 9] = std::array::from_fn(|k| v[k] - along * e1[k]);
            (e1, rest.map(|c| c / norm(&rest)))
        }
        _ => {
            return Ok(Folds {
                n,
                sigma: vec![0; n * n],
                reference,
                folded: 0.0,
                undefined: 1.0,
            })
        }
    };
    let dot = |a: &[f64; 9], b: &[f64; 9]| a.iter().zip(b).map(|(p, q)| p * q).sum::<f64>();
    let sigma: Vec<i8> = jacobians
        .iter()
        .map(|j| match j {
            None => 0,
            Some((u, v)) => {
                let det = dot(&e1, u) * dot(&e2, v) - dot(&e1, v) * dot(&e2, u);
                let floor = FOLD_FLOOR * gram(u, v).max(0.0).sqrt();
                match det.abs() < floor || det.abs() == 0.0 {
                    true => 0,
                    false => det.signum() as i8,
                }
            }
        })
        .collect();
    let defined = sigma.iter().filter(|s| **s != 0).count();
    let negative = sigma.iter().filter(|s| **s < 0).count();
    let opposite = negative.min(defined - negative);
    Ok(Folds {
        n,
        folded: match defined {
            0 => 0.0,
            d => opposite as f64 / d as f64,
        },
        undefined: (n * n - defined) as f64 / (n * n) as f64,
        sigma,
        reference,
    })
}
