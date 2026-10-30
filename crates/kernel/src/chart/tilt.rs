//! Tilt and slice on the charts that hold their blocks by latent controls (R-473 T1 to T6, J1 to J8; R-474 K1 to K3;
//! R-507 U1, U2; R-554, ruling RQ-471 (a); R-633's split; chart_decoder_contract Part 4, § "Tilt (basis edit)", "Tilt
//! on every chart").
//!
//! **The rotation** ([`TiltSpec::columns`]). The stored angles `τ₁`, `τ₂` (degrees, as set), the targets `X`, `Y` and
//! the map give `R`'s two columns `R·e_a` and `R·e_b` in the space `(a, b, d_X, d_Y)`:
//! - **Lean each axis** ([`TiltMap::Lean`]): `û₁ = cos τ₁ e_a + sin τ₁ e_X`, `û₂ = cos τ₂ e_b + sin τ₂ e_Y`; with
//!   `X ≠ Y` it is the contract's two-target form, `(cos τ₁·u, cos τ₂·v, sin τ₁·u, sin τ₂·v)`, which both maps give
//!   (R-474 R2, K2).
//! - **Tip as sheet** ([`TiltMap::Sheet`], the default), with one target and both angles set: `m = √(τ₁² + τ₂²)`, the
//!   rotation by `m` about the in-plane axis `k = (τ₂ e_a − τ₁ e_b)/m` (Rodrigues, R-474 K1). Past `m = 180°` the angles
//!   are wrapped in total tip, `τ·(m − 360)/m`, and the one path then runs on the wrapped angles, so a view stored at
//!   `m` and one stored at the wrap's output turn alike, bit for bit; never folded onto the 180° partner (R-474 R3,
//!   K4). A single tilt is the same under both maps (R2), so it takes Lean's closed form under either, and the two give
//!   bitwise the same columns there.
//!
//! **On a curved chart** ([`Tilted`]; R-473 T2, J3, J5; R-507 U1). A pixel at chart point `(u, v)` is offset from the
//! view's centre `c` by `(ũ, ṽ) = (u − c_u, v − c_v)`. The chart's `[0,1]` along each axis is the parameter span its
//! preset's window covers at zoom 1, so `(ũ, ṽ)` is already in units of each axis's full range at zoom 1 and its scale
//! takes no arithmetic. `(Δa, Δb, d̃_X, d̃_Y) = ũ·R·e_a + ṽ·R·e_b`; the chart's own parameters are `(a, b) = c + (Δa, Δb)`,
//! through its map exactly, and each hidden offset is scaled back by its target's slider span, `d = span·d̃`, and added
//! to that hidden coordinate's value at the slice centre ([`Shift`], [`Tiltable::decode_shifted`]). No unit reads the
//! zoom: a zoom changes only which `(u, v)` the window shows. The centre pixel has `(ũ, ṽ) = (0, 0)`, so `(a, b) = c`
//! and `d = 0` there: the tilt is anchor-preserving (J7). `R`, the hidden indices and the spans are the chart's params,
//! computed on the CPU from the stored angles and the map ([`Tilt::new`]); a tilt edits them, a sim key (R-92).
//!
//! **Untilted, no tilt path** (J6, C2). At `τ₁ = τ₂ = 0` under either map [`Tilt::on`] is false, and every entry of
//! [`Tilted`] is the chart's own, bit for bit, with no arithmetic of the tilt's.
//!
//! **Hidden coordinates** ([`Hidden`], [`Tiltable`]; R-473 J1, J2, J8; R-633). On these charts a hidden coordinate is a
//! latent control of a block the axes do not write, or on a mixed-axis chart a control its axes leave frozen, named by
//! its index in the decoder's block layout (`0..8`): the shape sphere's four momentum controls and two mass logits, an
//! invariant chart's two configuration controls and two mass logits, a mixed chart's six frozen controls. Its `d` is
//! added to that control, the link's pre-image, never to the physical value, so a mass tilt keeps every pixel's masses
//! positive and summing to 1 (T3, C7). The index is the chart's to read, so a chart whose hidden coordinate is a
//! physical value (R-633's construction charts, R-554 L5's `p_λ`) names it with an index of its own and adds `d` to that
//! value in its own [`Tiltable::decode_shifted`]: the path is this one. Slicing sets a hidden coordinate's value at the
//! centre ([`Hidden::sliced`], T6); on these charts every value of a latent control is a system, so no slice reaches an
//! edge.
//!
//! **On the latent chart** the tilt is the decoder's basis edit, with no scale (T2; R-507 U2): [`latent_basis`], the
//! columns placed on the axes `e_a`, `e_b` and the targets, turned by `γ` and scaled by `σ` as the GUI's `View::basis`
//! does (R-474 K1).
//!
//! **Past the square** (R-472 X3, X3a; R-512). The chart's own map and `validate` run on the tilted `(a, b)`, so its
//! axes' types act there exactly as on an untilted chart, and nothing is written per chart; `d` needs no type. A repeat
//! (past a period, over a pole) is the same system only where the type group's map leaves `d` unchanged, decided
//! exactly on the stored angles ([`TiltSpec::normal_zero`]; R-554 L3): past θ's period iff `N_a = 0`, at every `b`,
//! the spherical map repeating in `s` for every `t`; over a pole iff `N_a = 0` and `N_b = 0`. Elsewhere the view is
//! affine along that axis, and the shape sphere's pixel there is a system of its own ([`Tilted::pixel`]).
//!
//! **The rank-losing corner** (R-554, ruling RQ-471 (a); R-474 R7). Under Lean each axis with one target at
//! `|τ₁| = |τ₂| = 90°`, decided exactly on the stored angles, the window maps onto one line of systems: the chart
//! reports `ray-degenerate`, applies no fold, weights every pixel's area 0, and every share is undefined ([`share`]).
//! Elsewhere the per-pixel area weight is Lean's Gram factor `√(|û₁|²|û₂|² − (û₁·û₂)²)` with one target, and 1 under
//! Tip as sheet or with two targets, whose bases are orthonormal. The shape sphere has no fold at any tilt (R-544,
//! R-613): its tilted view is bijective, each of its pixels counted once ([`Tilted::counted_total`]).

