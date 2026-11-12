//! The monotone latches and their f16 packing (`kernel::driver::latch`; payload §1, §2; render_contract Part 5,
//! "Unpack layer"; R-271, R-933):
//! - `f16_pack_clamp_*`: values beyond ±65504 pack to the finite 65504, never `+∞`; a failed state (`sim_failed`,
//!   `decode_failed` and the reserved codes) packs 0.0 in `dE_max` and `dLz_max` and the unset bits `0x7C00` in `d_min`,
//!   whatever its latches held; a valid state's `d_min` is unset only before its first observation, an observed `+∞`
//!   included, and a zero separation packs as the least subnormal, never 0.0 (REQ-PAY-041).
//! - `latch_across_persist`: a sample resumed across many persist boundaries, its latches reloaded from the payload at
//!   each, stores the f16 of the one f32 latch over the whole run, bit for bit, on synthetic sequences and on a
//!   march's own measurements (REQ-PAY-044).
//! - `f16_clamp_fuzz`: on inputs that include ±10⁶, ±∞ and NaN, a valid state's halves are finite and the clamp of the
//!   live latch; a failed state's are exactly 0.0 and `0x7C00`, tested by the bits (REQ-PAY-045).
//! - `latch_live_f32`: the latches' accessors give the live f32 values, never their f16 packing, which is what the
//!   march's decisions read (REQ-PAY-044, REQ-INT-045).
//! - `latches_vs_offline`: the latches a march stores equal the f16 of the offline least separation and greatest
//!   drifts over its whole recorded trajectory, the IC included (REQ-PAY-050).
//! - `latch_t0_collision_d_min`: a start already collided is a collision at `t = 0` that stores its least separation
//!   as `d_min`, never the unset value (R-933).
//!
//! **Domain** (pitfalls §10): the packing of whatever values the latches hold, and the latches' agreement with an
//! offline reduction of the same measurements. The marches are short KDK runs from decoded latents and the
//! close-encounter fixture, at f32 and f64; what they show is the latching, not the march's accuracy.

use kernel::decode::{self, Ic, Latent};
use kernel::detect::collision::CollisionRadius;
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::latch::Latches;
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::substep::pair_d2_quantised;
use kernel::driver::wrapper::{
    integrate_detecting, integrate_with, monitored, WrapperReal, WrapperUniforms,
};
use kernel::generated::links::Defaults;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::{Geom, State};
use kernel::payload::{
    f32_to_f16_bits, pa_d_min, pb_dE_max, pb_dLz_max, sd_state, set_state, tm_t_end_step,
    DminCounters, SimStateBaseOf, F16_FINITE_MAX, F16_MIN_SUBNORMAL, STATE_BOUNDED,
    STATE_COLLISION, STATE_DECODE_FAILED, STATE_ESCAPE, STATE_RUNNING, STATE_SIM_FAILED,
};
use kernel::physics::energy::{angular_momentum, energy};
use proptest::prelude::*;
use validation::drift::{close_encounter, ic_at, sim_key, AsR};
use validation::{negative_control, prop};

/// A packer: the latches and the descriptor in, `(packed_a, packed_b)` out.
type Persist = fn(&Latches, u32) -> (u32, u32);

/// The latch writer under test, with a frame's counters of its own.
fn persist(l: &Latches, packed_a: u32) -> (u32, u32) {
    l.persist(packed_a, &DminCounters::new())
}

/// A descriptor in state `state`, its other used bits a fixed pattern (`detail` 2, `saturated`, `dmin_pair` 1,
/// `last_symbol` 2), so a packer that disturbs them is caught.
fn descriptor(state: u32) -> u32 {
    set_state(0x270, state)
}

/// The latches after observing a separation `d` (as `d²`) and the drifts `de`, `dlz`.
fn latches(d: f32, de: f32, dlz: f32) -> Latches {
    let mut l = Latches::unset();
    l.separation_squared(d * d);
    l.drift(de, dlz);
    l
}

/// `d_min`'s half of `packed_a`, its bits.
fn d_min_bits(a: u32) -> u16 {
    (a >> 16) as u16
}

/// Whether the f16 bits `h` are finite: exponent field not all ones.
fn finite16(h: u16) -> bool {
    h & 0x7c00 != 0x7c00
}

