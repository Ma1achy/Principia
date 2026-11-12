//! The Benettin FTLE shadow (dd_integrator §2, §3.4, §3.8; integrator_contract Part 3, Part 5; payload §5): the FTLE
//! tier's co-computation, riding the forward pass beside the sample's own march (REQ-INT-035, REQ-INT-039,
//! REQ-INT-043).
//!
//! **One shadow per sample** (§3.8, verbatim from stability-metrics.md):
//!
//! ```text
//! init: x' = x₀ + δ₀ ;  S = 0
//! every n_renorm steps:  δ⃗ = x' − x ;  δ = ‖δ⃗‖ ;  S += log(δ/δ₀) ;  x' ← x + δ₀·(δ⃗/δ)
//! λ (read at any playhead / termination) = S_final / (step_count · dt_macro),  S_final = S + log(δ_current/δ₀)
//! ```
//!
//! `x` is the sample's phase-space state, the twelve components `(r, p)`, and `δ` the Euclidean norm over them, the
//! read side's `benettin_delta` ([`delta`]). The offset `δ₀` is `‖δ₀‖` times §3.8's fixed direction
//! ([`offset`]; REQ-INT-077): in the positions alone, its components `±½` and 0, so it is exact at either
//! precision and the same on every backend. `‖δ₀‖` and `n_renorm` are REQ-INT-076's calibration ([`BenettinParams`]);
//! the sim key carries them.
//!
//! **Its own trajectory, its own projection** (§3.4; REQ-INT-039). The shadow is marched by the bound occupant through
//! its own [`WrapperCadence`]: its own COM projection, its `M` cached from its own masses, its own monitor and its own
//! detector, every `STEP`, so its CoM drift under its own roundoff is removed by its own `R_com`, never by the
//! sample's. It is marched after each of the sample's macro-steps ([`super::wrapper::CoMarch`]), from the same grid
//! time to the same grid time, with the sample's `N_sub`, and it takes no more substeps than the sample took: a
//! macro-step the sample's `done` cut short stops the shadow at the same substep, so `δ` is always read between two
//! states at one time. Taking the sample's `N_sub` makes the two the same discrete map, whose separation the exponent
//! is of. The shadow's detectors only watch (R-972, pending in the rulings PR; the owner's words of 12 Nov 2026 on
//! RQ-791: "The shadow's collision check only watches and the shadow carries on through"): what would be the shadow's
//! terminal neither stops the shadow nor touches the sample, whose own detectors and outcome are its own. A shadow that
//! marches through a close approach can blow up; that is the next note's.
//!
//! **A non-finite sum** (R-972, option (a) of RQ-791: "if its sum goes non-finite, (a)"). Once the shadow's finalised
//! sum `S + ln(δ/δ₀)` is non-finite, the FTLE is invalid from then on: a non-finite value stays non-finite under the
//! sums and the march, so the mark is derived at write, with no latch. Storage never holds NaN (R-79), so the sample's
//! `S` then stores [`s_failed`], `R`'s largest finite value (`f32::MAX`, the generated read side's `S_FAILED_BITS`; at
//! f64, `f64::MAX`), and `r_sh`, `p_sh` the sample's own `r`, `p`: failed-state values, every other field the
//! sample's. A real `S` never reaches it: each renormalisation adds `ln(δ/δ₀)`, and for a finite, nonzero f32 `δ`
//! (from the smallest subnormal, `1.4e-45`, to `f32::MAX`) and `‖δ₀‖` in the calibration's range `[1e-6, 1e-2]`
//! that lies in `[ln(1.4e-45/1e-2), ln(f32::MAX/1e-6)] ≈ [−98.7, 102.5]`; there is at most one per macro-step and at
//! most 65535 macro-steps (`t_end_step` is a u16, payload §2), so `|S| ≤ 65535 · 103 ≈ 6.8e6`, 31 orders below
//! `f32::MAX`. At f64 the same count gives `|S| ≤ 65535 · 760 ≈ 5.0e7` (`ln(f64::MAX) ≈ 709.8`, the smallest
//! subnormal's log `≈ −744.4`), far below `f64::MAX`. Payload §6's `ftle_valid` gains the
//! clause `S ≠ S_FAILED` (compared by its bits), so `ftle` reads NaN (R-254) and every aggregate drops the sample
//! (R-255); payload §5's `S` is the stored sum or the mark. *Applied per R-369 (the smallest mark that changes no
//! other field: no new field and no new bit, the descriptor's 16 bits being assigned, R-545); physics checks it.*
//!
//! **Renormalisation** at the end of every `n_renorm`-th completed macro-step, counted by an integer counter, so the
//! decision is a comparison of integers (integrator_contract Part 4): `completed_renorms = step_count / n_renorm`, as
//! the read side derives it. `n_renorm = 0` never renormalises, and the read side reads no completed renorm.
//!
//! **Two forms of the held shadow** (payload §8; REQ-VAL-038): [`Absolute`] holds `x'` between macro-steps, the
//! payload's `r_sh`, `p_sh`; [`Displacement`] holds `δ⃗`, forming `x + δ⃗` at each macro-step's start. The payload
//! stores the absolute form; the displacement form is measured against it on known-Lyapunov periodic orbits
//! (`validation::measure::benettin`), and the switch waits for REQ-PAY-085's criterion.
//!
//! **Finalised at read** ([`ftle`], [`ftle_read`]): the partial interval since the last renormalisation is folded in
//! before dividing by the full elapsed `step_count · dt_macro`, never plain `S / t` (payload §5); the read is NaN
//! whenever `ftle_valid` is false (R-253, R-254).
//!
//! **Never a sample** (§2; REQ-INT-035): the shadow is held here, apart from the sample's `SimState`, and written only
//! into `r_sh`, `p_sh` and `S`; the state a sample gives the colour/spread pool is its own `(r, p)` ([`pool_member`]).