use spirv_std::num_traits::{Float, FloatConst};

use super::shape_sphere::{PixelCount, ShapeMap, ShapeSphere};
use super::{Chart, ChartDecode, ChartOut, SystemImage, ValidationResult};
use crate::decode::{self, Decoded};
use crate::generated::links::{LinkParams, LinkReal, Selection};
use crate::payload::PayloadReal;

/// `|τ|` at the dead zone of Lean each axis, in degrees: the register's `tilt_right_angle` (R-474 R8).
const RIGHT_ANGLE: f64 = crate::generated::constants::TILT_RIGHT_ANGLE;
/// The total tip past which Tip as sheet wraps, in degrees: the register's `tilt_half_turn` (R-474 R3).
const HALF_TURN: f64 = crate::generated::constants::TILT_HALF_TURN;
/// A whole turn, in degrees: the register's `tilt_turn` (R-474 R3).
const TURN: f64 = crate::generated::constants::TILT_TURN;

/// The tilt map with one target and both angles set (R-474 R1, R5): recorded as `tilt_map=sheet|lean`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TiltMap {
    /// "Tip as sheet", the nav cube's axis-angle map, the default.
    #[default]
    Sheet,
    /// "Lean each axis", each screen axis a mix of its own coordinate and its target.
    Lean,
}

/// A view's stored tilt: the angles as set, in degrees, the map, and the hidden coordinates `τ₁` and `τ₂` tilt into.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TiltSpec {
    /// `τ₁`, tilting the horizontal axis into `x`, in degrees.
    pub tau1: f64,
    /// `τ₂`, tilting the vertical axis into `y`, in degrees.
    pub tau2: f64,
    /// The map.
    pub map: TiltMap,
    /// `τ₁`'s target, a hidden coordinate's index.
    pub x: u32,
    /// `τ₂`'s target.
    pub y: u32,
}

