//! The round trip `z → D → E → D` in physical units (inverse-encode contract Parts 1, 4 and 8; dd_encode §3.5, §5
//! tests 1 and 8; dd_decoder §5 test 7; TASK-M2-16), shared by the kernel's round-trip suite (`kernel/tests/
//! roundtrip.rs`) and the `encode-eps-phys` gate ([`crate::eps_phys`]):
//!
//! - [`phys`], the norm `‖·‖_phys` of inverse-encode Part 4 (REQ-ENC-029): over the masses, positions and momenta of two
//!   states on the canonical section, the largest of `|Δmᵢ|`, `‖Δrᵢ‖/R_x` and `‖Δpᵢ‖/P_x`, `R_x` and `P_x` the reference
//!   state's extent in positions and momenta, floored at 1, in `G = M = I = 1`;
//! - [`eps_phys`], the tolerance `ε_phys = k·ε_R` (REQ-ENC-024): the register's `eps_phys`, `k`, times the precision's
//!   machine epsilon;
//! - [`trip`], one round trip: `E` on a decoded state, then the latent decode of its `z`, and the residual;
//! - [`margins`] and [`Sides`], the clamp oracle: how far each of the eight controls lies inside its clamp, read off
//!   the decoded state by Part 3's own formulas, never by the encode's code, so a sample's clamp-adjacency is known
//!   independently of the `lookup_clamped` it is checked against;
//! - the chart registry ([`registered_charts!`](crate::registered_charts), [`registry`]): one representative of every
//!   chart `engine::chart_authoring` registers, validated, at f64 and at f32, so a chart added to it gets its T2 test
//!   and its row in the gate.
//!
//! **Where `E` lands** (Part 6 step 4): into the active chart where possible, otherwise into latent `z`, "always
//! possible via Part 3". The T2 round trip here is the latent one on every chart: the chart's own decode gives `D(·)`,
//! the block inverses give `z`, and the shared free decode gives `D(z)`. A chart's decoded state is canonical (decoder
//! dd §2: decoded states are already canonical), so `E`'s `C` is a no-op on it to rounding, and T2 exercises the block
//! inverses and their numerics (Part 1, T2).
//!
//! **The link set** is the registry's defaults ([`Defaults`]) throughout: the clamp oracle's formulas are Part 3's for
//! those links (`s_α = 2α/π`, `s_β = β/π`, `s_q = ½(q/q_max + 1)` into `[ε, 1 − ε]`, and `|μₖ| ≤ (1 − ε_μ)·μ_max`).

use engine::chart_authoring::{validate, Authored, ValidatedChart};
use kernel::chart::invariant::{Invariant, WarpReal};
use kernel::chart::mixed::{Alpha, PRhoX, ZMu1, ZQ2};
use kernel::chart::{
    Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, EuclidPlane, InvariantLE, InvariantLK,
    Latent, MassSimplex, Mixed, ShapeMap, ShapeSphere,
};
use kernel::decode::{self, Decoded, Ic};
use kernel::encode::{encode, Encoded, SimplexClamp};
use kernel::generated::links::{Defaults, LinkReal, Selection};
use kernel::payload::PayloadReal;
use kernel::real::IeeeBits;
use ledger::constants::{EPS_MU, EPS_PHYS, EPS_Q, EPS_Z, MU_MAX, Q_MAX};

/// A precision the round trip runs at: the shared source's two instantiations, f64 on the CPU and f32 as the kernel's.
pub trait Precision: LinkReal + IeeeBits + PayloadReal + WarpReal + Into<f64> + Copy {
    /// The precision's machine epsilon, `2⁻⁵²` or `2⁻²³`, as an f64.
    const EPS: f64;
    /// `x` rounded to the precision.
    fn of(x: f64) -> Self;
}

impl Precision for f64 {
    const EPS: f64 = f64::EPSILON;
    fn of(x: f64) -> f64 {
        x
    }
}

impl Precision for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    fn of(x: f64) -> f32 {
        x as f32
    }
}

/// `x` widened to f64, exactly.
pub fn widen<R: Precision>(x: &Ic<R>) -> Ic<f64> {
    Ic {
        m: x.m.map(Into::into),
        r: x.r.map(|v| v.map(Into::into)),
        p: x.p.map(|v| v.map(Into::into)),
    }
}

/// `‖v‖`.
fn norm(v: [f64; 2]) -> f64 {
    v[0].hypot(v[1])
}