use core::marker::PhantomData;

use spirv_std::num_traits::Float;

use super::advance::{Advance, AdvanceParams, AdvanceReal, Cadence};
use super::cadence::WrapperCadence;
use super::wrapper::{integrate_with_co, monitored, CoMarch, WrapperReal, WrapperUniforms};
use crate::decode::Ic;
use crate::detect::collision::CollisionRadius;
use crate::occupant::{Geom, State};
use crate::payload::{sd_is_failed, tm_t_end_step, PayloadReal, SimStateBaseOf, SimStateFTLEOf};

/// The sim key's Benettin values (integrator_contract Part 3): `‖δ₀‖` and the renormalisation interval in macro-steps,
/// REQ-INT-076's calibration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BenettinParams<R> {
    /// `‖δ₀‖`, the separation the shadow starts at and is reset to.
    pub delta_0: R,
    /// `n_renorm`, the completed macro-steps between renormalisations.
    pub n_renorm: u32,
}

/// What the shadow holds between macro-steps: `x'` itself, or `δ⃗ = x' − x` (the module's note).
pub trait ShadowForm {
    /// Whether the held state is the displacement `δ⃗`.
    const DISPLACEMENT: bool;
}

/// The absolute form: the shadow's state `x'`, the payload's.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Absolute;

impl ShadowForm for Absolute {
    const DISPLACEMENT: bool = false;
}

/// The displacement form: `δ⃗ = x' − x`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Displacement;

impl ShadowForm for Displacement {
    const DISPLACEMENT: bool = true;
}

/// `f` applied to each of the six components of `a` and `b`, written out: no loop (gpu_determinism_note § "The
/// discipline", rule 5) and no runtime index.
#[inline]
fn each<R: Copy>(a: &[[R; 2]; 3], b: &[[R; 2]; 3], f: impl Fn(R, R) -> R) -> [[R; 2]; 3] {
    [
        [f(a[0][0], b[0][0]), f(a[0][1], b[0][1])],
        [f(a[1][0], b[1][0]), f(a[1][1], b[1][1])],
        [f(a[2][0], b[2][0]), f(a[2][1], b[2][1])],
    ]
}