impl TiltSpec {
    /// Whether both stored angles are 0: the chart takes no tilt path (J6).
    #[inline]
    pub fn untilted(&self) -> bool {
        self.tau1 == 0.0 && self.tau2 == 0.0
    }

    /// Whether each angle has its own target, `X ≠ Y`.
    #[inline]
    pub fn two_targets(&self) -> bool {
        self.x != self.y
    }

    /// RQ-471's corner, decided exactly on the stored angles: Lean each axis, one target, `|τ₁| = |τ₂| = 90°`.
    #[inline]
    pub fn rank_loss(&self) -> bool {
        self.map == TiltMap::Lean
            && !self.two_targets()
            && Float::abs(self.tau1) == RIGHT_ANGLE
            && Float::abs(self.tau2) == RIGHT_ANGLE
    }

    /// `R·e_a` and `R·e_b` in `(a, b, d_X, d_Y)` (the module's note): Tip as sheet's rotation only with one target and
    /// both angles set, Lean's closed form otherwise.
    pub fn columns(&self) -> [[f64; 4]; 2] {
        let sheet = self.map == TiltMap::Sheet
            && !self.two_targets()
            && self.tau1 != 0.0
            && self.tau2 != 0.0;
        if sheet {
            tip_as_sheet(self.tau1, self.tau2)
        } else {
            lean(self.tau1, self.tau2, self.two_targets())
        }
    }

    /// `[N_a = 0, N_b = 0]` for chart_reference §3.3's normal `N ∝ (R·e_a) × (R·e_b)`, decided exactly on the stored
    /// angles from each map's closed form (R-554 L3): whether the view's hidden offsets keep their value as `a`, or `b`,
    /// moves, so that a repeat along that axis is the same system.
    /// - Untilted, `[true, true]`.
    /// - At RQ-471's corner, `[false, false]`: there is no normal, and no fold is applied (R-554, ruling RQ-471 (a)).
    /// - With two targets, per hidden offset, `d_X = sin τ₁·u` and `d_Y = sin τ₂·v`: `[sin τ₁ = 0, sin τ₂ = 0]`, each
    ///   angle in `{0, ±180}`.
    /// - Tip as sheet with one target and both angles set, `N_a = −τ₁ sin m/m` and `N_b = −τ₂ sin m/m`: both at
    ///   `m == 180` exactly, neither elsewhere.
    /// - Otherwise Lean's closed form, which a single angle takes under either map (R-474 R2):
    ///   `N ∝ (−sin τ₁ cos τ₂, −cos τ₁ sin τ₂, cos τ₁ cos τ₂)`.
    pub fn normal_zero(&self) -> [bool; 2] {
        let sin_zero = |t: f64| t == 0.0 || Float::abs(t) == HALF_TURN;
        let cos_zero = |t: f64| Float::abs(t) == RIGHT_ANGLE;
        let (t1, t2) = (self.tau1, self.tau2);
        if self.rank_loss() {
            [false, false]
        } else if self.two_targets() {
            [sin_zero(t1), sin_zero(t2)]
        } else if self.map == TiltMap::Sheet && t1 != 0.0 && t2 != 0.0 {
            let half = Float::sqrt(t1 * t1 + t2 * t2) == HALF_TURN;
            [half, half]
        } else {
            [sin_zero(t1) || cos_zero(t2), cos_zero(t1) || sin_zero(t2)]
        }
    }

    /// Each pixel's area weight: 0 at the corner, Lean's Gram factor with one target, 1 otherwise (R-474 R7; R-554).
    pub fn area_weight(&self) -> f64 {
        if self.rank_loss() {
            0.0
        } else if self.map == TiltMap::Lean && !self.two_targets() {
            gram(self.columns())
        } else {
            1.0
        }
    }