// ── f16_pack_clamp (REQ-PAY-041) ─────────────────────────────────────────────────────────────────────────────────

/// The states a decoder reads as untrusted: `sim_failed`, `decode_failed` and the reserved codes 6 and 7.
const FAILED: [u32; 4] = [STATE_SIM_FAILED, STATE_DECODE_FAILED, 6, 7];

/// The valid states: the resolved outcomes and `running`.
const VALID: [u32; 4] = [STATE_ESCAPE, STATE_BOUNDED, STATE_COLLISION, STATE_RUNNING];

/// Asserts `pack`'s clamp on each valid state: every value beyond ±65504 packs as 65504 (bits `0x7BFF`), finite, a
/// value inside the range as its own f16, a zero separation as the least subnormal and an unobserved `d_min` as
/// unset, with the descriptor's low 16 bits kept.
fn check_valid(pack: Persist) {
    let beyond = [65504.0, 65519.0, 65520.0, 1e5, 1e6, f32::MAX, f32::INFINITY];
    for &state in &VALID {
        for &x in &beyond {
            for sign in [1.0f32, -1.0] {
                let (a, b) = pack(&latches(x, sign * x, sign * x), descriptor(state));
                let halves = [d_min_bits(a), b as u16, (b >> 16) as u16];
                assert!(
                    halves.iter().all(|&h| finite16(h)),
                    "state {state}, value {x}, sign {sign}: a half of {halves:04x?} is not finite"
                );
                assert_eq!(
                    halves, [0x7bff; 3],
                    "state {state}, value {x}, sign {sign}: the halves are not clamped to 65504"
                );
                assert_eq!(
                    (pa_d_min(a), pb_dE_max(b), pb_dLz_max(b)),
                    (F16_FINITE_MAX, F16_FINITE_MAX, F16_FINITE_MAX),
                    "state {state}, value {x}: unpack does not read 65504"
                );
                assert_eq!(
                    a & 0xffff,
                    descriptor(state),
                    "state {state}: the descriptor is not kept"
                );
            }
        }
        let (a, b) = pack(&latches(1.0, -0.5, 2.0), descriptor(state));
        assert_eq!(
            (d_min_bits(a), b),
            (0x3c00, 0x4000_3800),
            "state {state}: values inside the range are not their own f16"
        );
        let (a, _) = pack(&latches(0.0, 0.0, 0.0), descriptor(state));
        assert_eq!(
            d_min_bits(a),
            0x0001,
            "state {state}: a zero separation is not the least subnormal"
        );
        let (a, b) = pack(&Latches::unset(), descriptor(state));
        assert_eq!(
            (d_min_bits(a), b),
            (0x7c00, 0),
            "state {state}: unobserved latches are not d_min unset and the drifts 0"
        );
    }
}

/// Asserts `pack`'s failed-state rule: each failed state packs 0.0 in both drift halves and `0x7C00` in `d_min`,
/// whatever its latches held, with the descriptor's low 16 bits kept.
fn check_failed(pack: Persist) {
    for &state in &FAILED {
        for l in [
            latches(1e6, 1e6, -1e6),
            latches(0.25, 0.5, 0.75),
            latches(0.0, f32::INFINITY, f32::NAN),
            Latches::unset(),
        ] {
            let (a, b) = pack(&l, descriptor(state));
            assert_eq!(
                d_min_bits(a),
                0x7c00,
                "state {state}, {l:?}: the d_min bits are not 0x7C00"
            );
            assert_eq!(
                b, 0,
                "state {state}, {l:?}: the drift halves are not exactly 0.0"
            );
            assert_eq!(
                a & 0xffff,
                descriptor(state),
                "state {state}: the descriptor is not kept"
            );
        }
    }
}

#[test]
fn f16_pack_clamp_valid() {
    check_valid(persist);
}

/// A packer with no clamp: each half whose live value is past ±65504 converted as it is, every other half as
/// [`persist`] packs it, so it differs from the writer in the clamp alone.
#[cfg(feature = "controls")]
fn unclamped(l: &Latches, a: u32) -> (u32, u32) {
    let (packed_a, packed_b) = persist(l, a);
    if kernel::payload::sd_is_failed(a) {
        return (packed_a, packed_b);
    }
    let raw = |x: f32, kept: u32| {
        if x.abs() > F16_FINITE_MAX {
            u32::from(f32_to_f16_bits(x))
        } else {
            kept
        }
    };
    let d = if l.d_min_set() {
        raw(l.d_min(), packed_a >> 16)
    } else {
        packed_a >> 16
    };
    (
        (packed_a & 0xffff) | (d << 16),
        raw(l.de_max(), packed_b & 0xffff) | (raw(l.dlz_max(), packed_b >> 16) << 16),
    )
}