/// `a + b` per component of the positions and momenta; `a`'s masses.
#[inline]
fn plus<R: AdvanceReal>(a: &State<R>, b: &State<R>) -> State<R> {
    State {
        m: a.m,
        r: each(&a.r, &b.r, |x, y| x + y),
        p: each(&a.p, &b.p, |x, y| x + y),
    }
}

/// `a − b` per component of the positions and momenta; `a`'s masses.
#[inline]
fn minus<R: AdvanceReal>(a: &State<R>, b: &State<R>) -> State<R> {
    State {
        m: a.m,
        r: each(&a.r, &b.r, |x, y| x - y),
        p: each(&a.p, &b.p, |x, y| x - y),
    }
}

/// `k·d + x` per component, each one rounded multiply-add (gpu_determinism_note § "The discipline", rule 2); `x`'s
/// masses.
#[inline]
fn scaled_plus<R: AdvanceReal>(k: R, d: &State<R>, x: &State<R>) -> State<R> {
    State {
        m: x.m,
        r: each(&d.r, &x.r, |dk, xk| k.fma(dk, xk)),
        p: each(&d.p, &x.p, |dk, xk| k.fma(dk, xk)),
    }
}

/// The shadow's initial state `x₀ + ‖δ₀‖·u` (dd_integrator §3.8; REQ-INT-077): `u`, the direction of `δ₀`, is
/// `δr₀ = (+½, 0)`, `δr₁ = (−½, +½)`, `δr₂ = (0, −½)` and no momentum, a unit vector over the twelve components; the
/// momenta are `x₀`'s and each position component `r + ‖δ₀‖·uₖ`, one rounded multiply-add each.
#[inline]
pub fn offset<R: AdvanceReal>(x: &State<R>, delta_0: R) -> State<R> {
    let one = R::one();
    let half = one / (one + one);
    let mut u = State {
        m: x.m,
        r: [[R::default(); 2]; 3],
        p: [[R::default(); 2]; 3],
    };
    u.r[0][0] = half;
    u.r[1][0] = -half;
    u.r[1][1] = half;
    u.r[2][1] = -half;
    scaled_plus(delta_0, &u, x)
}

/// `δ = ‖x' − x‖`, the Euclidean norm of the offset over the twelve components, in the read side's order (each body's
/// `dx² + dy²`, the positions' sum, the momenta's, then the root of their sum; payload §5's `benettin_delta`).
#[inline]
pub fn delta<R: WrapperReal>(
    r: &[[R; 2]; 3],
    p: &[[R; 2]; 3],
    r_sh: &[[R; 2]; 3],
    p_sh: &[[R; 2]; 3],
) -> R {
    let d = |a: [R; 2], b: [R; 2]| {
        let (x, y) = (b[0] - a[0], b[1] - a[1]);
        x * x + y * y
    };
    let r2 = d(r[0], r_sh[0]) + d(r[1], r_sh[1]) + d(r[2], r_sh[2]);
    let p2 = d(p[0], p_sh[0]) + d(p[1], p_sh[1]) + d(p[2], p_sh[2]);
    Float::sqrt(r2 + p2)
}

/// `λ = (S + ln(δ/δ₀)) / (n·dt_macro)`: the FTLE with the partial interval since the last renormalisation finalised,
/// divided by the full elapsed time (payload §5; integrator_contract Part 5), never plain `S / t`.
#[inline]
pub fn ftle<R: WrapperReal>(s_sum: R, delta: R, delta_0: R, n: u32, dt_macro: R) -> R {
    (s_sum + Float::ln(delta / delta_0)) / (R::count(n) * dt_macro)
}

/// The renormalisations completed by step `n` (payload §5): the generated read side's, one source for the kernel's
/// count and the read's.
pub use crate::payload::completed_renorms;