    /// The tilted view's `system_image`: `ray-degenerate` at the corner, else the untilted chart's `untilted`.
    #[inline]
    pub fn system_image(&self, untilted: SystemImage) -> SystemImage {
        image(self.rank_loss(), untilted)
    }
}

/// `ray-degenerate` where the view loses rank, else `untilted`.
#[inline]
fn image(rank_loss: bool, untilted: SystemImage) -> SystemImage {
    if rank_loss {
        SystemImage::RayDegenerate
    } else {
        untilted
    }
}

/// Lean each axis's columns at the angles in degrees, with one target or two.
fn lean(tau1: f64, tau2: f64, two: bool) -> [[f64; 4]; 2] {
    let (s1, c1) = Float::sin_cos(Float::to_radians(tau1));
    let (s2, c2) = Float::sin_cos(Float::to_radians(tau2));
    if two {
        [[c1, 0.0, s1, 0.0], [0.0, c2, 0.0, s2]]
    } else {
        [[c1, 0.0, s1, 0.0], [0.0, c2, s2, 0.0]]
    }
}

/// Tip as sheet's columns at the angles in degrees, both non-zero: the angles wrapped in total tip past 180°, then
/// Rodrigues' rotation by `m` about `k = (τ₂ e_a − τ₁ e_b)/m`, `v ↦ v cos m + (k × v) sin m + k (k·v)(1 − cos m)`.
fn tip_as_sheet(tau1: f64, tau2: f64) -> [[f64; 4]; 2] {
    let total = Float::sqrt(tau1 * tau1 + tau2 * tau2);
    let wrap = (total - TURN) / total;
    let (t1, t2) = if total > HALF_TURN {
        (tau1 * wrap, tau2 * wrap)
    } else {
        (tau1, tau2)
    };
    let m = Float::sqrt(t1 * t1 + t2 * t2);
    let (sm, cm) = Float::sin_cos(Float::to_radians(m));
    let (ka, kb) = (t2 / m, -t1 / m);
    let w = 1.0 - cm;
    [
        [cm + ka * ka * w, ka * kb * w, -kb * sm, 0.0],
        [ka * kb * w, cm + kb * kb * w, ka * sm, 0.0],
    ]
}

/// The Gram factor of one-target Lean's columns, `√(|û₁|²|û₂|² − (û₁·û₂)²)`, taken as `|û₁ × û₂|` (Lagrange's
/// identity), which no rounding makes negative. The columns are `û₁ = (cos τ₁, 0, sin τ₁)` and
/// `û₂ = (0, cos τ₂, sin τ₂)` in `(a, b, d_X)`, so the cross product's components are `−û₁[d]û₂[b]`, `−û₁[a]û₂[d]` and
/// `û₁[a]û₂[b]`.
fn gram([u1, u2]: [[f64; 4]; 2]) -> f64 {
    let x = u1[2] * u2[1];
    let y = u1[0] * u2[2];
    let z = u1[0] * u2[1];
    Float::sqrt(x * x + y * y + z * z)
}

/// A share of an area statistic: `part` of `total`, undefined (`None`) where the total is 0, never 0 (R-554 L2).
#[inline]
pub fn share(part: f64, total: f64) -> Option<f64> {
    if total == 0.0 {
        None
    } else {
        Some(part / total)
    }
}

// ── The latent chart: the basis edit ──────────────────────────────────────────────────────────────────────────────────

/// A latent view's untilted frame, as the GUI's `View` holds it: the axes `e_a`, `e_b`, the rotation `γ` (radians) and
/// the zoom's scale `σ`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LatentFrame {
    /// The horizontal axis's latent index `a`.
    pub a: u32,
    /// The vertical axis's latent index `b`.
    pub b: u32,
    /// The scale `σ`.
    pub scale: f64,
    /// The rotation `γ`, in radians.
    pub gamma: f64,
}

impl LatentFrame {
    /// Whether the latent dimension `k` is a hidden coordinate of the frame: one of the six off its axes (R-457).
    #[inline]
    pub fn hidden(&self, k: u32) -> bool {
        k < 8 && k != self.a && k != self.b
    }