negative_control!(
    f16_pack_clamp_valid,
    "an unclamped pack of a value past 65504 is not finite",
    expected = "is not finite",
    check_valid(unclamped)
);

#[test]
fn f16_pack_clamp_failed() {
    check_failed(persist);
}

negative_control!(
    f16_pack_clamp_failed,
    "a failed state packed as a valid one keeps its latches, not the failed-state values",
    expected = "the d_min bits are not 0x7C00",
    check_failed(|l, a| {
        let (packed, b) = persist(l, set_state(a, STATE_RUNNING));
        ((packed & 0xffff_0000) | (a & 0xffff), b)
    })
);

/// Asserts that an observed separation of `+∞` packs as the finite 65504 under `pack`, not as the unset value, which
/// is a valid state's only before its first observation (R-271).
fn check_observed_infinity(pack: Persist) {
    let (a, _) = pack(&latches(f32::INFINITY, 0.0, 0.0), descriptor(STATE_BOUNDED));
    assert_eq!(
        d_min_bits(a),
        0x7bff,
        "an observed separation of +inf does not pack as 65504"
    );
}

#[test]
fn f16_pack_clamp_observed_infinity() {
    check_observed_infinity(persist);
}

negative_control!(
    f16_pack_clamp_observed_infinity,
    "a packer that reads a latched +∞ as unset, with no observation flag, stores the unset value",
    expected = "does not pack as 65504",
    check_observed_infinity(|l, a| {
        let (packed, b) = persist(l, a);
        if l.d_min() == f32::INFINITY {
            ((packed & 0xffff) | (0x7c00 << 16), b)
        } else {
            (packed, b)
        }
    })
);

// ── latch_live_f32 (REQ-PAY-044, REQ-INT-045) ───────────────────────────────────────────────────────────────────

/// The live values a reader `get` gives of latches: `(d_min, dE_max, dLz_max)`.
type Live = fn(&Latches) -> (f32, f32, f32);

/// The latches' own accessors: the live f32 values.
fn live(l: &Latches) -> (f32, f32, f32) {
    (l.d_min(), l.de_max(), l.dlz_max())
}

/// Asserts that `get` gives the live f32 latches, not their f16 packing: values no f16 holds read back as themselves
/// (`d_min` as `√` of its latched `d²`); before any observation `d_min` reads `+∞`, unset, and the drifts 0; and the
/// default latches are the unset ones.
fn check_live(get: Live) {
    let (d, de, dlz) = (1.0001f32, -1.0001f32, 3.0001f32);
    let l = latches(d, de, dlz);
    assert_eq!(
        get(&l),
        ((d * d).sqrt(), de.abs(), dlz),
        "the latches read are not the live f32 values"
    );
    let u = Latches::unset();
    assert_eq!(
        (
            get(&u).0.to_bits(),
            get(&u).1,
            get(&u).2,
            u.d_min_set(),
            l.d_min_set()
        ),
        (0x7f80_0000, 0.0, 0.0, false, true),
        "unobserved latches do not read d_min unset (+inf) and the drifts 0"
    );
    assert_eq!(
        Latches::default(),
        u,
        "the default latches are not the unset ones"
    );
}

#[test]
fn latch_live_f32() {
    check_live(live);
}

negative_control!(
    latch_live_f32,
    "a reader of the unpacked f16 halves reads the packing, not the live f32",
    expected = "are not the live f32 values",
    check_live(|l| {
        let (a, b) = persist(l, descriptor(STATE_RUNNING));
        (pa_d_min(a), pb_dE_max(b), pb_dLz_max(b))
    })
);

// ── f16_clamp_fuzz (REQ-PAY-045) ─────────────────────────────────────────────────────────────────────────────────

