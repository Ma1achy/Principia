//! QA tests for TASK-M3-14 (the Benettin FTLE shadow), written from the requirements, not from the implementation:
//! REQ-INT-035, REQ-INT-039, REQ-INT-043, REQ-INT-076, REQ-INT-077, REQ-VAL-038; dd_integrator §2, §3.4, §3.8, §5
//! tests 10 and 11; payload §5, §8; R-253, R-254.
//!
//! - `qa_m314_sample_untouched` (REQ-INT-035): for every shadow configuration (‖δ₀‖ small, large; `n_renorm` 0, 1, the
//!   proposal; both forms), the FTLE tier's sample is the base tier's, bit for bit: the shadow never reaches the
//!   sample's state or its stored fields, and every sample carries a shadow of its own (`r_sh ≠ r`).
//! - `qa_m314_pool_reads_sample` (REQ-INT-035, dd test 11): the colour/spread pool's member is the sample's own `(r, p)`,
//!   whatever the shadow fields hold (NaN included), and the read side's `SimState` gives the same.
//! - `qa_m314_shadow_own_com` (REQ-INT-039, §3.4): on unequal masses, where §3.8's offset puts the shadow's CoM off the
//!   origin by `‖δ₀‖·|Σmᵢuᵢ|/M`, the shadow's CoM is at rounding level after every macro-step: it is projected with its
//!   own `R_com`, not the sample's (inheriting the sample's would keep the offset's CoM). Its cadence runs once per
//!   substep the sample took, every macro-step.
//! - `qa_m314_shadow_own_monitor` (REQ-INT-039): the shadow's monitor holds the shadow's own `E₀`, the energy of
//!   `x₀ + δ₀`, not the sample's.
//! - `qa_m314_renormalise` (REQ-INT-043, §3.8): one renormalisation at a sample state `x`, of a shadow at `x'`, adds
//!   `ln(δ/δ₀)` to `S` and leaves the shadow at `x + δ₀·δ⃗/δ`, `δ⃗ = x' − x`, computed here in f64: in both forms after a
//!   5000-step march with no renormalisation, and in the absolute form at a sample state moved off `x₀`.
//! - `qa_m314_renorm_cadence` (REQ-INT-043): over a march, `S` changes exactly at the end of every `n_renorm`-th
//!   macro-step and nowhere else, the separation is reset to `δ₀` there, and the number of changes is
//!   `step_count / n_renorm`.
//! - `qa_m314_completed_renorms` (REQ-INT-043): `completed_renorms = step_count / n_renorm`, none at `n_renorm = 0`.
//! - `qa_m314_ftle_read_known` (REQ-INT-043, R-253, R-254): on hand-filled samples the read is
//!   `(S + ln(δ/δ₀))/(n·dt)`, and NaN with `step_count = 0`, with no completed renormalisation, at `n_renorm = 0` and on
//!   a failed sample; the generated read side agrees at f32.
//! - `qa_m314_not_biased_low` (REQ-INT-043, dd test 10): at a renormalisation boundary the finalised read equals plain
//!   `S/T` (the separation is `δ₀` there); one step before a boundary, the finalised read is closer to the boundary's
//!   than plain `S/T` is (summed over several boundaries), so it does not under-read between boundaries.
//! - `qa_m314_dd10_invariance` (REQ-INT-043, REQ-INT-076, dd test 10): at the proposed `(‖δ₀‖, n_renorm)`, halving
//!   `‖δ₀‖`, doubling `n_renorm` and both leave `λ_T`, read from the stored sample, within the proposed tolerance on the
//!   Kepler-embedded orbit and Burrau, at f32 and f64.
//! - `qa_m314_kepler_vs_burrau` (REQ-INT-043, dd test 10): `λ_T ≈ 0` on the Kepler-embedded orbit (it falls as `T`
//!   doubles, as `ln(ct)/t` does for a separation growing polynomially), and Burrau's `λ_T` is positive and above it.
//! - `qa_m314_delta0_direction` (REQ-INT-077, §3.8): `x'₀ = x₀ + ‖δ₀‖·u` with `u = (+½,0 | −½,+½ | 0,−½)` on the
//!   positions and no momentum, each component one rounding, bit for bit at f32 and f64 for any state and masses;
//!   `‖u‖ = 1` and `Σu = 0`; the shadow a sample starts with is that offset.
//! - `qa_m314_lagrange_exponent` (REQ-VAL-038's ground truth): the known exponent of Lagrange's equilateral orbit is the
//!   largest real part of the roots of `s⁴ + s² + (27/4)β = 0` (Routh), computed here from the roots, and 0 in the
//!   stable case `27β < 1`.
//! - `qa_m314_forms_one_map` (REQ-VAL-038): the payload's form stays the absolute one (`integrate_ftle` is
//!   `integrate_ftle_with::<Absolute>`, bit for bit, at f32) until REQ-PAY-085's criterion says otherwise; both forms'
//!   start and reset are checked by `qa_m314_delta0_direction` and `qa_m314_renormalise`.
//!
//! **Tolerances.** Rounding bounds are `RND = 64` eps relative (as `qa_TASK-M2-13`), widened where a difference of two
//! nearby states loses digits: a state component `x` carrying an offset of size `δ₀` keeps `δ₀`'s relative precision
//! only to `ε·|x|/δ₀`. The dd test 10 tolerance is the REQ-INT-076 proposal's (`PROPOSED_TOLERANCE`, calibrated,
//! pending the M3 gate), applied to its own measure of change.
//!
//! **Domain** (pitfalls §10): Yoshida-4, `r_coll = 0` (no terminal fires), the orbits `validation::measure::benettin`
//! defines; each test bounds its march by the sim key's `horizon_steps`.
//!
//! Each test has a registered negative control naming the panic it expects (R-176, R-212).

use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::driver::advance::{Advance, AdvanceParams};
use kernel::driver::benettin::{
    completed_renorms, ftle_read, integrate_ftle, integrate_ftle_with, offset, pool_member,
    Absolute, Benettin, BenettinParams, Displacement, ShadowForm,
};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::wrapper::{
    integrate_detecting, integrate_with_co, CoMarch, WrapperReal, WrapperUniforms,
};
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::State;
use kernel::payload::{
    set_state, set_t_end_step, sim_state_from_ftle, ReadParams, SimStateBaseOf, SimStateFTLEOf,
    STATE_DECODE_FAILED, STATE_RUNNING, STATE_SIM_FAILED,
};
use validation::drift::{ic_at, sim_key, AsR};
use validation::measure::benettin::{
    burrau, kepler_embedded, lagrange, lagrange_exponent, routh_beta, Orbit, PROPOSED_DELTA_0,
    PROPOSED_N_RENORM, PROPOSED_TOLERANCE,
};
use validation::negative_control;

