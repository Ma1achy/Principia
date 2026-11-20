//! The live shape readout in the march (`kernel::driver::shape`, `kernel::driver::closure`, the wrapper's
//! `integrate` and `integrate_resumed_with`; dd_integrator §3.7; payload §1, §2; R-37, R-389, R-392, R-397, R-545):
//! - `theta_ref_code` (REQ-INT-087): every code round-trips, `encode(decode(c)) = c`, and decodes into `(−π, π)`; the
//!   encode's error on the circle is at most `π/65535 + 4·ulp_Real(2π)` on a million random longitudes and on the edge
//!   cases (±π, ±0, the cut, the rounding edges), with the encode written in either order; it never yields 0xFFFF.
//! - `theta_hold_state` (REQ-INT-088): through the march, on a real pole passage and from an IC inside the disc: a fresh
//!   sample's `_reserved` is 0xFFFF; it is 0xFFFF outside the disc after every step; entry writes the code of the last
//!   longitude outside; inside, `θ̃` and the code hold; exit adds `wrap(exit − decode(code))` and writes 0xFFFF; an IC
//!   inside adds nothing at its first exit; ±π adds +π; the inside test is recomputed from the stored state on resume;
//!   descriptor bits 10–14 stay zero and bit 15 is the departed bit, the latched departure, whatever the hold does.
//! - `theta_hold_disc_test` (R-547): the march's disc test on `Wide`s agrees with the float form away from the edge,
//!   is invariant under power-of-two scales far past f32's normal range, where the float form's intermediates are
//!   subnormal or zero, and under the mirror.
//! - `theta_resume_split` (REQ-INT-089): one call against `k ∈ {2, 3, 7}` resumed calls, at f32 and f64, split on a
//!   pole passage's entry step, inside it and on its exit step, and inside the disc for an IC that starts there: `θ̃`,
//!   `_reserved`, the closure registers and the state are bit-identical; the resume's step bounds, from the start and
//!   part-way in; the stored `saturated` bit kept through a resume.
//! - `march_loop_bound`: the march's loop stops at its end with a cadence that is never done, fresh and resumed, under a
//!   cadence that panics at the first macro-step past it, so a broken loop condition fails at once.
//! - `closure_registers` (REQ-PAY-043): after every horizon of a figure-eight run, `closure_min` is the offline minimum
//!   of the chord `|n̂(t) − n̂(0)|` over the steps from departure on and `closure_step` its first argmin.
//! - `closure_departure` (REQ-PAY-056): the figure-eight's `closure_min` is near 0 at `closure_step` near its shape
//!   period over `dt_macro`; a closure before departure is ignored, however near; the departure test's edges; an equal
//!   chord after departure does not move the step.
//! - `closure_non_finite_chord`: a non-finite chord is no sample, before departure (the registers stay unset, the stored
//!   minimum `+∞`) and after it (the minimum and its step kept).
//! - `theta_unwrap_real_orbit` (REQ-INT-082, R-580): on a circulating figure-eight with a rotation added, `θ̃` has no
//!   2π jumps, `orbit_count` matches a hand count, `retrograde` the hand-counted direction along the equator, and the
//!   mirror's run (`y → −y` on every position and momentum) has `θ̃` and `retrograde` bitwise identical and `L_z`
//!   bitwise opposite at every step; with R-433's near-pole arc report.
//!
//! Each test registers its negative control (R-176). Each check takes what it tests as an argument, so its control
//! runs the same check on a faulty one. `r_pole` and `δ_dep` are the register's, read through the kernel's generated
//! constants (`r_pole`, `delta_dep`), never literals here. The figure-eight's period is `ledger::fixtures`'.

use std::cell::Cell;
use std::f64::consts::{PI, TAU};

use kernel::decode::Ic;
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::closure::Closure;
use kernel::driver::shape::{
    decode, delta_dep, encode, r_pole, theta_hold, Readout, ReadoutReal, NO_REFERENCE,
};
use kernel::driver::substep::{d2_quantised, NsubThresholds, Quantise};
use kernel::driver::welford::chord2;
use kernel::driver::wrapper::{
    integrate, integrate_resumed_with, integrate_with, WrapperReal, WrapperUniforms,
};
use kernel::generated::nsub_thresholds::NSUB_TABLE_ROWS;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::{Geom, State};
use kernel::payload::{
    orbit_count, retrograde, sd_departed, sd_saturated, sd_state, set_saturated, tm_t_end_step,
    SimStateBaseOf, STATE_BOUNDED, STATE_RUNNING,
};
use kernel::shape::{config_in_disc, disc_input, in_disc, landmarks, longitude, shape, wrap};
use spirv_std::glam::UVec4;
use spirv_std::num_traits::{Float, NumCast};
use validation::decode_f32_eps::SplitMix;
use validation::drift::figure_eight;
use validation::measure::delta_dep::eight_period;
use validation::negative_control;

/// A uniform `[0, 1)` draw from the stream, its top 53 bits.
trait Unit {
    fn next_f64(&mut self) -> f64;
}