/// An input value: often one of the edges (±10⁶, ±∞, NaN, ±65504 and its rounding edge, ±0, the subnormal floor),
/// otherwise any f32.
fn value() -> impl Strategy<Value = f32> {
    prop_oneof![
        proptest::sample::select(vec![
            1e6,
            -1e6,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            65504.0,
            -65504.0,
            65520.0,
            -65520.0,
            0.0,
            -0.0,
            F16_MIN_SUBNORMAL,
            1e-9,
            f32::MAX,
        ]),
        any::<f32>(),
    ]
}

/// The f16 bits of a valid state's clamped value `x`, NaN as 0.0: the oracle for a drift half.
fn drift_bits(x: f32) -> u16 {
    if x.is_nan() {
        0
    } else {
        f32_to_f16_bits(x.clamp(-F16_FINITE_MAX, F16_FINITE_MAX))
    }
}

/// The f16 bits of a valid state's `d_min` `d`, clamped, with values below the least subnormal floored to it: the
/// oracle for `d_min`'s half.
fn d_min_oracle(d: f32) -> u16 {
    if d < F16_MIN_SUBNORMAL {
        0x0001
    } else {
        f32_to_f16_bits(d.min(F16_FINITE_MAX))
    }
}

/// The offline latches of a sequence of observations `(x, de, dlz)`, each separation `x` taken as `d² = x·x`: the
/// least non-NaN `d²` (`None` if none), and each drift's greatest magnitude, a non-finite one kept from its first
/// appearance on.
fn offline(obs: &[(f32, f32, f32)]) -> (Option<f32>, f32, f32) {
    let d2 = obs
        .iter()
        .map(|o| o.0 * o.0)
        .filter(|d| !d.is_nan())
        .reduce(f32::min);
    let max = |f: fn(&(f32, f32, f32)) -> f32| {
        let mags = obs.iter().map(|o| f(o).abs());
        mags.fold(0.0f32, |a, b| {
            if !a.is_finite() {
                a
            } else if !b.is_finite() || b > a {
                b
            } else {
                a
            }
        })
    };
    (d2, max(|o| o.1), max(|o| o.2))
}

/// Asserts the clamp and failed-state rule of `pack` on the latches of `obs` in `state`: a failed state's halves are
/// exactly `0x0000` in both drifts and `0x7C00` in `d_min`; a valid state's are finite, the clamp of the offline
/// latch, `d_min` unset only where no separation was met.
fn check_fuzz(pack: Persist, state: u32, obs: &[(f32, f32, f32)]) -> Result<(), TestCaseError> {
    let mut l = Latches::unset();
    for &(x, de, dlz) in obs {
        l.separation_squared(x * x);
        l.drift(de, dlz);
    }
    let (a, b) = pack(&l, descriptor(state));
    let halves = [d_min_bits(a), b as u16, (b >> 16) as u16];
    if FAILED.contains(&state) {
        prop_assert_eq!(
            halves,
            [0x7c00, 0x0000, 0x0000],
            "state {}: the failed-state bits are not 0x7C00, 0x0000, 0x0000",
            state
        );
    } else {
        let (d2, de, dlz) = offline(obs);
        let d = d2.map_or(0x7c00, |d2| d_min_oracle(d2.sqrt()));
        prop_assert!(
            halves[1..].iter().all(|&h| finite16(h)) && (d2.is_none() || finite16(halves[0])),
            "state {}: a half of {:04x?} is not finite and clamped",
            state,
            halves
        );
        prop_assert_eq!(
            halves,
            [d, drift_bits(de), drift_bits(dlz)],
            "state {}: the halves are not the clamped offline latches",
            state
        );
    }
    Ok(())
}

/// The fuzz property over `pack`: any state code, one to eight observations.
fn fuzz(pack: Persist) {
    let obs = proptest::collection::vec((value(), value(), value()), 1..8);
    prop::run(&(0u32..8, obs), |(state, obs)| {
        check_fuzz(pack, state, &obs)
    });
}

#[test]
fn f16_clamp_fuzz() {
    fuzz(persist);
}

negative_control!(
    f16_clamp_fuzz,
    "an unclamped pack lets ±∞ and values past 65504 through",
    expected = "is not finite and clamped",
    fuzz(unclamped)
);

// ── latch_across_persist (REQ-PAY-044) ───────────────────────────────────────────────────────────────────────────