/// `‖x′ − x‖_phys` (inverse-encode Part 4, REQ-ENC-029), `x` the reference state and `x′` the one measured against it,
/// both on the canonical section: the largest of `|m′ᵢ − mᵢ|`, `‖r′ᵢ − rᵢ‖/R_x` and `‖p′ᵢ − pᵢ‖/P_x` over the three
/// bodies, with `R_x = max(1, maxⱼ‖rⱼ‖)` and `P_x = max(1, maxⱼ‖pⱼ‖)` the reference's extent in each block. A NaN
/// anywhere gives NaN, which no tolerance admits.
pub fn phys(x: &Ic<f64>, x_prime: &Ic<f64>) -> f64 {
    let extent = |v: &[[f64; 2]; 3]| v.iter().map(|&w| norm(w)).fold(1.0f64, f64::max);
    let (r_x, p_x) = (extent(&x.r), extent(&x.p));
    let mut worst = 0.0f64;
    let mut nan = false;
    for i in 0..3 {
        let d = [
            (x_prime.m[i] - x.m[i]).abs(),
            norm([x_prime.r[i][0] - x.r[i][0], x_prime.r[i][1] - x.r[i][1]]) / r_x,
            norm([x_prime.p[i][0] - x.p[i][0], x_prime.p[i][1] - x.p[i][1]]) / p_x,
        ];
        for e in d {
            nan |= e.is_nan();
            worst = worst.max(e);
        }
    }
    if nan || r_x.is_nan() || p_x.is_nan() {
        f64::NAN
    } else {
        worst
    }
}

/// `ε_phys` at the precision `R` (REQ-ENC-024; inverse-encode Part 4): the register's `eps_phys`, `k`, times `R`'s
/// machine epsilon.
pub fn eps_phys<R: Precision>() -> f64 {
    EPS_PHYS.number() * R::EPS
}

/// One round trip from the decoded state `x`: `E(x)` under `S`, then the free decode of its `z`, and
/// `‖x − D(E(x))‖_phys`.
#[derive(Clone, Copy, Debug)]
pub struct Trip<R: PayloadReal> {
    /// `E(x)`.
    pub encoded: Encoded<R>,
    /// `D(E(x))`.
    pub back: Decoded<R>,
    /// `‖x − D(E(x))‖_phys`, in f64 from `R`'s values.
    pub residual: f64,
}

/// [`Trip`] at `x` under the selection `S`, encode and decode both.
pub fn trip<S, R>(x: &Ic<R>) -> Trip<R>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: Precision,
{
    trip_with::<S, S, R>(x)
}

/// [`Trip`] at `x` with `E` under the selection `E` and `D` under `D`: the round trip with a link swapped on one side
/// only, which T2 must catch (dd_encode §4, seam 2/13: encode's inverse swaps with the decode's link).
pub fn trip_with<E, D, R>(x: &Ic<R>) -> Trip<R>
where
    E: Selection,
    E::MassLink: SimplexClamp,
    D: Selection,
    R: Precision,
{
    let encoded = encode::<E, R>(x);
    let back = decode::decode::<D, R>(&encoded.z);
    Trip {
        encoded,
        back,
        residual: phys(&widen(x), &widen(&back.ic)),
    }
}

// ── The clamp oracle ───────────────────────────────────────────────────────────────────────────────────────────────