/// Rounding bound, in eps (the module's note).
const RND: f64 = 64.0;

// ── Precisions ──────────────────────────────────────────────────────────────────────────────────────────────────

trait P: WrapperReal + Into<f64> + std::fmt::Debug {
    const EPS: f64;
    fn bits(self) -> u64;
}

impl P for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    fn bits(self) -> u64 {
        u64::from(self.to_bits())
    }
}

impl P for f64 {
    const EPS: f64 = f64::EPSILON;
    fn bits(self) -> u64 {
        self.to_bits()
    }
}

fn params<R: P>(delta_0: f64, n_renorm: u32) -> BenettinParams<R>
where
    f64: AsR<R>,
{
    BenettinParams {
        delta_0: delta_0.to_r(),
        n_renorm,
    }
}

/// The sim key of `orbit` at horizon `t`, marched for `steps` macro-steps (`None`: the horizon's own count).
fn key<R: P>(t: f64, steps: Option<u32>) -> WrapperUniforms<R>
where
    f64: AsR<R>,
{
    let mut k = sim_key::<R>(t);
    if let Some(n) = steps {
        assert!(
            n <= k.horizon_steps,
            "a march past the horizon's step count"
        );
        k.horizon_steps = n;
    }
    k
}

fn bits6<R: P>(x: &[[R; 2]; 3]) -> Vec<u64> {
    x.iter().flatten().map(|v| v.bits()).collect()
}

/// `‖a − b‖` over the twelve components, in f64.
fn dist12(r: &[[f64; 2]; 3], p: &[[f64; 2]; 3], r2: &[[f64; 2]; 3], p2: &[[f64; 2]; 3]) -> f64 {
    let mut s = 0.0;
    for i in 0..3 {
        for k in 0..2 {
            s += (r2[i][k] - r[i][k]).powi(2) + (p2[i][k] - p[i][k]).powi(2);
        }
    }
    s.sqrt()
}

/// `Σmᵢrᵢ / M`, in f64.
fn com(m: &[f64; 3], r: &[[f64; 2]; 3]) -> [f64; 2] {
    let mt: f64 = m.iter().sum();
    [0, 1].map(|k| (0..3).map(|i| m[i] * r[i][k]).sum::<f64>() / mt)
}

/// `E = Σ pᵢ²/2mᵢ − Σ mᵢmⱼ/rᵢⱼ` (G = 1), in f64.
fn energy(m: &[f64; 3], r: &[[f64; 2]; 3], p: &[[f64; 2]; 3]) -> f64 {
    let kin: f64 = (0..3)
        .map(|i| (p[i][0] * p[i][0] + p[i][1] * p[i][1]) / (2.0 * m[i]))
        .sum();
    let pot: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| {
            let d = ((r[i][0] - r[j][0]).powi(2) + (r[i][1] - r[j][1]).powi(2)).sqrt();
            m[i] * m[j] / d
        })
        .sum();
    kin - pot
}

/// The unit direction of §3.8, over `(r, p)`.
const U_R: [[f64; 2]; 3] = [[0.5, 0.0], [-0.5, 0.5], [0.0, -0.5]];

// ── REQ-INT-035: the shadow never reaches the sample, the pool reads the sample ─────────────────────────────────

/// The sample's stored fields as bits, the base tier's set.
fn base_fields<R: P>(b: &SimStateBaseOf<R>) -> Vec<u64> {
    let mut v = bits6(&b.r);
    v.extend(bits6(&b.p));
    v.extend(
        [b.theta, b.mean_y, b.C_ty, b.E_0, b.Lz_0, b.closure_min]
            .iter()
            .map(|x| x.bits()),
    );
    v.extend(
        [b.packed_a, b.packed_b, b.times, b.total_substeps]
            .iter()
            .map(|&w| u64::from(w)),
    );
    v.push(u64::from(b.closure_step));
    v
}

fn ftle_fields<R: P>(s: &SimStateFTLEOf<R>) -> Vec<u64> {
    let mut v = bits6(&s.r);
    v.extend(bits6(&s.p));
    v.extend(
        [s.theta, s.mean_y, s.C_ty, s.E_0, s.Lz_0, s.closure_min]
            .iter()
            .map(|x| x.bits()),
    );
    v.extend(
        [s.packed_a, s.packed_b, s.times, s.total_substeps]
            .iter()
            .map(|&w| u64::from(w)),
    );
    v.push(u64::from(s.closure_step));
    v
}

/// The FTLE tier's run of `ic` under `prm` in the form `F`.
fn ftle_run<R: P, F: ShadowForm>(
    ic: &Ic<R>,
    k: &WrapperUniforms<R>,
    prm: BenettinParams<R>,
) -> SimStateFTLEOf<R>
where
    f64: AsR<R>,
{
    integrate_ftle_with::<Yoshida4, R, F>(ic, k, CollisionRadius::none(), prm).0
}

/// The sample from the FTLE tier, `tier`, is the base tier's `base`, bit for bit, and carries a shadow of its own.
fn check_untouched<R: P>(name: &str, tier: &SimStateFTLEOf<R>, base: &SimStateBaseOf<R>)
where
    f64: AsR<R>,
{
    assert_eq!(
        ftle_fields(tier),
        base_fields(base),
        "{name}: the FTLE tier's sample differs from the base tier's: the shadow reached the sample"
    );
    assert_ne!(
        bits6(&tier.r_sh),
        bits6(&tier.r),
        "{name}: the sample carries no shadow of its own"
    );
}

fn untouched_at<R: P>()
where
    f64: AsR<R>,
{
    for orbit in [burrau(), kepler_embedded()] {
        let ic = ic_at::<R>(&orbit.ic);
        let k = key::<R>(orbit.horizon, Some(2000));
        let base = integrate_detecting::<Yoshida4, R>(&ic, &k, CollisionRadius::none());
        for (d0, n) in [
            (PROPOSED_DELTA_0, PROPOSED_N_RENORM),
            (1e-6, 1),
            (0.1, 0),
            (1e-3, 7),
        ] {
            let name = format!("{} ‖δ₀‖ = {d0}, n_renorm = {n}", orbit.name);
            let a = ftle_run::<R, Absolute>(&ic, &k, params(d0, n));
            check_untouched(&format!("{name}, absolute"), &a, &base);
            let d = ftle_run::<R, Displacement>(&ic, &k, params(d0, n));
            check_untouched(&format!("{name}, displacement"), &d, &base);
        }
    }
}

#[test]
fn qa_m314_sample_untouched() {
    untouched_at::<f64>();
    untouched_at::<f32>();
}