/// A resume: the latches a sample continues from at a persist boundary, from its persisted words.
type Resume = fn(u32, u32) -> Latches;

/// The resume under test: [`Latches::reload`].
fn reload(a: u32, b: u32) -> Latches {
    Latches::reload(a, b)
}

/// The words a `running` sample persists after observing `obs` (`(d², ΔE, ΔL_z)` each) in one march, and after
/// observing them across persist boundaries at each index of `cuts`, resumed from its words by `resume` at each.
fn persisted(obs: &[(f32, f32, f32)], cuts: &[usize], resume: Resume) -> ((u32, u32), (u32, u32)) {
    let run = |l: &mut Latches, part: &[(f32, f32, f32)]| {
        for &(d2, de, dlz) in part {
            l.separation_squared(d2);
            l.drift(de, dlz);
        }
    };
    let mut single = Latches::unset();
    run(&mut single, obs);
    let a0 = descriptor(STATE_RUNNING);
    let mut words = persist(&Latches::unset(), a0);
    let mut start = 0;
    for &end in cuts.iter().chain([obs.len()].iter()) {
        let mut l = resume(words.0, words.1);
        run(&mut l, &obs[start..end]);
        words = persist(&l, words.0);
        start = end;
    }
    (persist(&single, a0), words)
}

/// Asserts that a sample resumed by `resume` across the persist boundaries `cuts` stores the f16 of its one f32
/// latch over the whole of `obs`, bit for bit.
fn check_persist(resume: Resume, obs: &[(f32, f32, f32)], cuts: &[usize]) {
    let (single, resumed) = persisted(obs, cuts, resume);
    assert_eq!(
        resumed, single,
        "resumed at {cuts:?}: the persisted latches {resumed:08x?} differ from the single latch's {single:08x?}"
    );
}

/// A finite measurement: a magnitude spread over binary16's range and past it, either sign.
fn measure() -> impl Strategy<Value = f32> {
    (any::<bool>(), -30i32..20, 1.0f32..2.0).prop_map(|(neg, e, m)| {
        let x = m * 2f32.powi(e);
        if neg {
            -x
        } else {
            x
        }
    })
}

/// The property over `resume`: up to 64 finite observations, cut at any subset of their indices.
fn across(resume: Resume) {
    let obs = proptest::collection::vec((measure(), measure(), measure()), 1..64);
    let cases = obs.prop_flat_map(|o| {
        let n = o.len();
        (Just(o), proptest::collection::btree_set(0..=n, 0..n + 1))
    });
    prop::run(&cases, |(obs, cuts)| {
        let obs: Vec<_> = obs.iter().map(|&(d, de, dlz)| (d * d, de, dlz)).collect();
        let cuts: Vec<usize> = cuts.into_iter().collect();
        check_persist(resume, &obs, &cuts);
        Ok(())
    });
}

/// A march's own per-step measurements: the least quantised `d²` and the drifts `ΔE`, `ΔL_z` at f32, of each state of
/// a recorded KDK march from the close-encounter fixture, the IC included.
fn march_measurements() -> Vec<(f32, f32, f32)> {
    let fixture = close_encounter();
    let (trace, _) = recorded::<f64>(&fixture.ic, &sim_key::<f64>(fixture.horizon));
    trace.measurements()
}

#[test]
fn latch_across_persist() {
    across(reload);
    let obs = march_measurements();
    check_moves("the close-encounter march", &obs);
    for every in [1, 2, 3, 7, 64, 1000, obs.len()] {
        let cuts: Vec<usize> = (every..obs.len()).step_by(every).collect();
        check_persist(reload, &obs, &cuts);
    }
}

negative_control!(
    latch_across_persist,
    "a resume that starts its latches afresh forgets the earlier segments",
    expected = "differ from the single latch's",
    {
        let obs = [(4.0, 1.0, 0.5), (9.0, 0.25, 0.125)];
        check_persist(|_, _| Latches::unset(), &obs, &[1]);
    }
);

// ── latches_vs_offline (REQ-PAY-050) ─────────────────────────────────────────────────────────────────────────────

/// A state's positions and momenta.
type Phase<R> = ([[R; 2]; 3], [[R; 2]; 3]);

/// A recorded march: the masses, the IC and each post-projection state the cadence saw.
struct Trace<R> {
    m: [R; 3],
    states: Vec<Phase<R>>,
}