impl Unit for SplitMix {
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// The precisions the march runs at: f32 and f64, each widened exactly to f64 for the checks.
trait Prec: WrapperReal + ReadoutReal + Into<f64> {}

impl<R: WrapperReal + ReadoutReal + Into<f64>> Prec for R {}

/// `x` at `R`, rounded once.
fn real<R: Prec>(x: f64) -> R {
    <R as NumCast>::from(x).expect("a finite f64")
}

/// `x`, widened exactly.
fn w<R: Prec>(x: R) -> f64 {
    x.into()
}

/// `x`'s bits, widened: f32 to f64 is exact, so equal bits at f64 are equal bits at f32.
fn bits<R: Prec>(x: R) -> u64 {
    w(x).to_bits()
}

/// A sim key with `N_sub = 1` throughout (every threshold 0), `dt_macro` and `horizon_steps` as given.
fn uniforms<R: Prec>(dt_macro: f64, horizon_steps: u32) -> WrapperUniforms<R> {
    WrapperUniforms {
        dt_macro: real(dt_macro),
        horizon_steps,
        n_max: 1,
        thresholds: NsubThresholds {
            rows: [UVec4::splat(0f32.to_bits()); NSUB_TABLE_ROWS],
        },
    }
}

/// `ulp_R(2π)`: `2π` lies in `[4, 8)`, so its ulp is `2^(2 − p)`, `p` the precision's mantissa bits.
fn ulp_tau<R: Prec>() -> f64 {
    let eps: f64 = R::epsilon().into();
    4.0 * eps
}

/// R-397's bound on the encode's error: `π/65535 + 4·ulp_R(2π)`.
fn code_bound<R: Prec>() -> f64 {
    PI / 65535.0 + 4.0 * ulp_tau::<R>()
}

/// `a − b` on the circle, in `[0, π]`.
fn circle(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(TAU);
    d.min(TAU - d)
}

// ── theta_ref_code ──────────────────────────────────────────────────────────────────────────────────────────────────

/// An encode under test: the kernel's, or a control's.
type Encode<R> = fn(R) -> u32;

/// The kernel's encode with its multiply in the other order, `(λ'·N)/(2π)`: R-397's bound holds whatever the order.
fn encode_reordered<R: Prec>(lon: R) -> u32 {
    let tau = R::PI() + R::PI();
    let shifted = if lon < R::zero() { lon + tau } else { lon };
    let half = R::one() / (R::one() + R::one());
    let c = Float::floor(shifted * R::count(65535) / tau + half).whole();
    if c >= 65535 {
        0
    } else {
        c
    }
}

/// The longitudes the encode is checked on: the edges (±π, ±0, the smallest normals and subnormals either side of the
/// cut, the rounding edges `(c ± ½)·2π/N` and a few ulps either side of them for codes across the range), then a
/// million uniform in `[−π, π]`.
fn longitudes<R: Prec>() -> Vec<R> {
    let mut out: Vec<R> = vec![
        R::PI(),
        -R::PI(),
        R::zero(),
        -R::zero(),
        R::min_positive_value(),
        -R::min_positive_value(),
        R::epsilon(),
        -R::epsilon(),
    ];
    for c in [0u32, 1, 2, 16383, 32766, 32767, 32768, 49151, 65533, 65534] {
        for half in [-0.5, 0.5] {
            let edge = (<f64 as From<_>>::from(c) + half) * TAU / 65535.0;
            let edge = if edge > PI { edge - TAU } else { edge };
            let mut x: R = real(edge);
            for _ in 0..3 {
                x = next_down(x);
            }
            for _ in 0..7 {
                out.push(x);
                x = next_up(x);
            }
        }
    }
    let mut rng = SplitMix::new(397);
    out.extend((0..1_000_000).map(|_| real::<R>(PI * (2.0 * rng.next_f64() - 1.0))));
    out.retain(|x| Float::abs(*x) <= R::PI());
    out
}

/// The next float above `x`.
fn next_up<R: Prec>(x: R) -> R {
    let step = Float::max(Float::abs(x) * R::epsilon(), R::min_positive_value());
    let y = x + step / (R::one() + R::one());
    if y == x {
        x + step
    } else {
        y
    }
}

/// The next float below `x`.
fn next_down<R: Prec>(x: R) -> R {
    -next_up(-x)
}

/// Checks R-397's code at `R` through `enc`, against the kernel's decode: the encode's error on the circle is within
/// [`code_bound`] on [`longitudes`] and is never the sentinel; then every code round-trips and decodes into `(−π, π)`.
/// Returns the worst error.
fn check_code<R: Prec>(enc: Encode<R>) -> f64 {
    let bound = code_bound::<R>();
    let mut worst = 0.0f64;
    for lon in longitudes::<R>() {
        let c = enc(lon);
        assert!(
            c != NO_REFERENCE,
            "the encode of {:e} is the sentinel 0xFFFF",
            w(lon)
        );
        assert!(c < 65535, "the encode of {:e} is {c}, out of range", w(lon));
        let err = circle(decode::<R>(c).into(), lon.into());
        assert!(
            err <= bound,
            "the code {c} of the longitude {:e} decodes {err:e} from it, which exceeds the bound {bound:e} \
             (π/65535 + 4·ulp(2π))",
            w(lon)
        );
        worst = worst.max(err);
    }
    for c in 0..65535u32 {
        let lon = decode::<R>(c);
        let x: f64 = lon.into();
        assert!(
            x > -PI && x < PI,
            "the code {c} decodes to {x}, outside (−π, π)"
        );
        assert_eq!(enc(lon), c, "the code {c} does not round-trip through {x}");
    }
    worst
}

#[test]
fn theta_ref_code() {
    let w32 = check_code::<f32>(encode::<f32>);
    let w64 = check_code::<f64>(encode::<f64>);
    let r32 = check_code::<f32>(encode_reordered::<f32>);
    let r64 = check_code::<f64>(encode_reordered::<f64>);
    // The comparisons' edges: `c ≥ N` at the last code and just past it, `λ < 0` at ±0, decode's `> π` either side.
    assert_eq!(encode::<f64>(decode(65534)), 65534, "the last code");
    assert_eq!(
        encode::<f64>(-f64::MIN_POSITIVE),
        0,
        "just below 0 wraps to code 0, never 65535"
    );
    assert_eq!(
        encode::<f32>(-f32::MIN_POSITIVE),
        0,
        "just below 0 wraps to code 0 at f32"
    );
    assert_eq!((encode::<f64>(0.0), encode::<f64>(-0.0)), (0, 0), "±0");
    assert!(
        decode::<f64>(32767) > 0.0 && decode::<f64>(32768) < 0.0,
        "decode's cut at π"
    );
    assert!(
        decode::<f32>(32767) > 0.0 && decode::<f32>(32768) < 0.0,
        "decode's cut at π, f32"
    );
    eprintln!(
        "theta_ref_code: worst encode error f32 {w32:.3e} (reordered {r32:.3e}) of bound {:.3e}; f64 {w64:.3e} \
         (reordered {r64:.3e}) of bound {:.3e}",
        code_bound::<f32>(),
        code_bound::<f64>()
    );
}

negative_control!(
    theta_ref_code,
    "a truncating encode, with no + ½, is up to a whole step off, past R-397's bound",
    expected = "exceeds the bound",
    {
        check_code::<f64>(|lon: f64| {
            let shifted = if lon < 0.0 { lon + TAU } else { lon };
            ((shifted * (65535.0 / TAU)).floor() as u32) % 65535
        });
    }
);

// ── The pole-passage fixtures ───────────────────────────────────────────────────────────────────────────────────────

/// The passage fixtures' macro-step: the shape moves about `3·10⁻⁴` rad a step near the pole, far below
/// `π·r_pole`, so the disc takes tens of steps to cross.
const PASSAGE_DT: f64 = 0.002;

/// The passage fixtures' horizon.
const PASSAGE_STEPS: u32 = 200;

/// The steps the passage IC is integrated back from the pivot.
const PASSAGE_BACK: u32 = 100;

/// The pivot: equal masses near the equilateral triangle, inside the pole disc (`ρ` about `0.004`), its momenta
/// deforming the shape, `Σp = 0`, the centre of mass at the origin.
fn pivot<R: Prec>() -> Ic<R> {
    let shift = 0.004;
    let r = [90.0f64, 210.0, 330.0].map(|a: f64| {
        let a = a.to_radians();
        [0.8 * a.cos(), 0.8 * a.sin()]
    });
    let r = [
        [r[0][0] - shift / 3.0, r[0][1]],
        [r[1][0] - shift / 3.0, r[1][1]],
        [r[2][0] + 2.0 * shift / 3.0, r[2][1]],
    ];
    let p = [[0.04, 0.01], [-0.04, 0.01], [0.0, -0.02]];
    Ic {
        m: [real(1.0 / 3.0); 3],
        r: r.map(|v| v.map(real)),
        p: p.map(|v| v.map(real)),
    }
}

/// The passage IC: the pivot integrated back [`PASSAGE_BACK`] steps (the march on the time-reversed pivot, its
/// momenta reversed back), so the march from it crosses the pole disc near step [`PASSAGE_BACK`].
fn passage<R: Prec>() -> Ic<R> {
    let pivot = pivot::<R>();
    let back = Ic {
        p: pivot.p.map(|v| v.map(|x| -x)),
        ..pivot
    };
    let s = integrate::<Kdk, R>(&back, &uniforms(PASSAGE_DT, PASSAGE_BACK));
    Ic {
        m: pivot.m,
        r: s.r,
        p: s.p.map(|v| v.map(|x| -x)),
    }
}

/// The `SimState` after each horizon `0 … steps` of `ic`, each a fresh call: the state a `k`-step horizon returns, which
/// the march reaches identically on the way to a longer one.
fn prefixes<R: Prec>(ic: &Ic<R>, dt: f64, steps: u32) -> Vec<SimStateBaseOf<R>> {
    (0..=steps)
        .map(|k| integrate::<Kdk, R>(ic, &uniforms(dt, k)))
        .collect()
}

/// Whether the stored state `s` of masses `m` lies inside the pole disc, by the march's test.
fn inside<R: Prec>(s: &SimStateBaseOf<R>, m: [R; 3]) -> bool {
    config_in_disc(s.r, m, r_pole())
}

/// The first step inside the disc after one outside, and the first outside after it, in `series`.
fn passage_steps<R: Prec>(series: &[SimStateBaseOf<R>], m: [R; 3]) -> (usize, usize) {
    let ins: Vec<bool> = series.iter().map(|s| inside(s, m)).collect();
    let entry = (1..ins.len())
        .find(|&k| ins[k] && !ins[k - 1])
        .expect("the passage fixture enters the disc");
    let exit = (entry..ins.len())
        .find(|&k| !ins[k])
        .expect("the passage fixture leaves the disc");
    (entry, exit)
}

// ── theta_hold_state ────────────────────────────────────────────────────────────────────────────────────────────────

/// What a march stored after one step: `θ̃`, `_reserved`, `packed_a` and the positions.
#[derive(Clone, Copy, Debug)]
struct Stored<R> {
    theta: R,
    code: u32,
    packed_a: u32,
    r: [[R; 2]; 3],
}

fn stored<R: Prec>(s: &SimStateBaseOf<R>) -> Stored<R> {
    Stored {
        theta: s.theta,
        code: <u32 as From<_>>::from(s._reserved),
        packed_a: s.packed_a,
        r: s.r,
    }
}

/// Bits `lo … lo + width − 1` of `w`.
fn field(w: u32, lo: u32, width: u32) -> u32 {
    (w >> lo) & ((1 << width) - 1)
}

/// Checks R-389, R-392 and R-397's hold on the march's `series` of masses `m`, step by step from the stored values
/// alone (`label` names the fixture), and that the hold leaves the descriptor's bits 10–14 zero and bit 15 the latched
/// departure. Returns how many steps were entries, holds inside, exits with a code and exits with none.
fn check_hold<R: Prec>(series: &[Stored<R>], m: [R; 3], label: &str) -> [usize; 4] {
    let n: Vec<[R; 3]> = series.iter().map(|s| shape(s.r, m)).collect();
    let ins: Vec<bool> = series
        .iter()
        .map(|s| config_in_disc(s.r, m, r_pole()))
        .collect();
    assert_eq!(
        series[0].code, NO_REFERENCE,
        "{label}: a fresh sample's _reserved is not 0xFFFF"
    );
    assert_eq!(bits(series[0].theta), 0, "{label}: θ̃(0) is not 0");
    let delta = delta_dep::<R>();
    let mut departed = false;
    let mut cases = [0; 4];
    for k in 1..series.len() {
        let (prev, now) = (series[k - 1], series[k]);
        let (lon, lon_prev) = (longitude(n[k]), longitude(n[k - 1]));
        departed = departed || chord2(n[k], n[0]) > delta * delta;
        assert_eq!(
            field(now.packed_a, 10, 5),
            0,
            "{label}, step {k}: descriptor bits 10–14 are set"
        );
        assert_eq!(
            sd_departed(now.packed_a),
            departed,
            "{label}, step {k}: descriptor bit 15 is not the latched departure"
        );
        if !ins[k] {
            assert_eq!(
                now.code, NO_REFERENCE,
                "{label}, step {k}: outside the disc, _reserved is not 0xFFFF"
            );
        }
        let want = match (ins[k - 1], ins[k]) {
            (false, false) => prev.theta + wrap(lon - lon_prev),
            (false, true) => {
                assert_eq!(
                    now.code,
                    encode(lon_prev),
                    "{label}, step {k}: entry wrote {:#06x}, not the code of the last longitude outside",
                    now.code
                );
                cases[0] += 1;
                prev.theta
            }
            (true, true) => {
                assert_eq!(
                    now.code, prev.code,
                    "{label}, step {k}: the code moved inside the disc"
                );
                cases[1] += 1;
                prev.theta
            }
            (true, false) if prev.code == NO_REFERENCE => {
                cases[3] += 1;
                prev.theta
            }
            (true, false) => {
                cases[2] += 1;
                prev.theta + wrap(lon - decode::<R>(prev.code))
            }
        };
        assert_eq!(
            bits(now.theta),
            bits(want),
            "{label}, step {k}: θ̃ is {:e}, not {:e} (inside {} → {})",
            w(now.theta),
            w(want),
            ins[k - 1],
            ins[k]
        );
    }
    cases
}

/// A march under test: the `Stored` after every step `0 … steps` of `ic`.
type March<R> = fn(&Ic<R>, f64, u32) -> Vec<Stored<R>>;

fn kernel_march<R: Prec>(ic: &Ic<R>, dt: f64, steps: u32) -> Vec<Stored<R>> {
    prefixes(ic, dt, steps).iter().map(stored).collect()
}

/// Checks the hold on the passage and on the IC inside, at `R`, through `march`; and the one-step resume from a stored
/// state inside the disc. Returns the cases seen.
fn check_hold_fixtures<R: Prec>(march: March<R>) -> [[usize; 4]; 2] {
    let ic = passage::<R>();
    let series = march(&ic, PASSAGE_DT, PASSAGE_STEPS);
    let through = check_hold(&series, ic.m, "the passage");
    assert!(
        through[0] >= 1 && through[1] >= 1 && through[2] >= 1,
        "the passage does not enter, hold and leave the disc: {through:?}"
    );
    let ic = pivot::<R>();
    assert!(
        config_in_disc(ic.r, ic.m, r_pole()),
        "the pivot is not inside the disc"
    );
    let series = march(&ic, PASSAGE_DT, PASSAGE_STEPS);
    let from_inside = check_hold(&series, ic.m, "the IC inside");
    assert!(
        from_inside[3] == 1,
        "the IC inside does not leave the disc once with no reference: {from_inside:?}"
    );
    [through, from_inside]
}

/// One more step from the stored `s` of `ic`, resumed.
fn resume_one<R: Prec>(
    ic: &Ic<R>,
    s: &SimStateBaseOf<R>,
    dt: f64,
    horizon: u32,
) -> SimStateBaseOf<R> {
    let mut cadence = WrapperCadence::new(&ic.m, &s.r, &s.p);
    integrate_resumed_with::<Kdk, R, _>(ic, &uniforms(dt, horizon), &mut cadence, s, 1)
}

/// The inside test is recomputed from the stored positions: from each stored state inside the disc, the IC inside's
/// (no reference) and the passage's (a reference), one resumed step is the uninterrupted march's.
fn check_resume_inside<R: Prec>() {
    for ic in [pivot::<R>(), passage::<R>()] {
        let series = prefixes(&ic, PASSAGE_DT, PASSAGE_STEPS);
        for k in 0..series.len() - 1 {
            if !inside(&series[k], ic.m) {
                continue;
            }
            let next = resume_one(&ic, &series[k], PASSAGE_DT, PASSAGE_STEPS);
            assert_eq!(
                (bits(next.theta), next._reserved),
                (bits(series[k + 1].theta), series[k + 1]._reserved),
                "resumed inside the disc at step {k}, the next step's θ̃ and _reserved are not the march's"
            );
        }
    }
}

#[test]
fn theta_hold_state() {
    let c64 = check_hold_fixtures::<f64>(kernel_march::<f64>);
    let c32 = check_hold_fixtures::<f32>(kernel_march::<f32>);
    check_resume_inside::<f64>();
    check_resume_inside::<f32>();
    // Exactly ±π from the decoded reference adds +π: code 0 decodes to 0, and the exit point lies at longitude
    // `atan2(±0, −ρ) = ±π`.
    for v in [0.0f64, -0.0] {
        let exit = [-0.5, v, 0.75f64.sqrt()];
        let (theta, code) = theta_hold(1.0f64, 0, [0.0, 0.0, 1.0], true, exit, false);
        assert_eq!(
            theta.to_bits(),
            (1.0 + PI).to_bits(),
            "an exit at {v:?}·π from the reference adds +π"
        );
        assert_eq!(code, NO_REFERENCE, "exit writes 0xFFFF");
    }
    eprintln!(
        "theta_hold_state: [entries, steps held inside, exits with a code, exits with none]: passage f64 {:?}, f32 \
         {:?}; IC inside f64 {:?}, f32 {:?}",
        c64[0], c32[0], c64[1], c32[1]
    );
}

/// A march whose hold freezes the first longitude inside the disc, not the last outside: the kernel's states, `θ̃`
/// and `_reserved` re-stepped with that entry.
#[cfg(feature = "controls")]
fn first_inside_march(ic: &Ic<f64>, dt: f64, steps: u32) -> Vec<Stored<f64>> {
    let series = kernel_march(ic, dt, steps);
    let mut out = vec![series[0]];
    let (mut theta, mut code) = (0.0, NO_REFERENCE);
    for k in 1..series.len() {
        let (a, b) = (shape(series[k - 1].r, ic.m), shape(series[k].r, ic.m));
        let (ia, ib) = (
            config_in_disc(series[k - 1].r, ic.m, r_pole()),
            config_in_disc(series[k].r, ic.m, r_pole()),
        );
        (theta, code) = theta_hold(theta, code, a, ia, b, ib);
        if ib && !ia {
            code = encode(longitude(b));
        }
        out.push(Stored {
            theta,
            code,
            ..series[k]
        });
    }
    out
}

negative_control!(
    theta_hold_state,
    "a hold that freezes the first longitude inside the disc, not the last outside, fails the entry check",
    expected = "not the code of the last longitude outside",
    {
        check_hold_fixtures::<f64>(first_inside_march);
    }
);

// ── theta_hold_disc_test ────────────────────────────────────────────────────────────────────────────────────────────

/// A disc test under check: the march's, or a control's.
type DiscTest<R> = fn([[R; 2]; 3], [R; 3], R) -> bool;

/// Checks `test` at `R` on random configurations near the pole, both sides of the disc: it is the float form's
/// ([`disc_input`], [`in_disc`]) wherever that is clear of the edge by a margin of `10⁻³` of `r_pole²·I²`; and it is
/// unchanged by the mirror and by scales `2^e` for `e` in `[−100, 100]`. Returns how many came out inside.
fn check_disc<R: Prec>(test: DiscTest<R>) -> usize {
    let mut rng = SplitMix::new(547);
    let pole: R = r_pole();
    let mut inside = 0;
    for _ in 0..4000 {
        let m: [f64; 3] = {
            let w = [
                0.1 + rng.next_f64(),
                0.1 + rng.next_f64(),
                0.1 + rng.next_f64(),
            ];
            let s = w[0] + w[1] + w[2];
            [w[0] / s, w[1] / s, w[2] / s]
        };
        // A configuration at the pole for these masses, `ρ̃ ⟂ λ̃` and `|ρ̃| = |λ̃|`, its `λ` moved by up to 3% of its
        // length, so up to a few `r_pole` on the sphere and both sides of the disc occur.
        let mr: [R; 3] = m.map(real);
        let m01 = m[0] + m[1];
        let rho = 1.0 / (m[0] * m[1] / m01).sqrt();
        let lambda = 1.0 / (m[2] * m01).sqrt();
        let (dx, dy) = (
            0.03 * (2.0 * rng.next_f64() - 1.0),
            0.03 * (2.0 * rng.next_f64() - 1.0),
        );
        let base = [
            [-m[1] / m01 * rho, 0.0],
            [m[0] / m01 * rho, 0.0],
            [dx * lambda, (1.0 + dy) * lambda],
        ];
        let r: [[R; 2]; 3] = base.map(|v| v.map(real));
        let got = test(r, mr, pole);
        let d = disc_input(r, mr);
        let (rho2, edge): (f64, f64) = (d.rho2.into(), (pole * pole * d.i2).into());
        if (rho2 - edge).abs() > 1e-3 * edge {
            assert_eq!(
                got,
                in_disc(d, pole),
                "the disc test differs from the float form away from the edge at {base:?}, masses {m:?}"
            );
        }
        let mirror = r.map(|[x, y]| [x, -y]);
        assert_eq!(
            test(mirror, mr, pole),
            got,
            "the mirror changes the disc test at {base:?}"
        );
        for e in [-100i32, -70, -40, -1, 1, 40, 70, 100] {
            let s: R = real(2f64.powi(e));
            if !Float::is_normal(s) {
                continue;
            }
            let scaled = r.map(|v| v.map(|x| x * s));
            assert_eq!(
                test(scaled, mr, pole),
                got,
                "the disc test at {base:?} changes under the scale 2^{e}"
            );
        }
        inside += <usize as From<_>>::from(got);
    }
    assert!(
        inside > 100 && inside < 3900,
        "the configurations do not straddle the disc: {inside} inside"
    );
    inside
}

#[test]
fn theta_hold_disc_test() {
    let i64s = check_disc::<f64>(config_in_disc::<f64>);
    let i32s = check_disc::<f32>(config_in_disc::<f32>);
    eprintln!("theta_hold_disc_test: 4000 configurations, {i64s} inside at f64, {i32s} at f32");
}

negative_control!(
    theta_hold_disc_test,
    "the float form at f32, whose intermediates underflow at 2^-40, is not invariant under the scale",
    expected = "changes under the scale",
    {
        check_disc::<f32>(|r, m, pole| in_disc(disc_input(r, m), pole));
    }
);

// ── theta_resume_split ──────────────────────────────────────────────────────────────────────────────────────────────

/// A split march under test: `ic`'s march to `horizon`, dispatched as the calls ending at `splits` (ascending, each
/// in `1 … horizon − 1`) then one to the horizon; no splits is one call.
type Split<R> = fn(&Ic<R>, u32, &[u32]) -> SimStateBaseOf<R>;

fn kernel_split<R: Prec>(ic: &Ic<R>, horizon: u32, splits: &[u32]) -> SimStateBaseOf<R> {
    let mut s = integrate::<Kdk, R>(ic, &uniforms(PASSAGE_DT, 0));
    let mut at = 0;
    for &end in splits.iter().chain([horizon].iter()) {
        let mut cadence = WrapperCadence::new(&ic.m, &s.r, &s.p);
        s = integrate_resumed_with::<Kdk, R, _>(
            ic,
            &uniforms(PASSAGE_DT, horizon),
            &mut cadence,
            &s,
            end - at,
        );
        assert_eq!(
            tm_t_end_step(s.times),
            end,
            "a resumed call did not end where asked"
        );
        at = end;
    }
    s
}

/// The `k − 1` split points for `k` calls over `horizon`, one of them `focus`, the rest spread evenly.
fn splits(k: u32, focus: u32, horizon: u32) -> Vec<u32> {
    let mut out = vec![focus];
    let spread = (1..k).map(|j| j * horizon / k);
    let near = (1..horizon).flat_map(|d| [focus + d, focus.saturating_sub(d)]);
    for c in spread.chain(near) {
        if out.len() == (k - 1) as usize {
            break;
        }
        if c >= 1 && c < horizon && !out.contains(&c) {
            out.push(c);
        }
    }
    out.sort_unstable();
    out
}

/// Checks that `split`'s march at `R` is bit-identical to one call for `k ∈ {2, 3, 7}` calls, split on the passage's
/// entry step, inside it, on its exit step, and inside the disc for the IC that starts there. Returns the runs made.
fn check_split<R: Prec>(split: Split<R>) -> usize {
    let ic = passage::<R>();
    let series = prefixes(&ic, PASSAGE_DT, PASSAGE_STEPS);
    let (entry, exit) = passage_steps(&series, ic.m);
    let (entry, exit) = (entry as u32, exit as u32);
    let pivot = pivot::<R>();
    let cases: [(&str, &Ic<R>, u32); 5] = [
        ("the passage's entry step", &ic, entry),
        ("inside the passage", &ic, (entry + exit) / 2),
        ("the passage's last step inside", &ic, exit - 1),
        ("the passage's exit step", &ic, exit),
        ("inside the disc for the IC inside", &pivot, 3),
    ];
    let mut runs = 0;
    for (what, ic, focus) in cases {
        let one = split(ic, PASSAGE_STEPS, &[]);
        for k in [2, 3, 7] {
            let at = splits(k, focus, PASSAGE_STEPS);
            let many = split(ic, PASSAGE_STEPS, &at);
            assert_eq!(
                (bits(many.theta), many._reserved),
                (bits(one.theta), one._reserved),
                "split {k} ways at {at:?}, on {what}: θ̃ and _reserved differ from one call's"
            );
            assert_eq!(
                (
                    bits(many.closure_min),
                    many.closure_step,
                    sd_departed(many.packed_a)
                ),
                (
                    bits(one.closure_min),
                    one.closure_step,
                    sd_departed(one.packed_a)
                ),
                "split {k} ways at {at:?}, on {what}: the closure registers differ from one call's"
            );
            assert_eq!(
                (
                    many.r.map(|v| v.map(bits)),
                    many.p.map(|v| v.map(bits)),
                    sd_state(many.packed_a)
                ),
                (
                    one.r.map(|v| v.map(bits)),
                    one.p.map(|v| v.map(bits)),
                    sd_state(one.packed_a)
                ),
                "split {k} ways at {at:?}, on {what}: the state differs from one call's"
            );
            runs += 1;
        }
    }
    runs
}

/// The resume's step bounds: `steps` short of the horizon is `running` at `t_end_step + steps`; exactly to it, or past
/// it, is `bounded` at the horizon; from the horizon nothing marches.
fn check_resume_bounds<R: Prec>() {
    let ic = passage::<R>();
    let start = integrate::<Kdk, R>(&ic, &uniforms(PASSAGE_DT, 0));
    let resume = |s: &SimStateBaseOf<R>, steps: u32| {
        let mut cadence = WrapperCadence::new(&ic.m, &s.r, &s.p);
        integrate_resumed_with::<Kdk, R, _>(&ic, &uniforms(PASSAGE_DT, 20), &mut cadence, s, steps)
    };
    let short = resume(&start, 19);
    assert_eq!(
        (tm_t_end_step(short.times), sd_state(short.packed_a)),
        (19, STATE_RUNNING),
        "one short"
    );
    for steps in [20, 21, u32::MAX] {
        let full = resume(&start, steps);
        assert_eq!(
            (tm_t_end_step(full.times), sd_state(full.packed_a)),
            (20, STATE_BOUNDED),
            "{steps} steps from 0 of a 20-step horizon"
        );
    }
    let again = resume(&resume(&start, 20), 5);
    assert_eq!(
        tm_t_end_step(again.times),
        20,
        "a march resumed at its horizon marched"
    );
    let fresh = integrate::<Kdk, R>(&ic, &uniforms(PASSAGE_DT, 20));
    assert_eq!(
        bits(again.theta),
        bits(fresh.theta),
        "a march resumed at its horizon moved θ̃"
    );
    // From a state part-way in, at step 5: the steps left are the horizon less the steps done, 15, so 15 or more is
    // `bounded` at the horizon, never past it, and fewer is `running` short of it.
    let mid = resume(&start, 5);
    for (steps, end, state) in [
        (14, 19, STATE_RUNNING),
        (15, 20, STATE_BOUNDED),
        (16, 20, STATE_BOUNDED),
        (19, 20, STATE_BOUNDED),
        (25, 20, STATE_BOUNDED),
    ] {
        let s = resume(&mid, steps);
        assert_eq!(
            (tm_t_end_step(s.times), sd_state(s.packed_a)),
            (end, state),
            "{steps} steps from 5 of a 20-step horizon"
        );
    }
}

/// A sim key whose `N_max` is 2 with every threshold 0, so each macro-step takes one substep and none saturates.
fn unsaturated<R: Prec>(dt_macro: f64, horizon_steps: u32) -> WrapperUniforms<R> {
    WrapperUniforms {
        n_max: 2,
        ..uniforms(dt_macro, horizon_steps)
    }
}

/// The resume keeps the stored `saturated` bit, sticky (payload §2): a stored state whose bit is set stays set through
/// a resumed march whose own macro-steps never saturate, and one whose bit is clear stays clear.
fn check_resume_saturated<R: Prec>() {
    let ic = pivot::<R>();
    let start = integrate::<Kdk, R>(&ic, &unsaturated(PASSAGE_DT, 5));
    assert!(
        !sd_saturated(start.packed_a),
        "a march at N_max 2 with every threshold 0 saturated"
    );
    let resume = |s: &SimStateBaseOf<R>| {
        let mut cadence = WrapperCadence::new(&ic.m, &s.r, &s.p);
        integrate_resumed_with::<Kdk, R, _>(&ic, &unsaturated(PASSAGE_DT, 10), &mut cadence, s, 5)
    };
    assert!(
        !sd_saturated(resume(&start).packed_a),
        "a resumed march that never saturates set the bit"
    );
    let marked = SimStateBaseOf {
        packed_a: set_saturated(start.packed_a, true),
        ..start
    };
    let s = resume(&marked);
    assert_eq!(tm_t_end_step(s.times), 10, "the resumed march's end");
    assert!(
        sd_saturated(s.packed_a),
        "a resumed march dropped the stored saturated bit"
    );
}

// ── march_loop_bound ────────────────────────────────────────────────────────────────────────────────────────────────

/// A cadence that is never done and counts the macro-steps the march begins, by the bucket's `d²`, which the march
/// reads once at the start of each, and panics at the first past `limit`: a march whose loop would run past its end
/// fails at once rather than running on.
struct Bounded {
    begun: Cell<u32>,
    limit: u32,
}

impl Bounded {
    fn new(limit: u32) -> Self {
        Bounded {
            begun: Cell::new(0),
            limit,
        }
    }
}

impl<R: Prec> Cadence<R> for Bounded {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {}