/// How far each control of `x` lies inside its clamp, in units of the clamp's own `ε` (Part 3, the defaults links),
/// in the latent block layout `z_α, z_β | z_q0 … z_q3 | z_μ1, z_μ2`: `min(s, 1 − s)/ε` for `α`, `β` and the four `q`,
/// and `(μ_max − |μₖ|)/(ε_μ·μ_max)` for the two masses, so a control is at its clamp at 1, inside above it and past it
/// below. Read off `x`'s own masses, positions and momenta: `ρ = r₁ − r₀`, `λ = r₂ − (m₀r₀ + m₁r₁)/M₀₁`, each weighted
/// by its reduced mass's root, `α = atan2(‖λ̃‖, ‖ρ̃‖)`, `β = atan2(|λ̃_y|, λ̃_x)`, `p_λ = p₂`, `p_ρ = p₁ + (m₁/M₀₁)p₂`,
/// `μₖ = ln(mₖ/m₀)`.
pub fn margins(x: &Ic<f64>) -> [f64; 8] {
    let [m0, m1, m2] = x.m;
    let m01 = m0 + m1;
    let [r0, r1, r2] = x.r;
    let rho = [r1[0] - r0[0], r1[1] - r0[1]];
    let lam = [
        r2[0] - (m0 * r0[0] + m1 * r1[0]) / m01,
        r2[1] - (m0 * r0[1] + m1 * r1[1]) / m01,
    ];
    let (w_rho, w_lam) = ((m0 * m1 / m01).sqrt(), (m2 * m01).sqrt());
    let rho_t = [w_rho * rho[0], w_rho * rho[1]];
    let lam_t = [w_lam * lam[0], w_lam * lam[1]];
    let pi = std::f64::consts::PI;
    let alpha = norm(lam_t).atan2(norm(rho_t));
    let beta = lam_t[1].abs().atan2(lam_t[0]);
    let [_, p1, p2] = x.p;
    let w = m1 / m01;
    let q = [p1[0] + w * p2[0], p1[1] + w * p2[1], p2[0], p2[1]];
    let (eps_z, eps_q, q_max) = (EPS_Z.number(), EPS_Q.number(), Q_MAX.number());
    let unit = |s: f64, eps: f64| s.min(1.0 - s) / eps;
    let mass = |mk: f64| {
        let (mu_max, eps_mu) = (MU_MAX.number(), EPS_MU.number());
        (mu_max - (mk / m0).ln().abs()) / (eps_mu * mu_max)
    };
    [
        unit(2.0 * alpha / pi, eps_z),
        unit(beta / pi, eps_z),
        unit(0.5 * (q[0] / q_max + 1.0), eps_q),
        unit(0.5 * (q[1] / q_max + 1.0), eps_q),
        unit(0.5 * (q[2] / q_max + 1.0), eps_q),
        unit(0.5 * (q[3] / q_max + 1.0), eps_q),
        mass(m1),
        mass(m2),
    ]
}

/// The margin at or above which a control is inside its clamp, clear of the band where `x`'s rounding and the encode's
/// own arithmetic could put it on either side: twice the clamp's `ε`.
pub const INSIDE: f64 = 2.0;

/// The margin at or below which a control is past its clamp: half the clamp's `ε`.
pub const PAST: f64 = 0.5;

/// A sample's controls by side of their clamps, from [`margins`]: bit `k` for `z[k]`, in the encode report's layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sides {
    /// The controls inside their clamps, margin at least [`INSIDE`]: each must leave its clamp bit clear.
    pub inside: u32,
    /// The controls past their clamps, margin at most [`PAST`]: each must set its clamp bit.
    pub past: u32,
}

impl Sides {
    /// The sides of the controls whose margins are `m`; a NaN margin is neither side.
    pub fn of(m: [f64; 8]) -> Sides {
        let bits = |pick: &dyn Fn(f64) -> bool| {
            m.iter()
                .enumerate()
                .filter(|(_, &x)| pick(x))
                .fold(0u32, |b, (k, _)| b | 1 << k)
        };
        Sides {
            inside: bits(&|x| x >= INSIDE),
            past: bits(&|x| x <= PAST),
        }
    }

    /// Every control inside its clamp: an interior sample, held to `ε_phys`.
    pub fn interior(&self) -> bool {
        self.inside == 0xff
    }
}

// ── The chart registry ─────────────────────────────────────────────────────────────────────────────────────────────

/// One registered chart, validated by `engine::chart_authoring::validate`, at f64 and at f32, decoded under the
/// defaults links.
pub trait Registered {
    /// The registry's key, the chart's entry in [`registered_charts!`](crate::registered_charts).
    fn key(&self) -> &'static str;
    /// The chart's type, its last path segment without its parameters (`Latent`, `Mixed`, …).
    fn type_name(&self) -> &'static str;
    /// The chart's `D ∘ Φ` at `(u, v)`, its parameters at f64.
    fn decode64(&self, u: f64, v: f64) -> Decoded<f64>;
    /// The chart's `D ∘ Φ` at `(u, v)`, its parameters at f32.
    fn decode32(&self, u: f32, v: f32) -> Decoded<f32>;
}

/// The decode at `R`: [`Registered::decode64`] or [`Registered::decode32`].
pub trait DecodeAt: Precision {
    /// `chart`'s decode at `(u, v)` at this precision.
    fn decode_at(chart: &dyn Registered, u: f64, v: f64) -> Decoded<Self>;
}

impl DecodeAt for f64 {
    fn decode_at(chart: &dyn Registered, u: f64, v: f64) -> Decoded<f64> {
        chart.decode64(u, v)
    }
}