negative_control!(
    qa_m314_sample_untouched,
    "a sample marched from its shadow's state (the shadow leaking into the sample) must fail",
    expected = "the shadow reached the sample",
    {
        let orbit = burrau();
        let ic = ic_at::<f64>(&orbit.ic);
        let k = key::<f64>(orbit.horizon, Some(2000));
        let base = integrate_detecting::<Yoshida4, f64>(&ic, &k, CollisionRadius::none());
        let x = State {
            m: ic.m,
            r: ic.r,
            p: ic.p,
        };
        let sh = offset(&x, PROPOSED_DELTA_0);
        let leaked = Ic {
            m: sh.m,
            r: sh.r,
            p: sh.p,
        };
        let tier = ftle_run::<f64, Absolute>(&leaked, &k, params(PROPOSED_DELTA_0, 300));
        check_untouched("leaked", &tier, &base);
    }
);

/// The pool's member, by `member`, is the sample's own `(r, p)` whatever the shadow's fields hold.
/// The pool's member a sample gives, by some rule.
type Member = fn(&SimStateFTLEOf<f64>) -> ([[f64; 2]; 3], [[f64; 2]; 3]);

fn check_pool(member: Member) {
    let orbit = burrau();
    let ic = ic_at::<f64>(&orbit.ic);
    let k = key::<f64>(orbit.horizon, Some(1500));
    let s = ftle_run::<f64, Absolute>(&ic, &k, params(PROPOSED_DELTA_0, PROPOSED_N_RENORM));
    let nan = SimStateFTLEOf {
        r_sh: [[f64::NAN; 2]; 3],
        p_sh: [[f64::NAN; 2]; 3],
        S: f64::NAN,
        ..s
    };
    let swapped = SimStateFTLEOf {
        r_sh: s.r,
        p_sh: s.p,
        r: s.r,
        p: s.p,
        ..s
    };
    for t in [s, nan, swapped] {
        let (r, p) = member(&t);
        assert!(
            bits6(&r) == bits6(&s.r) && bits6(&p) == bits6(&s.p),
            "the pool's member is not the sample's own (r, p): a shadow entered the pool"
        );
    }
}

#[test]
fn qa_m314_pool_reads_sample() {
    check_pool(pool_member::<f64>);
    // The read side (f32): its `(r, p)`, the pool's, is the sample's; the shadow is a separate field.
    let orbit = burrau();
    let ic = ic_at::<f32>(&orbit.ic);
    let k = key::<f32>(orbit.horizon, Some(1500));
    let s = ftle_run::<f32, Absolute>(&ic, &k, params(PROPOSED_DELTA_0, PROPOSED_N_RENORM));
    let read = sim_state_from_ftle(&s, [0; 4], false, 0.0, false, ic.m, &ReadParams::default());
    assert_eq!((bits6(&read.r), bits6(&read.p)), (bits6(&s.r), bits6(&s.p)));
    assert_eq!(
        (bits6(&read.r_sh), bits6(&read.p_sh)),
        (bits6(&s.r_sh), bits6(&s.p_sh))
    );
}

negative_control!(
    qa_m314_pool_reads_sample,
    "a pool that takes the shadow's state must fail",
    expected = "a shadow entered the pool",
    check_pool(|s| (s.r_sh, s.p_sh))
);

// ── REQ-INT-039: the shadow's own projection and monitor ────────────────────────────────────────────────────────

/// A recorder around a shadow: after each of the sample's macro-steps it lets the shadow march, then records the
/// shadow's state through `view`, the sample's end state, the shadow's `S`, its separation, and the substeps its
/// cadence ran against the sample's.
struct Rec<F: ShadowForm> {
    b: Benettin<f64, F>,
    view: fn(&Benettin<f64, F>, &State<f64>) -> State<f64>,
    log: Vec<Step>,
    sample_substeps: u32,
}

#[derive(Clone, Copy, Debug)]
struct Step {
    shadow: State<f64>,
    end: State<f64>,
    s_sum: f64,
    sep: f64,
    shadow_calls: u32,
    sample_calls: u32,
}

impl<F: ShadowForm> CoMarch<f64> for Rec<F> {
    fn after_macro<A: Advance>(
        &mut self,
        start: &State<f64>,
        end: &State<f64>,
        t_now: u32,
        params: &AdvanceParams<f64>,
        taken: u32,
    ) {
        self.b.after_macro::<A>(start, end, t_now, params, taken);
        self.sample_substeps += taken;
        self.log.push(Step {
            shadow: (self.view)(&self.b, end),
            end: *end,
            s_sum: self.b.s_sum(),
            sep: self.b.separation(end),
            shadow_calls: self.b.cadence().invocations(),
            sample_calls: self.sample_substeps,
        });
    }
}

/// Marches `orbit` for `steps` macro-steps at f64 with a recorded shadow.
fn record<F: ShadowForm>(
    orbit: &Orbit,
    steps: u32,
    prm: BenettinParams<f64>,
    view: fn(&Benettin<f64, F>, &State<f64>) -> State<f64>,
) -> (Rec<F>, Ic<f64>) {
    let ic = ic_at::<f64>(&orbit.ic);
    let k = key::<f64>(orbit.horizon, Some(steps));
    let mut cadence = WrapperCadence::new(&ic.m, &ic.r, &ic.p);
    let mut rec = Rec {
        b: Benettin::<f64, F>::new(&ic, CollisionRadius::none(), prm),
        view,
        log: Vec::new(),
        sample_substeps: 0,
    };
    integrate_with_co::<Yoshida4, f64, _, _>(&ic, &k, &mut cadence, &mut rec);
    assert_eq!(
        rec.log.len(),
        steps as usize,
        "the march took {steps} macro-steps"
    );
    (rec, ic)
}

/// The shadow's CoM is at rounding level after every macro-step, far below the offset's own CoM, and its cadence ran
/// once per substep the sample took.
fn check_own_com<F: ShadowForm>(rec: &Rec<F>, ic: &Ic<f64>, delta_0: f64) {
    let off_com = {
        let x = State {
            m: ic.m,
            r: ic.r,
            p: ic.p,
        };
        let o = offset(&x, delta_0);
        let c1 = com(&ic.m, &o.r);
        let c0 = com(&ic.m, &ic.r);
        ((c1[0] - c0[0]).powi(2) + (c1[1] - c0[1]).powi(2)).sqrt()
    };
    // The test can fire only on unequal masses: §3.8's offset then moves the CoM by ‖δ₀‖·|Σmᵢuᵢ|/M.
    assert!(
        off_com > 1e3 * RND * f64::EPSILON,
        "the fixture's offset does not move the CoM ({off_com:e}): unequal masses are needed"
    );
    for (n, s) in rec.log.iter().enumerate() {
        let scale = s
            .shadow
            .r
            .iter()
            .flatten()
            .fold(1.0f64, |a, &x| a.max(x.abs()));
        let c = com(&s.shadow.m, &s.shadow.r);
        let res = (c[0] * c[0] + c[1] * c[1]).sqrt();
        assert!(
            res <= RND * f64::EPSILON * scale,
            "step {}: the shadow's CoM is {res:e} (the offset's own is {off_com:e}): the shadow's CoM not removed by \
             its own projection",
            n + 1
        );
        let pc = [0, 1].map(|k| (0..3).map(|i| s.shadow.p[i][k]).sum::<f64>());
        let pscale = s
            .shadow
            .p
            .iter()
            .flatten()
            .fold(1.0f64, |a, &x| a.max(x.abs()));
        assert!(
            pc[0].abs().max(pc[1].abs()) <= RND * f64::EPSILON * pscale,
            "step {}: the shadow's total momentum {pc:?} is not removed by its own projection",
            n + 1
        );
        assert_eq!(
            s.shadow_calls,
            s.sample_calls,
            "step {}: the shadow's cadence ran {} times against the sample's {} substeps",
            n + 1,
            s.shadow_calls,
            s.sample_calls
        );
    }
}