    fn done(&self) -> bool {
        false
    }

    fn bucket_d2(&self, r: &[[R; 2]; 3]) -> f32
    where
        R: Quantise,
    {
        let n = self.begun.get() + 1;
        self.begun.set(n);
        assert!(
            n <= self.limit,
            "the march began macro-step {n}, past its end after {} (the loop bound)",
            self.limit
        );
        d2_quantised(r)
    }
}

/// The march's loop stops at its end with a cadence that is never done, from a fresh start (horizons 0 and 4) and on a
/// resume (3 more of a 20-step horizon, and 0 more), `slack` macro-steps fewer allowed than each should take. Every
/// march here is under a [`Bounded`] cadence, so a broken loop condition fails fast.
fn check_loop_bound<R: Prec>(slack: u32) {
    let ic = pivot::<R>();
    let fresh = |horizon: u32| {
        let mut c = Bounded::new(horizon.saturating_sub(slack));
        let s = integrate_with::<Kdk, R, _>(&ic, &uniforms(PASSAGE_DT, horizon), &mut c);
        assert_eq!(
            c.begun.get(),
            horizon,
            "macro-steps begun to a horizon of {horizon}"
        );
        s
    };
    assert_eq!(tm_t_end_step(fresh(0).times), 0, "a horizon of 0");
    let four = fresh(4);
    assert_eq!(
        (tm_t_end_step(four.times), sd_state(four.packed_a)),
        (4, STATE_BOUNDED),
        "a horizon of 4"
    );
    let resume = |from: &SimStateBaseOf<R>, steps: u32| {
        let mut c = Bounded::new(steps.saturating_sub(slack));
        let s = integrate_resumed_with::<Kdk, R, _>(
            &ic,
            &uniforms(PASSAGE_DT, 20),
            &mut c,
            from,
            steps,
        );
        assert_eq!(
            c.begun.get(),
            steps,
            "macro-steps begun on a resume of {steps}"
        );
        s
    };
    let at4 = resume(&fresh(0), 4);
    for steps in [3, 0] {
        let s = resume(&at4, steps);
        assert_eq!(
            (tm_t_end_step(s.times), sd_state(s.packed_a)),
            (4 + steps, STATE_RUNNING),
            "{steps} more of a 20-step horizon from 4"
        );
    }
}

#[test]
fn march_loop_bound() {
    check_loop_bound::<f64>(0);
    check_loop_bound::<f32>(0);
}

negative_control!(
    march_loop_bound,
    "a bound one macro-step short of the march's own must trip, so the bound detects a march that runs past its end",
    expected = "past its end",
    check_loop_bound::<f64>(1)
);

#[test]
fn theta_resume_split() {
    let r64 = check_split::<f64>(kernel_split::<f64>);
    let r32 = check_split::<f32>(kernel_split::<f32>);
    check_resume_bounds::<f64>();
    check_resume_bounds::<f32>();
    check_resume_saturated::<f64>();
    check_resume_saturated::<f32>();
    eprintln!("theta_resume_split: {r64} split runs at f64 and {r32} at f32, each bit-identical to one call");
}

/// A march that holds the reference unquantised within a dispatch, quantising it only to store it between calls:
/// re-stepped over the kernel's states, `θ̃` and `_reserved` replaced.
#[cfg(feature = "controls")]
fn unquantised_split(ic: &Ic<f64>, horizon: u32, splits: &[u32]) -> SimStateBaseOf<f64> {
    let series = prefixes(ic, PASSAGE_DT, horizon);
    let mut theta = kernel::shape::Theta::start();
    for k in 1..=horizon as usize {
        if splits.contains(&(k as u32 - 1)) {
            let code = if theta.has_frozen && config_in_disc(series[k - 1].r, ic.m, r_pole()) {
                encode(theta.frozen)
            } else {
                NO_REFERENCE
            };
            theta = kernel::shape::Theta {
                theta: theta.theta,
                frozen: decode(code),
                has_frozen: code != NO_REFERENCE,
            };
        }
        let (a, b) = (shape(series[k - 1].r, ic.m), shape(series[k].r, ic.m));
        theta = kernel::shape::theta_step_held(
            theta,
            a,
            b,
            config_in_disc(series[k - 1].r, ic.m, r_pole()),
            config_in_disc(series[k].r, ic.m, r_pole()),
        );
    }
    let last = series[horizon as usize];
    SimStateBaseOf {
        theta: theta.theta,
        _reserved: if config_in_disc(last.r, ic.m, r_pole()) && theta.has_frozen {
            encode(theta.frozen) as u16
        } else {
            NO_REFERENCE as u16
        },
        ..last
    }
}

negative_control!(
    theta_resume_split,
    "a march holding the reference unquantised within a dispatch differs at the exit from one split inside",
    expected = "θ̃ and _reserved differ from one call's",
    {
        check_split::<f64>(unquantised_split);
    }
);

// ── closure_registers and closure_departure ─────────────────────────────────────────────────────────────────────────

/// The figure-eight runs' macro-step.
const EIGHT_DT: f64 = 0.01;

/// The figure-eight (`validation::drift`), in the canonical frame, at `R`.
fn eight<R: Prec>() -> Ic<R> {
    let ic = figure_eight().ic;
    Ic {
        m: ic.m.map(real),
        r: ic.r.map(|v| v.map(real)),
        p: ic.p.map(|v| v.map(real)),
    }
}

/// The closure registers after each horizon: `(closure_min, closure_step, departed)`.
type Registers<R> = (R, u32, bool);

/// A closure under test: the registers after each horizon `0 … steps` of `ic`.
type ClosureRun<R> = fn(&Ic<R>, u32) -> Vec<Registers<R>>;

fn kernel_closure<R: Prec>(ic: &Ic<R>, steps: u32) -> Vec<Registers<R>> {
    prefixes(ic, EIGHT_DT, steps)
        .iter()
        .map(|s| {
            (
                s.closure_min,
                <u32 as From<_>>::from(s.closure_step),
                sd_departed(s.packed_a),
            )
        })
        .collect()
}

/// Checks `run`'s registers after every horizon of the figure-eight against the offline ones over the stored shape
/// path: departure the first step whose chord to `n̂(0)` exceeds `δ_dep`, `closure_min` the least chord from it on, and
/// `closure_step` the first step it is taken at; before departure, `+∞` and 0. Returns the departure step.
fn check_registers<R: Prec>(run: ClosureRun<R>, steps: u32) -> u32 {
    let ic = eight::<R>();
    let path = prefixes(&ic, EIGHT_DT, steps);
    let got = run(&ic, steps);
    let n_0 = shape(ic.r, ic.m);
    let delta = delta_dep::<R>();
    let d2: Vec<R> = path.iter().map(|s| chord2(shape(s.r, ic.m), n_0)).collect();
    let chords: Vec<f64> = d2.iter().map(|&x| w(Float::sqrt(x))).collect();
    let departure = (1..d2.len())
        .find(|&k| d2[k] > delta * delta)
        .expect("the figure-eight departs") as u32;
    for (k, &(min, step, departed)) in got.iter().enumerate() {
        let k = k as u32;
        let want = if k < departure {
            (f64::INFINITY, 0)
        } else {
            (departure..=k).fold((f64::INFINITY, 0), |(m, at), j| {
                let d = chords[j as usize];
                if d < m {
                    (d, j)
                } else {
                    (m, at)
                }
            })
        };
        assert_eq!(
            departed,
            k >= departure,
            "after {k} steps, departed is {departed}"
        );
        assert_eq!(
            (w(min).to_bits(), step),
            (want.0.to_bits(), want.1),
            "after {k} steps, closure_min {:e} at step {step} is not the offline minimum {:e} at step {}",
            w(min),
            want.0,
            want.1
        );
    }
    departure
}

#[test]
fn closure_registers() {
    let d64 = check_registers::<f64>(kernel_closure::<f64>, 300);
    let d32 = check_registers::<f32>(kernel_closure::<f32>, 300);
    eprintln!("closure_registers: 300 horizons each; departure at step {d64} (f64), {d32} (f32)");
}

/// The registers re-stepped over the kernel's states with no departure gate, as stored: the minimum from the first
/// step.
#[cfg(feature = "controls")]
fn ungated_closure(ic: &Ic<f64>, steps: u32) -> Vec<Registers<f64>> {
    let path = prefixes(ic, EIGHT_DT, steps);
    let n_0 = shape(ic.r, ic.m);
    let mut c = Closure::<f64>::fresh();
    let mut out = vec![(c.stored_min(), c.step, c.departed)];
    for (k, s) in path.iter().enumerate().skip(1) {
        c = c.sample(shape(s.r, ic.m), n_0, k as u32, 0.0);
        out.push((c.stored_min(), c.step, c.departed));
    }
    out
}

negative_control!(
    closure_registers,
    "registers with no departure gate take the first step's near-zero chord as the minimum",
    expected = "departed is true",
    {
        check_registers::<f64>(ungated_closure, 300);
    }
);

/// Checks the figure-eight's one-call registers at `R` from `run`: `closure_min` below `bound` at `closure_step`
/// within 1% of the shape period over `dt_macro`, the horizon three quarters of the orbit's period. The figure-eight
/// starts at the Euler configuration with body 2 midway between the others, which recurs, rotated, half a period
/// later: its shape curve closes at `P/2`. Returns `(closure_min, closure_step, the expected step)`.
fn check_departure<R: Prec>(
    run: fn(&Ic<R>, u32) -> SimStateBaseOf<R>,
    bound: f64,
) -> (f64, u32, f64) {
    let period = eight_period();
    let horizon = (0.75 * period / EIGHT_DT) as u32;
    let s = run(&eight::<R>(), horizon);
    let want = 0.5 * period / EIGHT_DT;
    let (min, step): (f64, u32) = (s.closure_min.into(), <u32 as From<_>>::from(s.closure_step));
    assert!(sd_departed(s.packed_a), "the figure-eight never departed");
    assert!(
        (<f64 as From<_>>::from(step) - want).abs() <= 0.01 * want,
        "closure_step {step} is not the shape period over dt_macro, {want:.1} steps"
    );
    assert!(
        min <= bound,
        "closure_min {min:e} at step {step} is not near 0 (bound {bound:e})"
    );
    (min, step, want)
}

fn kernel_eight<R: Prec>(ic: &Ic<R>, horizon: u32) -> SimStateBaseOf<R> {
    integrate::<Kdk, R>(ic, &uniforms(EIGHT_DT, horizon))
}

/// `closure_min`'s bound: one step's arc on the shape sphere, the curve closing at `P/2` after one turn of `2π` of
/// longitude round the equator, so about `2π·dt/(P/2)`, the most the nearest sample can miss by, and as much again for
/// the integrator's phase error.
fn departure_bound() -> f64 {
    2.0 * TAU * EIGHT_DT / (0.5 * eight_period())
}

/// A closure before departure is ignored, however near: samples at the IC's shape and just inside `δ_dep` leave the
/// registers unset; the first past it departs and is the minimum; the edges, a chord of exactly `δ` and the next float
/// past it.
fn check_gate(sample: Sample64) {
    let n_0 = [0.0, 0.0, 1.0];
    let c = Closure::<f64>::fresh();
    let c = sample(sample(c, n_0, n_0, 1, 0.5), [0.0, 0.5, 1.0], n_0, 2, 0.5);
    assert_eq!(
        c,
        Closure::fresh(),
        "a sample within δ_dep, its chord exactly δ, departed or set the minimum"
    );
    let past = [0.0, f64::from_bits(0.5f64.to_bits() + 1), 1.0];
    let c = sample(c, past, n_0, 3, 0.5);
    assert!(
        c.departed && c.step == 3,
        "the first chord past δ_dep is not the departure and the minimum: {c:?}"
    );
    let c = sample(c, [0.0, 0.4, 1.0], n_0, 4, 0.5);
    assert_eq!(
        c.step, 4,
        "a nearer chord after departure is not the minimum"
    );
    let c = sample(
        sample(c, [0.0, 0.4, 1.0], n_0, 5, 0.5),
        [0.0, 0.6, 1.0],
        n_0,
        6,
        0.5,
    );
    assert_eq!(
        (c.step, c.departed),
        (4, true),
        "an equal or farther chord moved the minimum, or departure cleared"
    );
}

/// A closure sample under check: the kernel's ([`Closure::sample`]), or a control's.
type Sample64 = fn(Closure<f64>, [f64; 3], [f64; 3], u32, f64) -> Closure<f64>;

#[test]
fn closure_departure() {
    let bound = departure_bound();
    let (m64, s64, want) = check_departure::<f64>(kernel_eight::<f64>, bound);
    let (m32, s32, _) = check_departure::<f32>(kernel_eight::<f32>, bound);
    check_gate(Closure::sample);
    eprintln!(
        "closure_departure: the figure-eight's shape period {want:.1} steps; closure_min {m64:.3e} at step {s64} \
         (f64), {m32:.3e} at step {s32} (f32); bound {bound:.3e}; δ_dep {:e}",
        delta_dep::<f64>()
    );
}

negative_control!(
    closure_departure,
    "a closure with no departure gate takes a sample before departure as its minimum",
    expected = "departed or set the minimum",
    {
        check_gate(|c, n, n_0, step, _| c.sample(n, n_0, step, 0.0));
    }
);

/// A non-finite chord (the shape point broke) is no sample: before departure it neither departs nor sets the
/// minimum, and the stored minimum stays `+∞`; after departure it moves neither the minimum nor its step.
fn check_non_finite(sample: Sample64) {
    let n_0 = [0.0, 0.0, 1.0];
    let far = sample(Closure::fresh(), [0.0, f64::INFINITY, 1.0], n_0, 1, 0.5);
    assert!(
        far == Closure::fresh() && far.stored_min() == f64::INFINITY,
        "a non-finite chord departed or set the minimum: {far:?}"
    );
    let c = sample(far, [0.0, 0.75, 1.0], n_0, 2, 0.5);
    let c = sample(c, [0.0, f64::NAN, 1.0], n_0, 3, 0.5);
    assert_eq!(
        (c.step, c.departed, c.stored_min()),
        (2, true, 0.75),
        "a non-finite chord after departure moved the minimum"
    );
}

#[test]
fn closure_non_finite_chord() {
    check_non_finite(Closure::sample);
}

negative_control!(
    closure_non_finite_chord,
    "a closure that takes a non-finite chord as a departure and its minimum",
    expected = "a non-finite chord departed or set the minimum",
    {
        check_non_finite(|c, n, n_0, step, dd| {
            if n.iter().all(|x| x.is_finite()) {
                c.sample(n, n_0, step, dd)
            } else {
                Closure {
                    min: f64::INFINITY,
                    step,
                    departed: true,
                }
            }
        });
    }
);

// ── theta_unwrap_real_orbit ─────────────────────────────────────────────────────────────────────────────────────────

/// The rotation added to the figure-eight, `ω`: each body's momentum gains `mᵢ·ω ẑ × rᵢ`, so `L_z` becomes `ω·I = ω`
/// in the canonical frame (`I = 1`), the figure-eight's own `L_z` being 0 to its published digits. At `ω = 0.02` the
/// orbit stays bound and circulating over two periods (0.05 breaks it up within half a period, measured).
const OMEGA: f64 = 0.02;

/// The fixture's `|L_z|` exceeds this, so "`L_z` opposite" is not vacuous (R-580's fix round).
const LZ_BOUND: f64 = 0.01;

/// The real orbit's horizon, in periods: the shape curve turns twice a period, so `orbit_count` reaches 2.
const ORBIT_PERIODS: f64 = 1.3;

/// The figure-eight with the rotation [`OMEGA`] added, at `R`, or its mirror (`y → −y` on every position and
/// momentum), or a positions-only mirror.
fn rotating<R: Prec>(mirror: Mirror) -> Ic<R> {
    let ic = figure_eight().ic;
    let p = [0, 1, 2].map(|i| {
        let [x, y] = ic.r[i];
        [
            ic.p[i][0] - ic.m[i] * OMEGA * y,
            ic.p[i][1] + ic.m[i] * OMEGA * x,
        ]
    });
    let ic = Ic { p, ..ic };
    let flip = |v: [f64; 2]| [v[0], -v[1]];
    let ic = match mirror {
        Mirror::None => ic,
        Mirror::Full => Ic {
            r: ic.r.map(flip),
            p: ic.p.map(flip),
            ..ic
        },
        Mirror::PositionsOnly => Ic {
            r: ic.r.map(flip),
            ..ic
        },
    };
    Ic {
        m: ic.m.map(real),
        r: ic.r.map(|v| v.map(real)),
        p: ic.p.map(|v| v.map(real)),
    }
}

#[derive(Clone, Copy, Debug)]
enum Mirror {
    None,
    Full,
    #[cfg_attr(not(feature = "controls"), allow(dead_code))] // the positions-only control's mirror
    PositionsOnly,
}

/// `L_z = Σ (xᵢ p_yᵢ − yᵢ p_xᵢ)` of a state, in a fixed order: each term's sign flips exactly under the mirror.
fn lz<R: Prec>(r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> R {
    let term = |i: usize| r[i][0] * p[i][1] - r[i][1] * p[i][0];
    term(0) + term(1) + term(2)
}

/// One sampled step of a run: `θ̃` as stored, the shape point and `L_z`.
#[derive(Clone, Copy, Debug)]
struct Sample<R> {
    theta: R,
    n: [R; 3],
    lz: R,
}

/// The run of `ic` sampled after every step `0 … steps`, each step a resumed call of one step.
fn sampled<R: Prec>(ic: &Ic<R>, dt: f64, steps: u32) -> Vec<Sample<R>> {
    let mut s = integrate::<Kdk, R>(ic, &uniforms(dt, 0));
    let mut out = Vec::with_capacity(steps as usize + 1);
    for _ in 0..=steps {
        out.push(Sample {
            theta: s.theta,
            n: shape(s.r, ic.m),
            lz: lz(&s.r, &s.p),
        });
        let mut cadence = WrapperCadence::new(&ic.m, &s.r, &s.p);
        s = integrate_resumed_with::<Kdk, R, _>(ic, &uniforms(dt, steps), &mut cadence, &s, 1);
    }
    out
}

/// The hand count along a path of shape points, with no `atan2`: the net signed crossings of the half-meridian
/// through the equatorial direction `b` (the half on `b`'s side, `n·b > 0`), `+1` where `n·(ẑ × b)` goes from negative
/// to non-negative (travel toward increasing longitude), `−1` the other way. Steps from `from` on.
fn crossings(path: &[[f64; 3]], b: [f64; 3], from: usize) -> i64 {
    let side = |n: [f64; 3]| -b[1] * n[0] + b[0] * n[1];
    let ahead = |n: [f64; 3]| b[0] * n[0] + b[1] * n[1] > 0.0;
    let mut net = 0;
    for k in from.max(1)..path.len() {
        let (a, n) = (path[k - 1], path[k]);
        if !(ahead(a) && ahead(n)) {
            continue;
        }
        let (sa, sn) = (side(a), side(n));
        if sa < 0.0 && sn >= 0.0 {
            net += 1;
        } else if sa >= 0.0 && sn < 0.0 {
            net -= 1;
        }
    }
    net
}

/// Derives `retrograde` from a sample: the read side's, from `θ̃`, or a control's.
type Retro<R> = fn(&Sample<R>) -> bool;

fn retro_theta<R: Prec>(s: &Sample<R>) -> bool {
    let t: f64 = s.theta.into();
    retrograde(t as f32)
}

/// Checks dd_integrator §5 test 8 on the run `x` and its mirror's run `y`, of masses `m`, `retro` the `retrograde`
/// derivation, in this order: the mirror arm at every step (`L_z` bitwise opposite, `θ̃` bitwise identical, `retrograde`
/// identical); no 2π jumps; `orbit_count` against the hand count on the half-meridian through `n(0)`; `retrograde`
/// against the hand-counted direction from the three binary-collision half-meridians. Returns `(orbit_count, the
/// signed hand count over the collision half-meridians)`.
fn check_orbit<R: Prec>(
    x: &[Sample<R>],
    y: &[Sample<R>],
    m: [R; 3],
    retro: Retro<R>,
) -> (u32, i64) {
    assert!(
        w(x[0].lz).abs() > LZ_BOUND,
        "the fixture's |L_z| {:e} is within the stated bound {LZ_BOUND}",
        w(x[0].lz)
    );
    for (k, (a, b)) in x.iter().zip(y).enumerate() {
        assert_eq!(
            bits(b.lz),
            bits(-a.lz),
            "step {k}: the mirror's L_z is not the run's, negated"
        );
        assert_eq!(
            bits(b.theta),
            bits(a.theta),
            "step {k}: the mirror's θ̃ is not the run's"
        );
        assert_eq!(
            retro(b),
            retro(a),
            "step {k}: retrograde differs across the mirror pair"
        );
    }
    for k in 1..x.len() {
        let jump = (w(x[k].theta) - w(x[k - 1].theta)).abs();
        assert!(jump < PI / 4.0, "step {k}: θ̃ jumps by {jump}");
    }
    let path: Vec<[f64; 3]> = x.iter().map(|s| s.n.map(w)).collect();
    let n0 = path[0];
    let rho0 = n0[0].hypot(n0[1]);
    let start = [n0[0] / rho0, n0[1] / rho0, 0.0];
    let turns = crossings(&path, start, 2);
    let last = x[x.len() - 1];
    let count = orbit_count(w(last.theta) as f32);
    assert_eq!(
        <i64 as From<_>>::from(count),
        turns.abs(),
        "orbit_count {count} is not the hand count {turns} of turns past n(0)'s half-meridian"
    );
    let bc: [[f64; 3]; 3] = landmarks(m).bc.map(|b| b.map(w));
    let signed: i64 = bc.iter().map(|&b| crossings(&path, b, 1)).sum();
    assert!(
        signed.abs() >= 3,
        "the orbit does not circulate: {signed} net crossings, by meridian {:?}; θ̃ at the end {:e}, orbit_count {count}",
        bc.map(|b| crossings(&path, b, 1)),
        w(last.theta)
    );
    assert_eq!(
        retro(&last),
        signed < 0,
        "retrograde {} does not match the hand-counted direction of travel ({signed} net crossings of the \
         binary-collision half-meridians)",
        retro(&last)
    );
    (count, signed)
}

/// The arcs R-433 asks for: the largest step's chord on the sphere among the steps within `3·r_pole` of the pole, and
/// the least polar radius reached; `None` for the arc where no step comes that near.
fn near_pole_arcs<R: Prec>(path: &[[R; 3]]) -> (Option<f64>, f64) {
    let pole: f64 = r_pole::<f64>();
    let mut worst: Option<f64> = None;
    let mut least = f64::INFINITY;
    for k in 1..path.len() {
        let (a, b) = (path[k - 1].map(w), path[k].map(w));
        let rho = |n: [f64; 3]| n[0].hypot(n[1]);
        least = least.min(rho(b));
        if rho(a).min(rho(b)) < 3.0 * pole {
            let arc =
                ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
            worst = Some(worst.map_or(arc, |w: f64| w.max(arc)));
        }
    }
    (worst, least)
}

/// The figure-eight run and its mirror at `R`, `ORBIT_PERIODS` periods; the second run's mirror as given.
fn orbit_pair<R: Prec>(mirror: Mirror) -> (Vec<Sample<R>>, Vec<Sample<R>>, [R; 3]) {
    let steps = (ORBIT_PERIODS * eight_period() / EIGHT_DT) as u32;
    let x = rotating::<R>(Mirror::None);
    let y = rotating::<R>(mirror);
    (
        sampled(&x, EIGHT_DT, steps),
        sampled(&y, EIGHT_DT, steps),
        x.m,
    )
}

#[test]
fn theta_unwrap_real_orbit() {
    let (x, y, m) = orbit_pair::<f64>(Mirror::Full);
    let (c64, h64) = check_orbit(&x, &y, m, retro_theta::<f64>);
    let (x32, y32, m32) = orbit_pair::<f32>(Mirror::Full);
    let (c32, h32) = check_orbit(&x32, &y32, m32, retro_theta::<f32>);
    let (arc8, rho8) = near_pole_arcs(&x.iter().map(|s| s.n).collect::<Vec<_>>());
    let pass = passage::<f64>();
    let pass_path: Vec<[f64; 3]> = prefixes(&pass, PASSAGE_DT, PASSAGE_STEPS)
        .iter()
        .map(|s| shape(s.r, pass.m))
        .collect();
    let (arcp, rhop) = near_pole_arcs(&pass_path);
    let pole = r_pole::<f64>();
    eprintln!(
        "theta_unwrap_real_orbit: L_z {:.3e} (bound {LZ_BOUND}); orbit_count {c64} (f64), {c32} (f32); signed hand \
         count over the collision half-meridians {h64} (f64), {h32} (f32); θ̃ end {:.6} (f64)",
        x[0].lz,
        x[x.len() - 1].theta
    );
    eprintln!(
        "R-433 near-pole arcs (steps within 3·r_pole of the pole), against π·r_pole = {:.4e} and (π/2)·r_pole = \
         {:.4e}: the rotating figure-eight, least polar radius {rho8:.4e}, largest arc {arc8:?}; the pole passage \
         (dt {PASSAGE_DT}), least polar radius {rhop:.4e}, largest arc {arcp:?}",
        PI * pole,
        PI / 2.0 * pole
    );
}

/// `θ̃` re-stepped over a run's shape points, each point transformed by `f` first: the R-389 step on other
/// coordinates.
#[cfg(feature = "controls")]
fn restepped(run: &[Sample<f64>], f: fn([f64; 3]) -> [f64; 3]) -> Vec<Sample<f64>> {
    let pole = r_pole::<f64>();
    let mut theta = kernel::shape::Theta::start();
    let mut out = vec![Sample {
        theta: 0.0,
        ..run[0]
    }];
    for k in 1..run.len() {
        theta = kernel::shape::theta_step(theta, f(run[k - 1].n), f(run[k].n), pole);
        out.push(Sample {
            theta: theta.theta,
            ..run[k]
        });
    }
    out
}

negative_control!(
    theta_unwrap_real_orbit,
    "θ̃'s sense reversed (u and v swapped) passes orbit_count and the mirror arm and fails the direction check",
    expected = "does not match the hand-counted direction",
    {
        let (x, y, m) = orbit_pair::<f64>(Mirror::Full);
        let swap = |n: [f64; 3]| [n[1], n[0], n[2]];
        check_orbit(&restepped(&x, swap), &restepped(&y, swap), m, retro_theta::<f64>);
    }
);

negative_control!(
    theta_unwrap_real_orbit_lz_retrograde,
    "retrograde computed from sign(L_z) differs across the mirror pair",
    expected = "retrograde differs across the mirror pair",
    {
        let (x, y, m) = orbit_pair::<f64>(Mirror::Full);
        check_orbit(&x, &y, m, |s: &Sample<f64>| s.lz < 0.0);
    }
);

negative_control!(
    theta_unwrap_real_orbit_theta_from_w,
    "θ̃ read from w, the longitude atan2(w, u), is not identical across the mirror pair",
    expected = "the mirror's θ̃ is not the run's",
    {
        let (x, y, m) = orbit_pair::<f64>(Mirror::Full);
        let from_w = |n: [f64; 3]| [n[0], n[2], n[1]];
        check_orbit(
            &restepped(&x, from_w),
            &restepped(&y, from_w),
            m,
            retro_theta::<f64>,
        );
    }
);

negative_control!(
    theta_unwrap_real_orbit_positions_only,
    "a positions-only mirror, the momenta's y-components nonzero, does not negate L_z",
    expected = "the mirror's L_z is not the run's, negated",
    {
        let ic = rotating::<f64>(Mirror::None);
        assert!(
            ic.p.iter().any(|v| v[1] != 0.0),
            "the fixture's momenta have no y-component"
        );
        let (x, y, m) = orbit_pair::<f64>(Mirror::PositionsOnly);
        check_orbit(&x, &y, m, retro_theta::<f64>);
    }
);

/// `Readout` is the march's own: a fresh one at the IC is the stored zero state.
#[test]
fn theta_hold_readout_fresh() {
    let ic = passage::<f64>();
    let r = Readout::fresh(ic.r, ic.m);
    let s = integrate::<Kdk, f64>(&ic, &uniforms(PASSAGE_DT, 0));
    let stored = r.store(SimStateBaseOf::default());
    assert_eq!(
        (
            stored.theta.to_bits(),
            stored._reserved,
            stored.closure_min,
            stored.closure_step
        ),
        (
            s.theta.to_bits(),
            s._reserved,
            s.closure_min,
            s.closure_step
        ),
        "a fresh readout's store is not the zero-step march's"
    );
    assert_eq!(s._reserved, 0xFFFF, "a fresh sample's _reserved");
    assert!(
        s.closure_min.is_infinite() && !sd_departed(s.packed_a),
        "a fresh sample's closure"
    );
}

negative_control!(
    theta_hold_readout_fresh,
    "a readout stored with a code is not the fresh march's",
    expected = "a fresh readout's store is not the zero-step march's",
    {
        let ic = passage::<f64>();
        let mut r = Readout::fresh(ic.r, ic.m);
        r.code = 0;
        let s = integrate::<Kdk, f64>(&ic, &uniforms(PASSAGE_DT, 0));
        let stored = r.store(SimStateBaseOf::default());
        assert_eq!(
            (
                stored.theta.to_bits(),
                stored._reserved,
                stored.closure_min,
                stored.closure_step
            ),
            (
                s.theta.to_bits(),
                s._reserved,
                s.closure_min,
                s.closure_step
            ),
            "a fresh readout's store is not the zero-step march's"
        );
    }
);