impl DecodeAt for f32 {
    fn decode_at(chart: &dyn Registered, u: f64, v: f64) -> Decoded<f32> {
        chart.decode32(u as f32, v as f32)
    }
}

struct Entry<A, B> {
    key: &'static str,
    c64: ValidatedChart<A>,
    c32: ValidatedChart<B>,
}

/// `T`'s last path segment, its parameters dropped.
fn short_type<T>() -> &'static str {
    let full = std::any::type_name::<T>();
    let bare = full.split('<').next().unwrap_or(full);
    bare.rsplit("::").next().unwrap_or(bare)
}

impl<A, B> Registered for Entry<A, B>
where
    A: Authored<Param = f64>,
    B: Authored<Param = f32>,
{
    fn key(&self) -> &'static str {
        self.key
    }

    fn type_name(&self) -> &'static str {
        short_type::<A>()
    }

    fn decode64(&self, u: f64, v: f64) -> Decoded<f64> {
        self.c64.decode::<Defaults>(u, v)
    }

    fn decode32(&self, u: f32, v: f32) -> Decoded<f32> {
        self.c32.chart().decode::<Defaults, f32>(u, v)
    }
}

/// Every registered chart's key, one representative each, in chart_reference §5.1's order, each the name of its
/// constructor in [`charts`]: `$m!(key)` for each. A chart added here, with its constructor, gets its T2 test in
/// `kernel/tests/roundtrip.rs` and its row in the `encode-eps-phys` gate; `kernel/tests/roundtrip.rs`'s
/// `t2_per_chart_registry_complete` fails while a chart `engine::chart_authoring` registers has no entry.
#[macro_export]
macro_rules! registered_charts {
    ($m:ident) => {
        $m!(latent);
        $m!(shape_sphere);
        $m!(shape_sphere_exponential);
        $m!(burrau);
        $m!(burrau_full);
        $m!(euclid_plane);
        $m!(burrau_theta_k);
        $m!(burrau_theta_lz);
        $m!(burrau_theta_delta_m);
        $m!(invariant_le);
        $m!(invariant_lk);
        $m!(mass_simplex);
        $m!(mixed);
        $m!(mixed_latent);
    };
}

/// The registry's representatives, each generic over its parameters' float type: the parameters
/// `engine/tests/well_posedness.rs` registers each chart with, and an oblique latent plane through every block.
pub mod charts {
    use super::*;

    fn of<P: Precision, const N: usize>(x: [f64; N]) -> [P; N] {
        x.map(P::of)
    }

    /// An oblique latent plane through every block, `|zₖ| ≤ 3.4` over `[0, 1]²`.
    pub fn latent<P: Precision>() -> Latent<P> {
        Latent {
            z0: of([0.3, -0.4, 0.5, -0.2, 0.1, 0.6, 0.2, -0.3]),
            q1: of([1.2, 0.4, -0.9, 1.5, 0.3, -1.1, 0.8, 0.5]),
            q2: of([-0.6, 1.4, 0.7, -0.3, 1.6, 0.9, -0.5, 1.1]),
        }
    }

    fn sphere<P: Precision>(map: ShapeMap, scale: f64) -> ShapeSphere<P> {
        ShapeSphere {
            map,
            n0: of([0.0, 0.0, 1.0]),
            e1: of([1.0, 0.0, 0.0]),
            e2: of([0.0, 1.0, 0.0]),
            scale: P::of(scale),
            inertia: P::of(1.0),
            phase: P::of(0.0),
            z_mass: of([0.1, -0.2]),
            z_mom: of([0.1, 0.2, -0.1, 0.0]),
        }
    }

    /// The `(θ, φ)` shape sphere, both hemispheres.
    pub fn shape_sphere<P: Precision>() -> ShapeSphere<P> {
        sphere(ShapeMap::Spherical, 0.5)
    }

    /// The exponential map about the pole.
    pub fn shape_sphere_exponential<P: Precision>() -> ShapeSphere<P> {
        sphere(ShapeMap::Exponential, 1.0)
    }

    /// `(ν, K)` on a one-sided range, bijective.
    pub fn burrau<P: Precision>() -> Burrau<P> {
        Burrau {
            nu_range: of([0.05, 0.4]),
            k_max: P::of(1.0),
            gamma_k: P::of(2.0),
        }
    }

    /// `(ν, K)` on the full Euclid range, `DoubleCover`.
    pub fn burrau_full<P: Precision>() -> Burrau<P> {
        Burrau {
            nu_range: of([1.0 / 32.0, 31.0 / 32.0]),
            k_max: P::of(1.0),
            gamma_k: P::of(2.0),
        }
    }