#[test]
fn qa_m314_shadow_own_com() {
    let orbit = burrau();
    // Burrau's masses are (3, 4, 5)/12, unequal: the roundoff and the offset's CoM differ from the sample's.
    for n_renorm in [PROPOSED_N_RENORM, 7, 0] {
        let prm = params::<f64>(PROPOSED_DELTA_0, n_renorm);
        let (rec, ic) = record::<Absolute>(&orbit, 1500, prm, |b, x| b.shadow(x));
        check_own_com(&rec, &ic, PROPOSED_DELTA_0);
        let (rec, ic) = record::<Displacement>(&orbit, 1500, prm, |b, x| b.shadow(x));
        check_own_com(&rec, &ic, PROPOSED_DELTA_0);
    }
}

negative_control!(
    qa_m314_shadow_own_com,
    "a shadow that inherits the sample's projection (the sample's projected state plus the offset) must fail",
    expected = "the shadow's CoM not removed",
    {
        let prm = params::<f64>(PROPOSED_DELTA_0, 0);
        let (rec, ic) = record::<Absolute>(&burrau(), 50, prm, |_, x| offset(x, PROPOSED_DELTA_0));
        check_own_com(&rec, &ic, PROPOSED_DELTA_0);
    }
);

/// The shadow's monitor `E₀` is the energy of `x₀ + δ₀`, by `expected` (the shadow's state from the IC).
fn check_own_monitor(expected: fn(&Ic<f64>) -> State<f64>) {
    let orbit = burrau();
    let ic = ic_at::<f64>(&orbit.ic);
    let b = Benettin::<f64, Absolute>::new(
        &ic,
        CollisionRadius::none(),
        params(PROPOSED_DELTA_0, PROPOSED_N_RENORM),
    );
    let x = expected(&ic);
    let e_ref = energy(&x.m, &x.r, &x.p);
    let e_sample = energy(&ic.m, &ic.r, &ic.p);
    let sh = offset(
        &State {
            m: ic.m,
            r: ic.r,
            p: ic.p,
        },
        PROPOSED_DELTA_0,
    );
    // The shadow's and the sample's energies differ by far more than rounding, so the check tells them apart.
    assert!((energy(&sh.m, &sh.r, &sh.p) - e_sample).abs() > 1e6 * f64::EPSILON * e_sample.abs());
    let e0 = b.cadence().monitor_state().e_0();
    assert!(
        (e0 - e_ref).abs() <= RND * f64::EPSILON * e_ref.abs(),
        "the shadow's E₀ {e0} is not the energy of its own state {e_ref} (the sample's is {e_sample}): the monitor \
         is not the shadow's own"
    );
}

#[test]
fn qa_m314_shadow_own_monitor() {
    check_own_monitor(|ic| {
        offset(
            &State {
                m: ic.m,
                r: ic.r,
                p: ic.p,
            },
            PROPOSED_DELTA_0,
        )
    });
}

negative_control!(
    qa_m314_shadow_own_monitor,
    "a shadow monitored against the sample's E₀ must fail",
    expected = "the monitor is not the shadow's own",
    check_own_monitor(|ic| State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    })
);

// ── REQ-INT-043: renormalisation, cadence, the finalised read ───────────────────────────────────────────────────

/// After one renormalisation at the sample's `x` of a shadow that stood at `pre` with sum `s_before`, `b` has
/// `S = s_before + ln(‖δ⃗‖/δ₀)` and its shadow at `x + δ₀·δ⃗/‖δ⃗‖`, `δ⃗ = pre − x`, computed here in f64.
fn check_renorm<F: ShadowForm>(
    b: &Benettin<f64, F>,
    pre: &State<f64>,
    s_before: f64,
    x: &State<f64>,
    delta_0: f64,
) {
    let mut dr = [[0.0; 2]; 3];
    let mut dp = [[0.0; 2]; 3];
    for i in 0..3 {
        for k in 0..2 {
            dr[i][k] = pre.r[i][k] - x.r[i][k];
            dp[i][k] = pre.p[i][k] - x.p[i][k];
        }
    }
    let z = [[0.0; 2]; 3];
    let d = dist12(&z, &z, &dr, &dp);
    let s_expected = s_before + (d / delta_0).ln();
    // Each component of δ⃗ is a difference of states of size `scale`, so it is known to ε·scale absolute.
    let scale =
        x.r.iter()
            .chain(x.p.iter())
            .flatten()
            .fold(1.0f64, |a, &v| a.max(v.abs()));
    let tol_s = RND * f64::EPSILON * scale / d.min(delta_0) + RND * f64::EPSILON * s_expected.abs();
    assert!(
        (b.s_sum() - s_expected).abs() <= tol_s,
        "S = {} after one renormalisation, expected S + ln(δ/δ₀) = {s_expected}: the renormalisation's log is wrong",
        b.s_sum()
    );
    let sh = b.shadow(x);
    for i in 0..3 {
        for k in 0..2 {
            let want_r = x.r[i][k] + delta_0 * dr[i][k] / d;
            let want_p = x.p[i][k] + delta_0 * dp[i][k] / d;
            assert!(
                (sh.r[i][k] - want_r).abs() <= RND * f64::EPSILON * scale
                    && (sh.p[i][k] - want_p).abs() <= RND * f64::EPSILON * scale,
                "body {i} axis {k}: the shadow is not reset to x + δ₀·δ⃗/δ"
            );
        }
    }
    let sep = b.separation(x);
    assert!(
        (sep / delta_0 - 1.0).abs() <= RND * f64::EPSILON * scale / delta_0,
        "the separation after the reset is {sep}, not δ₀ = {delta_0}: the shadow is not reset to x + δ₀·δ⃗/δ"
    );
}