/// `S`'s failed-state value, the mark of a shadow whose finalised sum went non-finite (R-972; the module's note):
/// `R`'s largest finite value, `f32::MAX` (the generated read side's `S_FAILED_BITS`) or `f64::MAX`. Never NaN (R-79),
/// and never a real `S`, whose magnitude is bounded far below it.
#[inline]
pub fn s_failed<R: WrapperReal>() -> R {
    R::max_value()
}

/// Whether a stored `S` is the mark [`s_failed`]: the shadow's sum went non-finite, so the FTLE is invalid (R-972;
/// payload §6's `ftle_valid`, its clause `S ≠ S_FAILED`).
#[inline]
pub fn sum_failed<R: WrapperReal>(s_sum: R) -> bool {
    s_sum == s_failed::<R>()
}

/// The FTLE read from a stored sample `s` (payload §5): [`ftle`] from `S` and the stored shadow's `δ`, at `step_count
/// = t_end_step`; NaN whenever `ftle_valid` is false: a failed sample, `step_count = 0`, no completed
/// renormalisation (R-253, R-254), or a shadow whose sum went non-finite, `S` the mark [`s_failed`] (R-972).
#[inline]
pub fn ftle_read<R: WrapperReal>(
    s: &SimStateFTLEOf<R>,
    params: &BenettinParams<R>,
    dt_macro: R,
) -> R {
    let n = tm_t_end_step(s.times);
    // `step_count ≥ 1` (R-253) is implied: a completed renormalisation needs `n ≥ n_renorm ≥ 1`.
    let valid =
        !sd_is_failed(s.packed_a) && completed_renorms(n, params.n_renorm) > 0 && !sum_failed(s.S);
    if valid {
        let d = delta(&s.r, &s.p, &s.r_sh, &s.p_sh);
        ftle(s.S, d, params.delta_0, n, dt_macro)
    } else {
        R::nan()
    }
}

/// The state a sample gives the colour/spread pool (dd_integrator §2, §3.8; REQ-INT-035): its own `(r, p)`. The
/// shadow's `r_sh`, `p_sh` are never read: a renormalised endpoint is an algorithmic artefact, not a trajectory.
#[inline]
pub fn pool_member<R: PayloadReal>(s: &SimStateFTLEOf<R>) -> ([[R; 2]; 3], [[R; 2]; 3]) {
    (s.r, s.p)
}

/// The shadow's cadence for one macro-step: its own [`WrapperCadence`], done once it has taken `left` more substeps,
/// the sample's count. Its detectors only watch (R-972): what would be the shadow's terminal never ends its march. The
/// shadow's `N_sub` is the sample's, so no bucket is read from it.
struct Limited<'a, C> {
    inner: &'a mut C,
    left: u32,
}

impl<R: WrapperReal, C: Cadence<R>> Cadence<R> for Limited<'_, C> {
    #[inline]
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.inner.after_step(state, geom);
        self.left -= 1;
    }

    #[inline]
    fn done(&self) -> bool {
        self.left == 0
    }
}

/// One sample's Benettin shadow in the form `F` (the module's note): what it holds, its own cadence, `S` and the
/// macro-steps since its last renormalisation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Benettin<R, F> {
    /// `x'` ([`Absolute`]) or `δ⃗` ([`Displacement`]); the masses are the sample's.
    held: State<R>,
    /// The force evaluation at `x'`'s positions ([`Absolute`]; unused by [`Displacement`], which forms it per
    /// macro-step).
    geom: Geom<R>,
    /// The shadow's own projection, monitor and detector.
    cadence: WrapperCadence<R>,
    /// `‖δ₀‖` and `n_renorm`.
    params: BenettinParams<R>,
    /// `S`, the sum of `ln(δ/δ₀)` over the renormalisations so far.
    s_sum: R,
    /// The completed macro-steps since the last renormalisation.
    since: u32,
    form: PhantomData<F>,
}