    /// The latent chart sliced along the hidden dimension `k` to `value`: `z₀`'s component `k` set, nothing else; `None`
    /// where `k` is not hidden (T6).
    pub fn sliced(
        &self,
        chart: &super::Latent<f64>,
        k: u32,
        value: f64,
    ) -> Option<super::Latent<f64>> {
        self.hidden(k).then(|| {
            let mut z0 = chart.z0;
            z0[k as usize] = value;
            super::Latent { z0, ..*chart }
        })
    }
}

/// The tilted basis `(q₁, q₂)` of a latent view (T2, the basis edit; R-474 K1): `û₁ = R e_a`, `û₂ = R e_b` placed on
/// `e_a`, `e_b` and the targets, then `q₁ = σ(cos γ û₁ + sin γ û₂)` and `q₂ = σ(cos γ û₂ − sin γ û₁)`, as the GUI's
/// `View::basis`. `None` where a target is not one of the frame's hidden dimensions.
pub fn latent_basis(frame: &LatentFrame, spec: &TiltSpec) -> Option<([f64; 8], [f64; 8])> {
    if !(frame.hidden(spec.x)
        && frame.hidden(spec.y)
        && frame.a < 8
        && frame.b < 8
        && frame.a != frame.b)
    {
        return None;
    }
    let place = |c: [f64; 4]| {
        let mut u = [0.0; 8];
        u[frame.a as usize] += c[0];
        u[frame.b as usize] += c[1];
        u[spec.x as usize] += c[2];
        u[spec.y as usize] += c[3];
        u
    };
    let [c1, c2] = spec.columns();
    let (u1, u2) = (place(c1), place(c2));
    let (sg, cg) = Float::sin_cos(frame.gamma);
    let s = frame.scale;
    let mut q1 = [0.0; 8];
    let mut q2 = [0.0; 8];
    for i in 0..8 {
        q1[i] = u1[i] * cg * s + sg * s * u2[i];
        q2[i] = u2[i] * cg * s + -sg * s * u1[i];
    }
    Some((q1, q2))
}

// ── A curved chart's tilt ─────────────────────────────────────────────────────────────────────────────────────────────

/// A curved chart's tilt params (J5): computed on the CPU from the stored angles and the map, sent with the chart's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tilt<P> {
    /// Whether a tilt path is taken: false at `τ₁ = τ₂ = 0`, where the chart is its own (J6).
    pub on: bool,
    /// Whether `x` and `y` are two targets, each with its own offset.
    pub two: bool,
    /// Whether the view loses rank, RQ-471's corner ([`TiltSpec::rank_loss`]).
    pub rank_loss: bool,
    /// The view's centre `c`, in the chart's own parameters.
    pub centre: [P; 2],
    /// `R·e_a` and `R·e_b` in `(a, b, d_X, d_Y)` ([`TiltSpec::columns`]).
    pub columns: [[P; 4]; 2],
    /// `τ₁`'s target.
    pub x: u32,
    /// `τ₂`'s target.
    pub y: u32,
    /// The targets' slider spans: each hidden offset's unit (R-507 U1).
    pub span: [P; 2],
    /// Each pixel's area weight ([`TiltSpec::area_weight`]).
    pub weight: P,
    /// `[N_a = 0, N_b = 0]` ([`TiltSpec::normal_zero`]): where a repeat along each axis is the same system.
    pub normal_zero: [bool; 2],
}

impl Tilt<f64> {
    /// The params of the stored tilt `spec` about the view's centre `centre`, the targets' slider spans `span`.
    pub fn new(spec: &TiltSpec, centre: [f64; 2], span: [f64; 2]) -> Self {
        Tilt {
            on: !spec.untilted(),
            two: spec.two_targets(),
            rank_loss: spec.rank_loss(),
            centre,
            columns: spec.columns(),
            x: spec.x,
            y: spec.y,
            span,
            weight: spec.area_weight(),
            normal_zero: spec.normal_zero(),
        }
    }
}