/// A sample state away from `x₀`: each component moved by its own amount, so `δ⃗` has a new direction.
fn moved(x0: &State<f64>) -> State<f64> {
    let mut x = *x0;
    for i in 0..3 {
        for k in 0..2 {
            x.r[i][k] += 0.01 * (1.0 + i as f64) * if k == 0 { 1.0 } else { -0.7 };
            x.p[i][k] += 0.003 * (2.0 - i as f64) * if k == 0 { -1.3 } else { 0.4 };
        }
    }
    x
}

fn renorm_form<F: ShadowForm>() {
    for d0 in [PROPOSED_DELTA_0, 1e-5, 1e-2] {
        // A shadow marched 5000 steps on Burrau, never renormalised, so δ⃗ has grown and turned; then one
        // renormalisation at the sample's state.
        let (rec, _) = record::<F>(&burrau(), 5000, params(d0, 0), |b, x| b.shadow(x));
        let mut b = rec.b;
        let x = rec.log.last().expect("5000 steps").end;
        let pre = b.shadow(&x);
        let s_before = b.s_sum();
        let grown = b.separation(&x) / d0;
        assert!(
            grown.ln().abs() > 0.1,
            "the fixture's separation did not grow ({grown})"
        );
        b.renormalise(&x);
        check_renorm(&b, &pre, s_before, &x, d0);
        // A second renormalisation at the same `x` finds δ = δ₀: S unchanged to rounding.
        let s1 = b.s_sum();
        b.renormalise(&x);
        assert!(
            (b.s_sum() - s1).abs() <= 1e3 * RND * f64::EPSILON / d0,
            "a renormalisation at δ = δ₀ changed S by {}",
            b.s_sum() - s1
        );
    }
}

/// The absolute form, its shadow `x'₀` fixed, renormalised at a sample state `x` away from `x₀`.
fn renorm_moved() {
    let ic = ic_at::<f64>(&burrau().ic);
    let x0 = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    for d0 in [PROPOSED_DELTA_0, 1e-5, 1e-2] {
        let mut b = Benettin::<f64, Absolute>::new(&ic, CollisionRadius::none(), params(d0, 1));
        let x = moved(&x0);
        let pre = b.shadow(&x);
        b.renormalise(&x);
        check_renorm(&b, &pre, 0.0, &x, d0);
    }
}

#[test]
fn qa_m314_renormalise() {
    renorm_moved();
    renorm_form::<Absolute>();
    renorm_form::<Displacement>();
}

negative_control!(
    qa_m314_renormalise,
    "a shadow that was never renormalised must fail",
    expected = "the renormalisation's log is wrong",
    {
        let ic = ic_at::<f64>(&burrau().ic);
        let x0 = State {
            m: ic.m,
            r: ic.r,
            p: ic.p,
        };
        let b = Benettin::<f64, Absolute>::new(&ic, CollisionRadius::none(), params(1e-3, 1));
        let x = moved(&x0);
        check_renorm(&b, &b.shadow(&x), 0.0, &x, 1e-3);
    }
);

/// `S` changes exactly at the end of each `k`-th macro-step, where the separation is reset to `δ₀`; elsewhere it is
/// unchanged, bit for bit; the changes number `steps / k`.
fn check_cadence(log: &[Step], k: u32, delta_0: f64) {
    let mut prev = 0.0f64;
    let mut changes = 0u32;
    for (i, s) in log.iter().enumerate() {
        let n = i as u32 + 1;
        let boundary = k != 0 && n.is_multiple_of(k);
        let changed = s.s_sum.to_bits() != prev.to_bits();
        assert_eq!(
            changed, boundary,
            "step {n}: S changed = {changed} where a renormalisation is due = {boundary} (n_renorm = {k}): not \
             renormalised every n_renorm steps"
        );
        if boundary {
            let scale = s
                .end
                .r
                .iter()
                .chain(s.end.p.iter())
                .flatten()
                .fold(1.0f64, |a, &x| a.max(x.abs()));
            assert!(
                (s.sep / delta_0 - 1.0).abs() <= RND * f64::EPSILON * scale / delta_0,
                "step {n}: the separation after the renormalisation is {}, not δ₀",
                s.sep
            );
            changes += 1;
        }
        prev = s.s_sum;
    }
    let steps = log.len() as u32;
    assert_eq!(changes, completed_renorms(steps, k));
    assert_eq!(changes, steps.checked_div(k).unwrap_or(0));
}

#[test]
fn qa_m314_renorm_cadence() {
    for k in [1, 7, 100, 0] {
        let (rec, _) = record::<Absolute>(&burrau(), 1000, params(PROPOSED_DELTA_0, k), |b, x| {
            b.shadow(x)
        });
        check_cadence(&rec.log, k, PROPOSED_DELTA_0);
        let (rec, _) =
            record::<Displacement>(&burrau(), 1000, params(PROPOSED_DELTA_0, k), |b, x| {
                b.shadow(x)
            });
        check_cadence(&rec.log, k, PROPOSED_DELTA_0);
    }
}

negative_control!(
    qa_m314_renorm_cadence,
    "a shadow renormalised every 7 steps read as if every 8 must fail",
    expected = "not renormalised every n_renorm steps",
    {
        let (rec, _) = record::<Absolute>(&burrau(), 200, params(PROPOSED_DELTA_0, 7), |b, x| {
            b.shadow(x)
        });
        check_cadence(&rec.log, 8, PROPOSED_DELTA_0);
    }
);

/// `completed_renorms` by `f` is `n / k`, 0 at `k = 0`.
fn check_completed(f: fn(u32, u32) -> u32) {
    for (n, k, want) in [
        (0, 300, 0),
        (299, 300, 0),
        (300, 300, 1),
        (599, 300, 1),
        (600, 300, 2),
        (50000, 300, 166),
        (65535, 1, 65535),
        (65535, 65535, 1),
        (65534, 65535, 0),
        (1000, 0, 0),
        (0, 0, 0),
    ] {
        assert_eq!(
            f(n, k),
            want,
            "completed_renorms({n}, {k}) is not step_count / n_renorm"
        );
    }
}

#[test]
fn qa_m314_completed_renorms() {
    check_completed(completed_renorms);
    check_completed(kernel::payload::completed_renorms);
}

negative_control!(
    qa_m314_completed_renorms,
    "a count that rounds up must fail",
    expected = "is not step_count / n_renorm",
    check_completed(|n, k| if k == 0 { 0 } else { n.div_ceil(k) })
);

/// A hand-filled FTLE sample: the sample at the origin, the shadow's body 0 at `(0.6, 0.8)·sep`, so `δ = sep`; `S`,
/// `step_count` and `state` as given.
fn hand<R: P>(sep: f64, s: f64, n: u32, state: u32) -> SimStateFTLEOf<R>
where
    f64: AsR<R>,
{
    let mut x = SimStateFTLEOf::<R>::default();
    x.r_sh[0] = [(0.6 * sep).to_r(), (0.8 * sep).to_r()];
    x.S = s.to_r();
    x.times = set_t_end_step(0, n);
    x.packed_a = set_state(0, state);
    x
}