impl<R: WrapperReal> Trace<R> {
    /// Each state's least quantised `d²` and its drifts from the IC's invariants, rounded to f32, the IC first.
    fn measurements(&self) -> Vec<(f32, f32, f32)> {
        let (r0, p0) = self.states[0];
        let e0 = energy(&self.m, &p0, &Geom::new(&self.m, &r0).pair);
        let l0 = angular_momentum(&r0, &p0);
        self.states
            .iter()
            .map(|(r, p)| {
                let d2 = pair_d2_quantised(r)
                    .into_iter()
                    .fold(f32::INFINITY, f32::min);
                let de = energy(&self.m, p, &Geom::new(&self.m, r).pair) - e0;
                let dl = angular_momentum(r, p) - l0;
                (d2, de.quantise(), dl.quantise())
            })
            .collect()
    }
}

/// A cadence that records each state the wrapper's cadence leaves, after its projection, and does what it does.
struct Recording<R> {
    inner: WrapperCadence<R>,
    states: Vec<Phase<R>>,
}

impl<R: WrapperReal> Cadence<R> for Recording<R> {
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.inner.after_step(state, geom);
        self.states.push((state.r, state.p));
    }

    fn done(&self) -> bool {
        self.inner.done()
    }

    fn bucket_d2(&self, r: &[[R; 2]; 3]) -> f32 {
        self.inner.bucket_d2(r)
    }
}

/// The KDK march of `ic` under `u` at `R`, recorded, and its stored state with the monitored fields written.
fn recorded<R: WrapperReal>(ic: &Ic<f64>, u: &WrapperUniforms<R>) -> (Trace<R>, SimStateBaseOf<R>)
where
    f64: AsR<R>,
{
    let ic = ic_at::<R>(ic);
    let mut rec = Recording {
        inner: WrapperCadence::new(&ic.m, &ic.r, &ic.p),
        states: vec![(ic.r, ic.p)],
    };
    let out = integrate_with::<Kdk, R, _>(&ic, u, &mut rec);
    let out = monitored(out, &rec.inner);
    (
        Trace {
            m: ic.m,
            states: rec.states,
        },
        out,
    )
}

/// Asserts that the stored words `(packed_a, packed_b)` hold the f16 of the offline latches of `obs`: the least
/// `√d²` and the greatest `|ΔE|` and `|ΔL_z|` over every state, each clamped.
fn check_offline(what: &str, obs: &[(f32, f32, f32)], (a, b): (u32, u32)) {
    let (d2, de, dlz) = obs.iter().fold(
        (f32::INFINITY, 0.0f32, 0.0f32),
        |(d2, de, dl), &(x, y, z)| (d2.min(x), de.max(y.abs()), dl.max(z.abs())),
    );
    let want = [d_min_oracle(d2.sqrt()), drift_bits(de), drift_bits(dlz)];
    let got = [d_min_bits(a), b as u16, (b >> 16) as u16];
    assert_eq!(
        got, want,
        "{what}: the stored latches {got:04x?} are not the offline ones {want:04x?}"
    );
}

/// The march's uniforms at `R`: `dt_macro = 0.01` for 100 macro-steps, the default substep law.
fn short<R: WrapperReal>() -> WrapperUniforms<R>
where
    f64: AsR<R>,
{
    let law = NsubConfig::DEFAULT;
    WrapperUniforms {
        dt_macro: 0.01.to_r(),
        horizon_steps: 100,
        n_max: law.n_max,
        thresholds: thresholds(&law).expect("the default substep law generates"),
    }
}

/// The latent property at `R`: a march from each decoded latent stores the offline latches of its trajectory.
fn offline_property<R: WrapperReal>()
where
    f64: AsR<R>,
{
    let z = proptest::array::uniform8(-1.5f64..1.5);
    prop::run(&z, |z| {
        let ic = decode::decode::<Defaults, f64>(&Latent::new(z)).ic;
        let finite =
            ic.r.iter()
                .chain(ic.p.iter())
                .flatten()
                .all(|x| x.is_finite());
        prop_assume!(finite);
        let (trace, out) = recorded::<R>(&ic, &short::<R>());
        prop_assume!(sd_state(out.packed_a) != STATE_SIM_FAILED);
        check_offline(
            &format!("{} from {z:?}", std::any::type_name::<R>()),
            &trace.measurements(),
            (out.packed_a, out.packed_b),
        );
        Ok(())
    });
}