/// The hidden offsets of one pixel: `dx` added to the hidden coordinate `x`, and with two targets `dy` to `y`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shift<R> {
    /// `τ₁`'s target.
    pub x: u32,
    /// Its offset, in its own units.
    pub dx: R,
    /// `τ₂`'s target.
    pub y: u32,
    /// Its offset, added only with two targets.
    pub dy: R,
    /// Whether `y` is a second target.
    pub two: bool,
}

impl<P: Copy> Tilt<P> {
    /// `(Δa, Δb, d_X, d_Y) = ũ·R·e_a + ṽ·R·e_b` for the offsets `(ũ, ṽ)` from the centre, each hidden offset scaled by
    /// its slider span (R-507 U1).
    #[inline]
    pub fn offsets<R: Float + From<P>>(&self, du: R, dv: R) -> [R; 4] {
        let w = <R as From<P>>::from;
        let [c1, c2] = self.columns;
        [
            w(c1[0]) * du + w(c2[0]) * dv,
            w(c1[1]) * du + w(c2[1]) * dv,
            w(self.span[0]) * (w(c1[2]) * du + w(c2[2]) * dv),
            w(self.span[1]) * (w(c1[3]) * du + w(c2[3]) * dv),
        ]
    }

    /// The tilted chart point `(a, b)` of the pixel at `(u, v)` and its hidden offsets.
    #[inline]
    pub fn place<R: Float + From<P>>(&self, u: R, v: R) -> ([R; 2], Shift<R>) {
        let w = <R as From<P>>::from;
        let (cu, cv) = (w(self.centre[0]), w(self.centre[1]));
        let [da, db, dx, dy] = self.offsets(u - cu, v - cv);
        (
            [cu + da, cv + db],
            Shift {
                x: self.x,
                dx,
                y: self.y,
                dy,
                two: self.two,
            },
        )
    }

    /// The tilted view's `system_image`: `ray-degenerate` at the corner, else the untilted chart's `untilted`.
    #[inline]
    pub fn system_image(&self, untilted: SystemImage) -> SystemImage {
        image(self.rank_loss, untilted)
    }
}

/// One latent control `z`, index `k`, with `shift`'s offsets added where it is a target.
#[inline]
fn shifted<R: Float>(z: R, k: u32, shift: &Shift<R>) -> R {
    let z = if shift.x == k { z + shift.dx } else { z };
    if shift.two && shift.y == k {
        z + shift.dy
    } else {
        z
    }
}

/// The latent controls `z` with `shift`'s offsets added to its targets, each read by a literal index
/// (gpu_determinism_note rule 5); an index past the controls changes none.
#[inline]
pub fn shift_controls<R: Float>(z: [R; 8], shift: &Shift<R>) -> [R; 8] {
    [
        shifted(z[0], 0, shift),
        shifted(z[1], 1, shift),
        shifted(z[2], 2, shift),
        shifted(z[3], 3, shift),
        shifted(z[4], 4, shift),
        shifted(z[5], 5, shift),
        shifted(z[6], 6, shift),
        shifted(z[7], 7, shift),
    ]
}

/// A chart's hidden coordinates and its slice (R-473 J1, T6; the module's note).
pub trait Hidden: Chart + Sized {
    /// Whether the index `k` names one of the chart's hidden coordinates.
    fn hidden(&self, k: u32) -> bool;

    /// The chart with the hidden coordinate `k` set to `value` at the slice centre, nothing else changed; `None` where
    /// `k` is not hidden.
    fn sliced(&self, k: u32, value: Self::Param) -> Option<Self>;
}

/// A chart the tilt path runs on: its decode at its own parameters with the hidden offsets added (the module's note).
pub trait Tiltable: Hidden {
    /// `D ∘ Φ` at `(u, v)` with `shift` added to the hidden coordinates at the slice centre, at the tunable parameters
    /// `p`; at a zero shift the chart's own decode.
    fn decode_shifted<S, R>(&self, u: R, v: R, shift: &Shift<R>, p: &LinkParams<R>) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<Self::Param>;
}