type Reader<R> = fn(&SimStateFTLEOf<R>, &BenettinParams<R>, R) -> R;

fn check_read_known<R: P>(read: Reader<R>)
where
    f64: AsR<R>,
{
    let (d0, k, dt) = (0.25f64, 300u32, 1e-3f64);
    let prm = params::<R>(d0, k);
    let dtr: R = dt.to_r();
    // Valid: (S + ln(δ/δ₀)) / (n·dt), δ = 4δ₀ = 1, S = 2.
    for (n, state) in [(600u32, 0u32), (300, STATE_RUNNING), (451, 1)] {
        let s = hand::<R>(1.0, 2.0, n, state);
        let want = (2.0 + 4f64.ln()) / (f64::from(n) * dt);
        let got: f64 = read(&s, &prm, dtr).into();
        assert!(
            (got - want).abs() <= RND * R::EPS * want.abs(),
            "n = {n}: the read {got} is not (S + ln(δ/δ₀))/(n·dt) = {want}"
        );
    }
    // Invalid: NaN, no sentinel (R-253, R-254).
    let cases = [
        ("step_count = 0", hand::<R>(1.0, 0.0, 0, 0), k),
        (
            "no completed renormalisation",
            hand::<R>(1.0, 2.0, 299, 0),
            k,
        ),
        ("n_renorm = 0", hand::<R>(1.0, 2.0, 600, 0), 0),
        ("sim_failed", hand::<R>(1.0, 2.0, 600, STATE_SIM_FAILED), k),
        (
            "decode_failed",
            hand::<R>(1.0, 2.0, 600, STATE_DECODE_FAILED),
            k,
        ),
    ];
    for (what, s, kk) in cases {
        let got: f64 = read(&s, &params::<R>(d0, kk), dtr).into();
        assert!(
            got.is_nan(),
            "{what}: the read is {got}, not NaN: ftle_valid ignored"
        );
    }
}

#[test]
fn qa_m314_ftle_read_known() {
    check_read_known::<f64>(ftle_read::<f64>);
    check_read_known::<f32>(ftle_read::<f32>);
    // The generated read side agrees with the kernel's read, NaN included.
    for (n, state, sep, s) in [
        (600u32, 0u32, 1.0f64, 2.0f64),
        (299, 0, 1.0, 2.0),
        (0, 0, 1.0, 0.0),
        (600, STATE_SIM_FAILED, 1.0, 2.0),
        (1234, 1, 0.01, -3.0),
    ] {
        let x = hand::<f32>(sep, s, n, state);
        let rp = ReadParams {
            dt_macro: 1e-3,
            delta_0: 0.25,
            n_renorm: 300,
            horizon_steps: 65535,
        };
        let read = sim_state_from_ftle(&x, [0; 4], false, 0.0, false, [1.0 / 3.0; 3], &rp);
        let kernel_read = ftle_read::<f32>(&x, &params(0.25, 300), 1e-3);
        assert!(
            read.ftle.to_bits() == kernel_read.to_bits()
                || (read.ftle.is_nan() && kernel_read.is_nan()),
            "n = {n}, state {state}: the read side gives {} and the kernel {kernel_read}",
            read.ftle
        );
        assert_eq!(read.ftle_valid, !kernel_read.is_nan());
    }
}

negative_control!(
    qa_m314_ftle_read_known,
    "a read that ignores ftle_valid must fail",
    expected = "ftle_valid ignored",
    check_read_known::<f64>(|s, p, dt| {
        let d = dist12(&s.r, &s.p, &s.r_sh, &s.p_sh);
        let n = kernel::payload::tm_t_end_step(s.times);
        (s.S + (d / p.delta_0).ln()) / (f64::from(n) * dt)
    })
);

/// The read at `n` macro-steps of `orbit` by `read`, and plain `S/(n·dt)` there.
fn reads_at(orbit: &Orbit, n: u32, prm: BenettinParams<f64>, read: Reader<f64>) -> (f64, f64) {
    let ic = ic_at::<f64>(&orbit.ic);
    let k = key::<f64>(orbit.horizon, Some(n));
    let s = ftle_run::<f64, Absolute>(&ic, &k, prm);
    assert_eq!(kernel::payload::tm_t_end_step(s.times), n);
    (
        read(&s, &prm, k.dt_macro),
        s.S / (f64::from(n) * k.dt_macro),
    )
}

fn check_not_biased_low(orbit: &Orbit, read: Reader<f64>) {
    let k = 100u32;
    let prm = params::<f64>(PROPOSED_DELTA_0, k);
    let (mut finalised_gap, mut plain_gap) = (0.0, 0.0);
    for m in 2..=9u32 {
        let (at_boundary, plain_boundary) = reads_at(orbit, m * k, prm, read);
        // At a boundary the separation is δ₀: the finalised read is plain S/T.
        assert!(
            (at_boundary - plain_boundary).abs() <= 1e-9 * plain_boundary.abs().max(1.0),
            "{}: at the boundary {}, the read {at_boundary} is not S/T = {plain_boundary}",
            orbit.name,
            m * k
        );
        let (before, plain_before) = reads_at(orbit, m * k - 1, prm, read);
        finalised_gap += (before - at_boundary).abs();
        plain_gap += (plain_before - at_boundary).abs();
    }
    assert!(
        finalised_gap < plain_gap,
        "{}: one step before the boundaries the read is {finalised_gap:e} from the boundaries' reads, plain S/T is \
         {plain_gap:e}: the partial interval is not finalised",
        orbit.name
    );
}

#[test]
fn qa_m314_not_biased_low() {
    check_not_biased_low(&burrau(), ftle_read::<f64>);
    check_not_biased_low(
        &lagrange("Lagrange, equal masses", [1.0 / 3.0; 3], 30.0),
        ftle_read::<f64>,
    );
}

negative_control!(
    qa_m314_not_biased_low,
    "plain S/T as the read must fail",
    expected = "the partial interval is not finalised",
    check_not_biased_low(&burrau(), |s, _, dt| s.S
        / (f64::from(kernel::payload::tm_t_end_step(s.times)) * dt))
);

// ── dd test 10 at the proposal (REQ-INT-043, REQ-INT-076) ───────────────────────────────────────────────────────