impl<R: WrapperReal, F: ShadowForm> Benettin<R, F> {
    /// The shadow of a sample starting from `ic`, before its first step: `x'₀ = x₀ + δ₀` ([`offset`]), its cadence
    /// built from `x'₀` under the sample's collision threshold `radius`, `S = 0`.
    #[inline]
    pub fn new(ic: &Ic<R>, radius: CollisionRadius, params: BenettinParams<R>) -> Self {
        let x = State {
            m: ic.m,
            r: ic.r,
            p: ic.p,
        };
        let shadow = offset(&x, params.delta_0);
        let held = if F::DISPLACEMENT {
            minus(&shadow, &x)
        } else {
            shadow
        };
        Benettin {
            held,
            geom: Geom::new(&shadow.m, &shadow.r),
            cadence: WrapperCadence::with_radius(&shadow.m, &shadow.r, &shadow.p, radius),
            params,
            s_sum: R::default(),
            since: 0,
            form: PhantomData,
        }
    }

    /// The shadow's state `x'` beside the sample's `x`.
    #[inline]
    pub fn shadow(&self, x: &State<R>) -> State<R> {
        if F::DISPLACEMENT {
            plus(x, &self.held)
        } else {
            self.held
        }
    }

    /// `δ⃗ = x' − x` beside the sample's `x`.
    #[inline]
    pub fn offset_from(&self, x: &State<R>) -> State<R> {
        if F::DISPLACEMENT {
            self.held
        } else {
            minus(&self.held, x)
        }
    }

    /// `S`, the renormalisations' sum so far.
    #[inline]
    pub fn s_sum(&self) -> R {
        self.s_sum
    }

    /// The shadow's own cadence: its projection, monitor, detector and the calls made.
    #[inline]
    pub fn cadence(&self) -> &WrapperCadence<R> {
        &self.cadence
    }

    /// `δ` beside the sample's `x`.
    #[inline]
    pub fn separation(&self, x: &State<R>) -> R {
        let d = self.offset_from(x);
        let zero = [[R::default(); 2]; 3];
        delta(&zero, &zero, &d.r, &d.p)
    }

    /// The renormalisation at the sample's `x` (§3.8): `δ⃗ = x' − x`, `S += ln(δ/δ₀)`, `x' ← x + (δ₀/δ)·δ⃗`.
    #[inline]
    pub fn renormalise(&mut self, x: &State<R>) {
        let d = self.offset_from(x);
        let zero = [[R::default(); 2]; 3];
        let dist = delta(&zero, &zero, &d.r, &d.p);
        self.s_sum = self.s_sum + Float::ln(dist / self.params.delta_0);
        let k = self.params.delta_0 / dist;
        if F::DISPLACEMENT {
            let none = State {
                m: x.m,
                r: zero,
                p: zero,
            };
            self.held = scaled_plus(k, &d, &none);
        } else {
            self.held = scaled_plus(k, &d, x);
            self.geom = Geom::new(&self.held.m, &self.held.r);
        }
    }

    /// The FTLE finalised beside the sample's `x` after `n` completed macro-steps ([`ftle`]), whatever the validity.
    #[inline]
    pub fn ftle(&self, x: &State<R>, n: u32, dt_macro: R) -> R {
        ftle(
            self.s_sum,
            self.separation(x),
            self.params.delta_0,
            n,
            dt_macro,
        )
    }

    /// Whether the sum finalised beside the sample's `x`, `S + ln(δ/δ₀)`, is finite: false once the shadow's sum has
    /// gone non-finite, and from then on (R-972; the module's note).
    #[inline]
    pub fn sum_finite(&self, x: &State<R>) -> bool {
        let finalised = self.s_sum + Float::ln(self.separation(x) / self.params.delta_0);
        finalised.is_finite()
    }