/// A chart and its tilt (the module's note): pixel `(u, v)` decodes at the tilted `(a, b)` with the hidden offsets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tilted<C, P> {
    chart: C,
    tilt: Tilt<P>,
}

/// Whether the index `k` names a mass control.
#[inline]
fn mass(k: u32) -> bool {
    k == 6 || k == 7
}

impl<C: Tiltable<Param = P>, P: Copy> Tilted<C, P> {
    /// `chart` under `tilt`; `None` where a tilt is on and a target is not one of the chart's hidden coordinates.
    pub fn new(chart: C, tilt: Tilt<P>) -> Option<Self> {
        let targets = chart.hidden(tilt.x) && chart.hidden(tilt.y);
        (!tilt.on || targets).then_some(Tilted { chart, tilt })
    }

    /// The untilted chart.
    #[inline]
    pub fn chart(&self) -> &C {
        &self.chart
    }

    /// The tilt.
    #[inline]
    pub fn tilt(&self) -> &Tilt<P> {
        &self.tilt
    }

    /// The chart point `(a, b)` of the pixel at `(u, v)` and its hidden offsets; the pixel's own point, with none,
    /// untilted.
    #[inline]
    pub fn at<R: Float + From<P>>(&self, u: R, v: R) -> ([R; 2], Option<Shift<R>>) {
        if self.tilt.on {
            let (ab, shift) = self.tilt.place(u, v);
            (ab, Some(shift))
        } else {
            ([u, v], None)
        }
    }
}

impl<C: Tiltable<Param = P>, P: Copy> Chart for Tilted<C, P>
where
    f64: From<P>,
{
    type Param = P;

    /// The chart's output at the tilted `(a, b)`, its latent controls shifted.
    #[inline]
    fn map<F: Float + From<P>>(&self, u: F, v: F) -> ChartOut<F> {
        match self.at(u, v) {
            ([a, b], Some(shift)) => ChartOut {
                z: decode::Latent::new(shift_controls(self.chart.map(a, b).z.z, &shift)),
            },
            (_, None) => self.chart.map(u, v),
        }
    }

    fn forbids_energy_normalisation(&self) -> bool {
        self.chart.forbids_energy_normalisation()
    }

    fn name(&self) -> &str {
        self.chart.name()
    }

    /// The chart's, or true where a tilt moves a mass control, so the masses vary per pixel.
    fn requires_per_pixel_mass(&self) -> bool {
        let tilted = mass(self.tilt.x) || (self.tilt.two && mass(self.tilt.y));
        self.chart.requires_per_pixel_mass() || (self.tilt.on && tilted)
    }

    /// The chart's check on the tilted `(a, b)`, where its axes' types act (X3a); a failure is reported as refused,
    /// since a projected or clamped `(a, b)` names no point of the tilted view.
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        self.validate_params(u, v, &LinkParams::DEFAULT)
    }

    /// [`Chart::validate`] at the tunable parameters `p`.
    fn validate_params(&self, u: f64, v: f64, p: &LinkParams<f64>) -> ValidationResult {
        match self.at(u, v) {
            (_, None) => self.chart.validate_params(u, v, p),
            ([a, b], Some(_)) => match self.chart.validate_params(a, b, p) {
                ValidationResult::Valid => ValidationResult::Valid,
                ValidationResult::Project { failed, .. }
                | ValidationResult::Clamp { failed, .. }
                | ValidationResult::Reject { failed } => ValidationResult::Reject { failed },
            },
        }
    }

    /// The chart's own decode untilted; else [`Chart::decode_params`] at the defaults.
    #[inline]
    fn decode<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        if self.tilt.on {
            self.decode_params::<S, R>(u, v, &LinkParams::DEFAULT)
        } else {
            self.chart.decode::<S, R>(u, v)
        }
    }

    /// The chart's decode at the tilted `(a, b)` with the hidden offsets ([`Tiltable::decode_shifted`]); the chart's
    /// own untilted.
    #[inline]
    fn decode_params<S, R>(&self, u: R, v: R, p: &LinkParams<R>) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        match self.at(u, v) {
            ([a, b], Some(shift)) => self.chart.decode_shifted::<S, R>(a, b, &shift, p),
            (_, None) => self.chart.decode_params::<S, R>(u, v, p),
        }
    }
}

