//! The live shape readout, per macro-step (dd_integrator §3.7; integrator_contract Part 1, `expose_state_readout`;
//! dd_generation_root §3.5): after each completed macro-step the wrapper derives the shape point `n = (u, v, w)/I` from
//! the current positions and masses alone, never the momenta ([`shape`]; payload §5's `shape(r, masses)`), and stores
//! nothing of it: no checkpoint array, no history. From it the readout advances two things the payload carries:
//! - **`θ̃`**, the unwrapped phase, with R-389's pole hold kept in `SimState` alone (R-397): the frozen reference in
//!   `_reserved` as a u16 code ([`encode`], [`decode`]), `theta_ref_none` (0xFFFF) for "no reference";
//! - **the closure registers** `closure_min` and `closure_step` with the departed bit ([`super::closure`]; R-37).
//!
//! **The hold in `SimState` (R-397; payload §2, `_reserved`).** A fresh sample's `_reserved` is 0xFFFF, wherever its IC
//! lies. Each macro-step, with "inside" decided on the state before it and the state after it by the march's disc test
//! ([`crate::shape::config_in_disc`]; R-547), never stored:
//! - outside → outside: `θ̃` adds `wrap(longitude(n) − longitude(n_prev))`, unquantised; `_reserved` stays 0xFFFF;
//! - outside → inside (entry): `_reserved` takes the code of `n_prev`'s longitude, the last one outside; `θ̃` holds;
//! - inside → inside: both hold;
//! - inside → outside (exit): with a code, `θ̃` adds `wrap(longitude(n) − decode(code))`, exactly ±π adding +π; with
//!   0xFFFF (R-392's IC that starts inside), nothing; `_reserved` becomes 0xFFFF.
//!
//! The reference is quantised when it is stored, and every exit differences against the decoded code, so `θ̃` and
//! `_reserved` are bit-identical however the march is split into dispatches: a resumed march reloads `θ̃`, `_reserved`
//! and the closure registers from the stored `SimState` ([`Readout::resume`]) and recomputes the previous point and its
//! side of the disc from the stored positions, the same function of the same bits as the uninterrupted march's.
//!
//! **The code** (payload §2; R-397). Encode, of a longitude `λ ∈ [−π, π]`: `λ' = λ`, or `λ + 2π` where `λ < 0`;
//! `c = ⌊λ'·N/(2π) + ½⌋`, `N` the register's `theta_ref_steps` (65535), and `c = N` becomes 0, so the encode never
//! yields the sentinel. Decode: `c·2π/N`, less `2π` where it exceeds `π`, so it lies in `(−π, π)`. `encode(decode(c)) =
//! c` for every code. The decoded reference is within `π/N + 4·ulp_Real(2π)` of the longitude encoded. `N/(2π)` and
//! `2π/N` are each formed once from `π` and `N`, so the encode is one multiply and one add before the floor, the decode
//! one multiply.
//!
//! **The constants** are the register's, read through the generated `R_POLE`, `DELTA_DEP`, `THETA_REF_STEPS` and
//! `THETA_REF_NONE`: `r_pole` and `δ_dep` are one f64 each, read at the march's width ([`Width`]).

use spirv_std::num_traits::{Float, FloatConst};

use super::advance::{Count, Width};
use super::closure::Closure;
use crate::generated::constants::{DELTA_DEP, R_POLE, THETA_REF_NONE, THETA_REF_STEPS};
use crate::generated::links::LinkReal;
use crate::payload::{sd_departed, set_departed, PayloadReal, SimStateBaseOf};
use crate::real::IeeeBits;
use crate::shape::{config_in_disc, longitude, theta_step_held, Theta};
use crate::Real;

pub use crate::shape::shape;

/// The float type the readout runs at: a payload's, with the shape map's and the disc test's operations, step counts
/// converted and a register value read at its width. f32 and f64.
pub trait ReadoutReal: PayloadReal + LinkReal + IeeeBits + FloatConst + Count + Width {}

impl<R: PayloadReal + LinkReal + IeeeBits + FloatConst + Count + Width> ReadoutReal for R {}

/// `_reserved`'s "no reference" sentinel, the register's `theta_ref_none` (0xFFFF; R-397).
pub const NO_REFERENCE: u32 = THETA_REF_NONE;

/// `r_pole`, the register's, at the width `R` (R-389, R-433).
#[inline]
pub fn r_pole<R: Width>() -> R {
    R::at_width(R_POLE as f32, R_POLE)
}

/// `δ_dep`, the register's, at the width `R` (R-37; REQ-VAL-055).
#[inline]
pub fn delta_dep<R: Width>() -> R {
    R::at_width(DELTA_DEP as f32, DELTA_DEP)
}

/// `2π`.
#[inline]
fn tau<R: Real + FloatConst>() -> R {
    R::PI() + R::PI()
}

/// The code of the longitude `lon ∈ [−π, π]` (the module's note): `⌊λ'·N/(2π) + ½⌋`, `N` mapped to 0. Never the
/// sentinel.
#[inline]
pub fn encode<R: Real + Float + FloatConst + Count>(lon: R) -> u32 {
    let shifted = if lon < R::zero() { lon + tau() } else { lon };
    let steps = R::count(THETA_REF_STEPS);
    let half = R::one() / (R::one() + R::one());
    let c = Float::floor(shifted * (steps / tau()) + half).whole();
    if c >= THETA_REF_STEPS {
        0
    } else {
        c
    }
}