#[test]
fn latches_vs_offline() {
    offline_property::<f32>();
    offline_property::<f64>();
    let fixture = close_encounter();
    let (trace, out) = recorded::<f64>(&fixture.ic, &sim_key::<f64>(fixture.horizon));
    let obs = trace.measurements();
    check_moves(fixture.name, &obs);
    check_offline(fixture.name, &obs, (out.packed_a, out.packed_b));
}

/// Asserts that the measurements `obs` exercise the latches: the least `d²` is met after the IC, not at it, and
/// neither drift is zero throughout.
fn check_moves(what: &str, obs: &[(f32, f32, f32)]) {
    let least = obs.iter().map(|o| o.0).fold(f32::INFINITY, f32::min);
    assert!(
        least < obs[0].0,
        "{what}: the least separation is the IC's, so the trace does not exercise d_min's latch"
    );
    assert!(
        obs.iter().any(|o| o.1 != 0.0) && obs.iter().any(|o| o.2 != 0.0),
        "{what}: a drift is zero throughout, so the trace does not exercise its latch"
    );
}

negative_control!(
    latches_vs_offline,
    "latches of the final state alone are not the trajectory's",
    expected = "are not the offline ones",
    {
        let fixture = close_encounter();
        let (trace, out) = recorded::<f64>(&fixture.ic, &sim_key::<f64>(fixture.horizon));
        let obs = trace.measurements();
        let &(d2, de, dlz) = obs.last().expect("the trace holds the IC");
        let mut last = Latches::unset();
        last.separation_squared(d2);
        last.drift(de, dlz);
        check_offline(fixture.name, &obs, persist(&last, out.packed_a));
    }
);

// ── latch_t0_collision_d_min (R-933) ─────────────────────────────────────────────────────────────────────────────

/// Equal masses, bodies 0 and 1 `5·10⁻⁴` apart, body 2 far: inside the default `r_coll = 10⁻³`, pair 2.
fn collided() -> Ic<f64> {
    let third = 1.0 / 3.0;
    Ic {
        m: [third; 3],
        r: [[-0.00025, -0.3], [0.00025, -0.3], [0.0, 0.6]],
        p: [[0.1, 0.0], [-0.1, 0.0], [0.0, 0.0]],
    }
}

/// Asserts that `packed_a` of a start already collided is `collision` at `t_end_step` 0 with `d_min` the IC's least
/// separation, never unset.
fn check_t0_d_min(ic: &Ic<f64>, (a, times): (u32, u32)) {
    assert_eq!(
        (sd_state(a), tm_t_end_step(times)),
        (STATE_COLLISION, 0),
        "the start is not a collision at t = 0"
    );
    assert_ne!(
        d_min_bits(a),
        0x7c00,
        "the t = 0 collision's d_min is unset"
    );
    let d2 = pair_d2_quantised(&ic.r)
        .into_iter()
        .fold(f32::INFINITY, f32::min);
    assert_eq!(
        d_min_bits(a),
        d_min_oracle(d2.sqrt()),
        "the t = 0 collision's d_min is not the IC's least separation"
    );
}

/// The run of `ic` under the default `r_coll`, its `packed_a` and `times`.
fn t0_run(ic: &Ic<f64>) -> (u32, u32) {
    let u = WrapperUniforms {
        horizon_steps: 50,
        ..short::<f64>()
    };
    let out = integrate_detecting::<Kdk, f64>(ic, &u, CollisionRadius::from_r_coll(1e-3));
    (out.packed_a, out.times)
}

#[test]
fn latch_t0_collision_d_min() {
    let ic = collided();
    check_t0_d_min(&ic, t0_run(&ic));
    let mut coincident = collided();
    coincident.r[1] = coincident.r[0];
    check_t0_d_min(&coincident, t0_run(&coincident));
}

negative_control!(
    latch_t0_collision_d_min,
    "a t = 0 collision written with its latches unobserved keeps d_min unset",
    expected = "d_min is unset",
    {
        let ic = collided();
        let (a, times) = t0_run(&ic);
        check_t0_d_min(&ic, (persist(&Latches::unset(), a).0, times));
    }
);