/// `λ_T` from the stored sample of `orbit`'s full-horizon run at `(δ₀, k)`.
fn lambda_t<R: P>(orbit: &Orbit, t: f64, d0: f64, k: u32) -> f64
where
    f64: AsR<R>,
{
    let ic = ic_at::<R>(&orbit.ic);
    let key = key::<R>(t, None);
    let prm = params::<R>(d0, k);
    let s = ftle_run::<R, Absolute>(&ic, &key, prm);
    ftle_read(&s, &prm, key.dt_macro).into()
}

/// dd test 10's change, the proposal's measure: the largest `|λ' − λ|` over the variants relative to
/// `max(|λ|, 1/T)`.
fn check_invariance<R: P>(orbit: &Orbit, d0: f64, k: u32, word: &str)
where
    f64: AsR<R>,
{
    let t = orbit.horizon;
    let l = [
        lambda_t::<R>(orbit, t, d0, k),
        lambda_t::<R>(orbit, t, d0 / 2.0, k),
        lambda_t::<R>(orbit, t, d0, 2 * k),
        lambda_t::<R>(orbit, t, d0 / 2.0, 2 * k),
    ];
    // A λ_T that is not finite is an infinite change (the proposal's measure).
    let scale = l[0].abs().max(1.0 / t);
    let change = if l.iter().all(|x| x.is_finite()) {
        l[1..].iter().map(|x| (x - l[0]).abs()).fold(0.0, f64::max) / scale
    } else {
        f64::INFINITY
    };
    println!(
        "{} {word} (‖δ₀‖ = {d0}, n_renorm = {k}): λ_T {l:?}, change {change:.3}",
        orbit.name
    );
    assert!(
        change <= PROPOSED_TOLERANCE,
        "{} {word}: halving δ₀ / doubling n_renorm changed λ_T by {change:.3} > {PROPOSED_TOLERANCE}: λ_T not \
         invariant",
        orbit.name
    );
}

#[test]
fn qa_m314_dd10_invariance() {
    for orbit in [kepler_embedded(), burrau()] {
        check_invariance::<f64>(&orbit, PROPOSED_DELTA_0, PROPOSED_N_RENORM, "f64");
        check_invariance::<f32>(&orbit, PROPOSED_DELTA_0, PROPOSED_N_RENORM, "f32");
    }
}

negative_control!(
    qa_m314_dd10_invariance,
    "a δ₀ outside the linear regime (the separation the size of the system) must break the invariance",
    expected = "λ_T not invariant",
    check_invariance::<f64>(&burrau(), 2.0, PROPOSED_N_RENORM, "f64, nonlinear δ₀")
);

/// Kepler's `λ_T` falls as `T` doubles; Burrau's is positive and above Kepler's.
fn check_kepler_vs_burrau(kepler: &Orbit, chaotic: &Orbit) {
    let (d0, k) = (PROPOSED_DELTA_0, PROPOSED_N_RENORM);
    for (word, kh, kf, b) in [
        (
            "f64",
            lambda_t::<f64>(kepler, 25.0, d0, k),
            lambda_t::<f64>(kepler, 50.0, d0, k),
            lambda_t::<f64>(chaotic, 50.0, d0, k),
        ),
        (
            "f32",
            lambda_t::<f32>(kepler, 25.0, d0, k),
            lambda_t::<f32>(kepler, 50.0, d0, k),
            lambda_t::<f32>(chaotic, 50.0, d0, k),
        ),
    ] {
        println!(
            "{word}: Kepler λ_T(25) = {kh}, λ_T(50) = {kf}; {} λ_T(50) = {b}",
            chaotic.name
        );
        assert!(
            kf < kh,
            "{word}: Kepler's λ_T does not fall as T doubles ({kh} → {kf}): not ≈ 0"
        );
        assert!(
            b > 0.0 && b > kf,
            "{word}: {}'s λ_T {b} is not positive and above Kepler's {kf}: not large on Burrau",
            chaotic.name
        );
    }
}

#[test]
fn qa_m314_kepler_vs_burrau() {
    check_kepler_vs_burrau(&kepler_embedded(), &burrau());
}

negative_control!(
    qa_m314_kepler_vs_burrau,
    "the regular orbit in Burrau's place must fail",
    expected = "not large on Burrau",
    check_kepler_vs_burrau(&kepler_embedded(), &kepler_embedded())
);

// ── REQ-INT-077: δ₀'s direction ────────────────────────────────────────────────────────────────────────────────

/// A small deterministic generator of states (no external RNG).
fn lcg(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*seed >> 11) as f64) / ((1u64 << 53) as f64) * 2.0 - 1.0
}

fn check_direction<R: P>(off: fn(&State<R>, R) -> State<R>)
where
    f64: AsR<R>,
{
    let mut seed = 0x5eed_u64;
    for case in 0..500 {
        let m0 = 0.05 + 0.9 * (lcg(&mut seed) * 0.5 + 0.5);
        let m1 = (1.0 - m0) * (lcg(&mut seed) * 0.45 + 0.5);
        let m = [m0, m1, 1.0 - m0 - m1].map(AsR::to_r);
        let r = [[0; 2]; 3].map(|v: [i32; 2]| v.map(|_| (lcg(&mut seed) * 3.0).to_r()));
        let p = [[0; 2]; 3].map(|v: [i32; 2]| v.map(|_| lcg(&mut seed).to_r()));
        let x = State { m, r, p };
        let d0: R = [PROPOSED_DELTA_0, 1e-6, 0.1, 1e-3][case % 4].to_r();
        let o = off(&x, d0);
        for (i, ui) in U_R.iter().enumerate() {
            for (k, &uk) in ui.iter().enumerate() {
                let u: R = uk.to_r();
                // d0·u is exact (u is ±½ or 0), so the sum is one rounding: the plain sum here.
                let want = x.r[i][k] + d0 * u;
                assert!(
                    o.r[i][k].bits() == want.bits(),
                    "case {case}, body {i} axis {k}: r' = {:?}, x + ‖δ₀‖·u = {want:?}: not §3.8's direction",
                    o.r[i][k]
                );
                assert!(
                    o.p[i][k].bits() == x.p[i][k].bits(),
                    "case {case}, body {i} axis {k}: a momentum moved: not §3.8's direction"
                );
            }
            assert_eq!(o.m[i].bits(), x.m[i].bits(), "the masses changed");
        }
    }
}