    /// The Euclid plane on its full range.
    pub fn euclid_plane<P: Precision>() -> EuclidPlane<P> {
        EuclidPlane {
            m_range: of([1.0, 32.0]),
            nu_range: of([1.0 / 32.0, 31.0 / 32.0]),
        }
    }

    fn theta<P: Precision>() -> [P; 2] {
        of([0.2, std::f64::consts::FRAC_PI_4])
    }

    /// `(θ, K)`.
    pub fn burrau_theta_k<P: Precision>() -> BurrauThetaK<P> {
        BurrauThetaK {
            theta_range: theta(),
            k_max: P::of(1.0),
            gamma_k: P::of(2.0),
        }
    }

    /// `(θ, L_z)`.
    pub fn burrau_theta_lz<P: Precision>() -> BurrauThetaLz<P> {
        BurrauThetaLz {
            theta_range: theta(),
            k: P::of(0.5),
            lz_fraction: P::of(0.9),
        }
    }

    /// `(θ, δm)` toward equal masses.
    pub fn burrau_theta_delta_m<P: Precision>() -> BurrauThetaDeltaM<P> {
        BurrauThetaDeltaM {
            theta_range: theta(),
            m_target: BurrauThetaDeltaM::<P>::equal_masses(),
        }
    }

    fn invariant<P: Precision>() -> Invariant<P> {
        Invariant {
            z_cfg: of([0.3, -0.5]),
            z_mass: of([0.2, 0.1]),
            k_max: P::of(1.0),
            gamma_k: P::of(2.0),
        }
    }

    /// `(L_z, E)`.
    pub fn invariant_le<P: Precision>() -> InvariantLE<P> {
        InvariantLE(invariant())
    }

    /// `(L_z, K)`.
    pub fn invariant_lk<P: Precision>() -> InvariantLK<P> {
        InvariantLK(invariant())
    }

    /// The ternary mass plot at `ν₀ = ½`.
    pub fn mass_simplex<P: Precision>() -> MassSimplex<P> {
        MassSimplex {
            nu0: P::of(0.5),
            eps_m: P::of(1e-4),
        }
    }

    /// A mixed chart of two derived quantities, `α` and `p_ρx`.
    pub fn mixed<P: Precision>() -> Mixed<Alpha, PRhoX, P> {
        Mixed::new(of([0.0; 8]), of([0.1, 1.4]), of([-1.0, 1.0]))
    }

    /// A mixed chart of two latent coordinates, `z_μ1` and `z_q2`.
    pub fn mixed_latent<P: Precision>() -> Mixed<ZMu1, ZQ2, P> {
        Mixed::new(of([0.0; 8]), of([-1.0, 1.0]), of([-1.0, 1.0]))
    }
}

/// The registered chart `key`, validated at both precisions, or `None` for a key the registry does not hold. Panics
/// naming the chart where a representative is refused by the validator: a registered chart validates.
pub fn chart(key: &str) -> Option<Box<dyn Registered>> {
    fn entry<A, B>(key: &'static str, a: A, b: B) -> Box<dyn Registered>
    where
        A: Authored<Param = f64> + 'static,
        B: Authored<Param = f32> + 'static,
    {
        Box::new(Entry {
            key,
            c64: checked(key, a),
            c32: checked(key, b),
        })
    }
    fn checked<C: Authored>(key: &str, c: C) -> ValidatedChart<C> {
        validate(c, None).unwrap_or_else(|e| {
            panic!("the registered chart {key} is refused by the validator: {e}")
        })
    }
    let mut found = None;
    macro_rules! pick {
        ($k:ident) => {
            if found.is_none() && key == stringify!($k) {
                found = Some(entry(
                    stringify!($k),
                    charts::$k::<f64>(),
                    charts::$k::<f32>(),
                ));
            }
        };
    }
    registered_charts!(pick);
    found
}

/// Every registered chart's key, in the registry's order.
pub fn keys() -> Vec<&'static str> {
    let mut out = Vec::new();
    macro_rules! push {
        ($k:ident) => {
            out.push(stringify!($k));
        };
    }
    registered_charts!(push);
    out
}

/// Every registered chart, in the registry's order.
pub fn registry() -> Vec<Box<dyn Registered>> {
    keys()
        .into_iter()
        .map(|k| chart(k).unwrap_or_else(|| panic!("the registry lists {k} and holds none")))
        .collect()
}