/// The longitude of the code `c ∈ 0 … N − 1` (the module's note): `c·2π/N`, less `2π` where it exceeds `π`, in
/// `(−π, π)`.
#[inline]
pub fn decode<R: Real + Float + FloatConst + Count>(c: u32) -> R {
    let lon = R::count(c) * (tau::<R>() / R::count(THETA_REF_STEPS));
    if lon > R::PI() {
        lon - tau()
    } else {
        lon
    }
}

/// One macro-step of `θ̃` with its reference in `_reserved` (the module's note): `theta` and the stored `code` before
/// the step, `n_prev` and `n` the shape points before and after it, `prev_inside` and `inside` their sides of the disc.
/// Returns `θ̃` and the code after it. The step itself is R-389's ([`theta_step_held`]), its reference the decoded code.
#[inline]
pub fn theta_hold<R: Real + Float + FloatConst + Count>(
    theta: R,
    code: u32,
    n_prev: [R; 3],
    prev_inside: bool,
    n: [R; 3],
    inside: bool,
) -> (R, u32) {
    let held = Theta {
        theta,
        frozen: decode(code),
        has_frozen: code != NO_REFERENCE,
    };
    let next = theta_step_held(held, n_prev, n, prev_inside, inside);
    let code = if !inside {
        NO_REFERENCE
    } else if prev_inside {
        code
    } else {
        encode(longitude(n_prev))
    };
    (next.theta, code)
}

/// The readout's state across macro-steps: what the payload stores (`θ̃`, `_reserved`, the closure registers and the
/// departed bit) and, recomputed from the stored positions, the previous point and its side of the disc; and the IC's
/// shape `n̂(0)`, which the closure is measured from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Readout<R> {
    /// `θ̃`, the net turning since the IC (R-389).
    pub theta: R,
    /// `_reserved`: the code of the frozen reference, or [`NO_REFERENCE`].
    pub code: u32,
    /// The closure registers and the departed bit.
    pub closure: Closure<R>,
    /// `n̂(0)`, the IC's shape.
    pub n_0: [R; 3],
    /// The shape point of the state the next macro-step starts from.
    n_prev: [R; 3],
    /// Whether that state lies inside the pole disc.
    prev_inside: bool,
}

impl<R: ReadoutReal> Readout<R> {
    /// A fresh sample's readout at its IC's positions `r_0` with masses `m`: `θ̃ = 0`, `_reserved` 0xFFFF, wherever the
    /// IC lies (R-389, R-392, R-397), and the closure not departed ([`Closure::fresh`]).
    #[inline]
    pub fn fresh(r_0: [[R; 2]; 3], m: [R; 3]) -> Self {
        Self::resume(
            shape(r_0, m),
            r_0,
            m,
            R::zero(),
            NO_REFERENCE,
            Closure::fresh(),
        )
    }

    /// The readout resumed at the positions `r` with masses `m`, the IC's shape `n_0`, and `θ̃`, the code and the
    /// closure as stored: the previous point and whether it is inside are recomputed from `r` (R-397).
    #[inline]
    pub fn resume(
        n_0: [R; 3],
        r: [[R; 2]; 3],
        m: [R; 3],
        theta: R,
        code: u32,
        closure: Closure<R>,
    ) -> Self {
        Readout {
            theta,
            code,
            closure,
            n_0,
            n_prev: shape(r, m),
            prev_inside: config_in_disc(r, m, r_pole()),
        }
    }

    /// The readout of the stored `s`, whose IC's shape is `n_0` and masses `m` ([`Readout::resume`] on what `s`
    /// stores: its `θ̃`, `_reserved`, closure registers and departed bit).
    #[inline]
    pub fn load(s: &SimStateBaseOf<R>, n_0: [R; 3], m: [R; 3]) -> Self {
        let closure = Closure {
            min: s.closure_min,
            step: u32::from(s.closure_step),
            departed: sd_departed(s.packed_a),
        };
        Self::resume(n_0, s.r, m, s.theta, u32::from(s._reserved), closure)
    }

    /// After the macro-step that completes the count `step`, at the positions `r` with masses `m`: the shape point,
    /// its side of the disc, `θ̃` and `_reserved` ([`theta_hold`]), and the closure at `step` ([`Closure::sample`]).
    #[inline]
    pub fn after_macro(&mut self, r: [[R; 2]; 3], m: [R; 3], step: u32) {
        self.point(shape(r, m), config_in_disc(r, m, r_pole()), step);
    }

    /// After the macro-step that completes the count `step`, at the shape point `n`, inside the disc or not.
    #[inline]
    pub fn point(&mut self, n: [R; 3], inside: bool, step: u32) {
        let (theta, code) = theta_hold(
            self.theta,
            self.code,
            self.n_prev,
            self.prev_inside,
            n,
            inside,
        );
        self.theta = theta;
        self.code = code;
        self.closure = self.closure.sample(n, self.n_0, step, delta_dep());
        self.n_prev = n;
        self.prev_inside = inside;
    }

    /// `out` with what the readout stores written in: `theta`, `_reserved`, `closure_min`, `closure_step` and the
    /// departed bit, `packed_a`'s other bits kept. The code and the step are at most 0xFFFF (`THETA_REF_NONE`,
    /// `horizon_steps_max`), so each fits its u16.
    #[inline]
    pub fn store(&self, out: SimStateBaseOf<R>) -> SimStateBaseOf<R> {
        SimStateBaseOf {
            theta: self.theta,
            _reserved: self.code as u16,
            closure_min: self.closure.stored_min(),
            closure_step: self.closure.step as u16,
            packed_a: set_departed(out.packed_a, self.closure.departed),
            ..out
        }
    }
}