#[test]
fn qa_m314_delta0_direction() {
    check_direction::<f64>(offset::<f64>);
    check_direction::<f32>(offset::<f32>);
    // ‖u‖ = 1 and Σu = 0: the offset of the zero state has norm exactly ‖δ₀‖ and sums to zero per component.
    let zero = State {
        m: [1.0 / 3.0; 3],
        r: [[0.0f64; 2]; 3],
        p: [[0.0; 2]; 3],
    };
    for d0 in [PROPOSED_DELTA_0, 1e-6, 0.5] {
        let o = offset(&zero, d0);
        let z = [[0.0; 2]; 3];
        let n = dist12(&z, &z, &o.r, &o.p);
        assert!(
            (n / d0 - 1.0).abs() <= 4.0 * f64::EPSILON,
            "‖δ₀·u‖ = {n} ≠ δ₀ = {d0}"
        );
        for k in 0..2 {
            assert_eq!(o.r[0][k] + o.r[1][k] + o.r[2][k], 0.0, "Σu ≠ 0 on axis {k}");
        }
    }
    // The shadow a sample starts with is the offset of its IC, in either form, independent of the sample's masses.
    for orbit in [burrau(), kepler_embedded()] {
        let ic = ic_at::<f64>(&orbit.ic);
        let x = State {
            m: ic.m,
            r: ic.r,
            p: ic.p,
        };
        let want = offset(&x, PROPOSED_DELTA_0);
        let prm = params::<f64>(PROPOSED_DELTA_0, PROPOSED_N_RENORM);
        let a = Benettin::<f64, Absolute>::new(&ic, CollisionRadius::none(), prm).shadow(&x);
        assert_eq!((bits6(&a.r), bits6(&a.p)), (bits6(&want.r), bits6(&want.p)));
        let d = Benettin::<f64, Displacement>::new(&ic, CollisionRadius::none(), prm).shadow(&x);
        for i in 0..3 {
            for k in 0..2 {
                assert!(
                    (d.r[i][k] - want.r[i][k]).abs()
                        <= 2.0 * f64::EPSILON * want.r[i][k].abs().max(1.0)
                );
                assert_eq!(d.p[i][k], want.p[i][k]);
            }
        }
    }
}

negative_control!(
    qa_m314_delta0_direction,
    "a direction that also moves a momentum must fail",
    expected = "not §3.8's direction",
    check_direction::<f64>(|x, d0| {
        let mut o = offset(x, d0);
        o.p[2][0] += d0 * 0.5;
        o
    })
);

// ── REQ-VAL-038: the known exponent and the forms ───────────────────────────────────────────────────────────────

/// The largest real part of the roots of `s⁴ + s² + c = 0`, `c = (27/4)β`, from the roots: `s² = (−1 ± √(1 − 4c))/2`,
/// then `s = ±√(s²)`, by complex square roots.
fn largest_real_root(beta: f64) -> f64 {
    let c = 27.0 / 4.0 * beta;
    let disc = 1.0 - 4.0 * c;
    // s² as complex numbers (re, im).
    let s2: [(f64, f64); 2] = if disc >= 0.0 {
        [
            ((-1.0 + disc.sqrt()) / 2.0, 0.0),
            ((-1.0 - disc.sqrt()) / 2.0, 0.0),
        ]
    } else {
        [(-0.5, (-disc).sqrt() / 2.0), (-0.5, -(-disc).sqrt() / 2.0)]
    };
    // Re √z = √((|z| + Re z)/2); the roots come in ± pairs, so the largest real part is the non-negative one.
    s2.iter()
        .map(|&(a, b)| {
            let modulus = (a * a + b * b).sqrt();
            ((modulus + a) / 2.0).max(0.0).sqrt()
        })
        .fold(0.0, f64::max)
}

fn check_exponent(exponent: fn([f64; 3]) -> f64) {
    let masses = [
        [1.0 / 3.0; 3],
        [0.5, 0.3, 0.2],
        [0.98, 0.01, 0.01],
        [0.9, 0.05, 0.05],
        [0.6, 0.39, 0.01],
        [0.999, 0.0005, 0.0005],
    ];
    for m in masses {
        let mt: f64 = m.iter().sum();
        let beta = (m[0] * m[1] + m[0] * m[2] + m[1] * m[2]) / (mt * mt);
        assert!(
            (routh_beta(m) - beta).abs() <= 4.0 * f64::EPSILON,
            "β({m:?})"
        );
        let want = largest_real_root(beta);
        let got = exponent(m);
        assert!(
            (got - want).abs() <= 1e3 * f64::EPSILON,
            "masses {m:?} (27β = {}): the known exponent {got} is not Routh's {want}",
            27.0 * beta
        );
        if 27.0 * beta < 1.0 {
            assert_eq!(
                got, 0.0,
                "masses {m:?}: a linearly stable Lagrange orbit has exponent 0"
            );
        }
    }
    // Equal masses: 27β = 9, so s² = (−1 ± i√8)/2 and the largest real part is 1/√2.
    assert!((exponent([1.0 / 3.0; 3]) - 0.5f64.sqrt()).abs() <= 1e3 * f64::EPSILON);
}

#[test]
fn qa_m314_lagrange_exponent() {
    check_exponent(lagrange_exponent);
}

negative_control!(
    qa_m314_lagrange_exponent,
    "the exponent without the inner root, ½√(27β − 1), must fail",
    expected = "is not Routh's",
    check_exponent(|m| {
        let x = 27.0 * routh_beta(m);
        if x > 1.0 {
            0.5 * (x - 1.0).sqrt()
        } else {
            0.0
        }
    })
);

/// How the payload's FTLE sample is integrated.
type Payload = fn(&Ic<f32>, &WrapperUniforms<f32>, BenettinParams<f32>) -> SimStateFTLEOf<f32>;

/// The payload's form, by `payload`, is the absolute one, bit for bit (the forms' shared start is
/// `qa_m314_delta0_direction`'s, their shared reset `qa_m314_renormalise`'s).
fn check_forms(payload: Payload) {
    // The payload stays in the absolute form until REQ-PAY-085's criterion is met (payload §8).
    for orbit in [
        burrau(),
        lagrange("Lagrange, (0.5, 0.3, 0.2)", [0.5, 0.3, 0.2], 30.0),
    ] {
        let icf = ic_at::<f32>(&orbit.ic);
        let k = key::<f32>(orbit.horizon, Some(3000));
        let prm = params::<f32>(PROPOSED_DELTA_0, PROPOSED_N_RENORM);
        let stored = payload(&icf, &k, prm);
        let abs = ftle_run::<f32, Absolute>(&icf, &k, prm);
        assert!(
            bits6(&stored.r_sh) == bits6(&abs.r_sh)
                && bits6(&stored.p_sh) == bits6(&abs.p_sh)
                && stored.S.to_bits() == abs.S.to_bits(),
            "{}: the payload's shadow is not the absolute form's: switched without the criterion",
            orbit.name
        );
    }
}

#[test]
fn qa_m314_forms_one_map() {
    check_forms(|ic, k, prm| integrate_ftle::<Yoshida4, f32>(ic, k, CollisionRadius::none(), prm));
}

negative_control!(
    qa_m314_forms_one_map,
    "a payload holding the displacement form's shadow must fail",
    expected = "switched without the criterion",
    check_forms(ftle_run::<f32, Displacement>)
);