    /// The FTLE tier's `SimState`: the sample's `out`, every field kept, with the shadow's absolute state in `r_sh`,
    /// `p_sh` and `S`; or, once the shadow's finalised sum is non-finite, the failed-state values: `S` the mark
    /// [`s_failed`] and `r_sh`, `p_sh` the sample's own `r`, `p`, never NaN (R-79, R-972).
    #[inline]
    pub fn write(&self, out: SimStateBaseOf<R>) -> SimStateFTLEOf<R> {
        let x = State {
            m: self.held.m,
            r: out.r,
            p: out.p,
        };
        let (sh, s_sum) = if self.sum_finite(&x) {
            (self.shadow(&x), self.s_sum)
        } else {
            (x, s_failed())
        };
        SimStateFTLEOf {
            r: out.r,
            p: out.p,
            r_sh: sh.r,
            p_sh: sh.p,
            S: s_sum,
            theta: out.theta,
            mean_y: out.mean_y,
            C_ty: out.C_ty,
            E_0: out.E_0,
            Lz_0: out.Lz_0,
            packed_a: out.packed_a,
            packed_b: out.packed_b,
            times: out.times,
            total_substeps: out.total_substeps,
            closure_min: out.closure_min,
            closure_step: out.closure_step,
            _reserved: out._reserved,
            ..SimStateFTLEOf::default()
        }
    }
}

impl<R: WrapperReal, F: ShadowForm> CoMarch<R> for Benettin<R, F> {
    /// The shadow's macro-step beside the sample's: from `x'` (or `start + δ⃗`) through its own cadence, `N_sub` as the
    /// sample's and at most `taken` substeps; then, at the end of every `n_renorm`-th completed macro-step, the
    /// renormalisation at `end`.
    #[inline]
    fn after_macro<A: Advance>(
        &mut self,
        start: &State<R>,
        end: &State<R>,
        t_now: u32,
        params: &AdvanceParams<R>,
        taken: u32,
    ) {
        let (from, geom) = if F::DISPLACEMENT {
            let s = plus(start, &self.held);
            (s, Geom::new(&s.m, &s.r))
        } else {
            (self.held, self.geom)
        };
        let mut limited = Limited {
            inner: &mut self.cadence,
            left: taken,
        };
        let out = A::advance(&from, &geom, t_now, t_now + 1, params, &mut limited);
        if F::DISPLACEMENT {
            self.held = minus(&out.state, end);
        } else {
            self.held = out.state;
            self.geom = out.geom;
        }
        if taken == params.n_sub {
            self.since += 1;
            if self.since == self.params.n_renorm {
                self.renormalise(end);
                self.since = 0;
            }
        }
    }
}

/// The FTLE tier's integration of `ic` under the sim key's `uniforms`, collision threshold `radius` and Benettin
/// values `params` by the occupant `A`, the shadow in the form `F`: the sample's march as
/// [`super::wrapper::integrate_detecting`] runs it, bit for bit, with its shadow marched beside it, written into the
/// `SimState`'s `r_sh`, `p_sh` and `S` ([`Benettin::write`]). Also returns the shadow.
#[inline]
pub fn integrate_ftle_with<A: Advance, R: WrapperReal, F: ShadowForm>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    radius: CollisionRadius,
    params: BenettinParams<R>,
) -> (SimStateFTLEOf<R>, Benettin<R, F>) {
    let mut cadence = WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, radius);
    let mut shadow = Benettin::<R, F>::new(ic, radius, params);
    let out = integrate_with_co::<A, R, _, _>(ic, uniforms, &mut cadence, &mut shadow);
    let out = monitored(out, &cadence);
    (shadow.write(out), shadow)
}

/// [`integrate_ftle_with`] in the payload's form, [`Absolute`]: the FTLE tier's `SimState`.
#[inline]
pub fn integrate_ftle<A: Advance, R: WrapperReal>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    radius: CollisionRadius,
    params: BenettinParams<R>,
) -> SimStateFTLEOf<R> {
    integrate_ftle_with::<A, R, Absolute>(ic, uniforms, radius, params).0
}