impl<C: Tiltable<Param = P>, P: Copy> ChartDecode for Tilted<C, P>
where
    f64: From<P>,
{
    #[inline]
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + super::invariant::WarpReal + From<P>,
    {
        Chart::decode::<S, R>(self, u, v)
    }

    #[inline]
    fn decode_at_params<S, R>(&self, u: R, v: R, p: &LinkParams<R>) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + super::invariant::WarpReal + From<P>,
    {
        Chart::decode_params::<S, R>(self, u, v, p)
    }
}

// ── The tilted shape sphere: one-to-one at every tilt (R-544, R-613) ──────────────────────────────────────────────────

impl<P: Copy + FloatConst> Tilted<ShapeSphere<P>, P>
where
    f64: From<P>,
{
    /// The tilted view's `system_image`: bijective on either map at every tilt, no mirror pair folded (R-544, R-613),
    /// `ray-degenerate` at RQ-471's corner.
    pub fn system_image(&self) -> SystemImage {
        self.tilt.system_image(self.chart.system_image())
    }

    /// How the tilted view's pixel at `(u, v)` counts, from its tilted `(a, b)`.
    /// - Inside the chart's square the spherical map's point is its own system, and the exponential map's counts as its
    ///   cap says.
    /// - Past the square the exponential map's bounded axes end the domain (R-512).
    /// - Past the square on the spherical map, the pixel is a repeat, not counted (`None`), only where the type group's
    ///   map leaves the hidden offset `d` unchanged (chart_decoder_contract § "Repeats and the canonical point"; R-554
    ///   L3): past θ's period, iff `N_a = 0`, at every `b`, the map being periodic in `s` for every `t`; over a pole,
    ///   iff `N_a = 0` and `N_b = 0`, decided exactly on the stored angles ([`TiltSpec::normal_zero`]). Elsewhere the
    ///   view is affine along that axis, its primary range unbounded, and the pixel is a system of its own, counted
    ///   (R-512 F1, F3). A repeat `(s + k, t + 2j)` that keeps `d` while `N_a` and `N_b` are not both 0 is counted, as
    ///   the ruled rule says (RQ-631).
    pub fn pixel(&self, u: f64, v: f64) -> Option<PixelCount> {
        let ([a, b], _) = self.at(u, v);
        let a_in = (0.0..=1.0).contains(&a);
        let b_in = (0.0..=1.0).contains(&b);
        let [na, nb] = self.tilt.normal_zero;
        match self.chart.map {
            ShapeMap::Spherical => {
                let repeat = (!a_in && na) || (!b_in && na && nb);
                (!repeat).then_some(PixelCount::System)
            }
            ShapeMap::Exponential if a_in && b_in => Some(self.chart.exp_pixel(a, b)),
            ShapeMap::Exponential => Some(PixelCount::DomainEnd),
        }
    }

    /// The counted total over the pixels at `points`: each system pixel once, no pixel folded onto another, a repeat
    /// that keeps `d` not at all ([`Tilted::pixel`]), and the pixels on `r = π`, the one system `−n₀`, once together
    /// (REQ-CHART-078).
    pub fn counted_total<I: IntoIterator<Item = [f64; 2]>>(&self, points: I) -> u64 {
        let mut total = 0u64;
        let mut antipode = false;
        for [u, v] in points {
            match self.pixel(u, v) {
                Some(PixelCount::System) => total += 1,
                Some(PixelCount::Antipode) => antipode = true,
                Some(PixelCount::DomainEnd) | None => {}
            }
        }
        total + u64::from(antipode)
    }
}
