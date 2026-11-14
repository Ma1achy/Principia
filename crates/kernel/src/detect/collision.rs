//! The collision detector and the `dmin_pair` latch (integrator_contract Part 7; dd_integrator §3.6; payload §2).
//!
//! **One computation per step, three consumers** (REQ-EVT-004). After every `STEP`, on the projected state, the
//! [`Detector`] forms the three pairs' squared separations once ([`Separations::of`]), in the substep law's
//! position-quantised form (`crate::driver::substep`: each position cast to f32, one subtraction per axis, each square
//! by `square`, one addition; R-547). The collision test reads them, the `dmin_pair` latch reads their least, and the
//! next macro-step's `N_sub` bucket reads that same least through the cadence (`Cadence::bucket_d2`), so no second
//! `d²` of the state is formed. `d_min` is the latch's `√`, packed by the latch writer
//! (`crate::driver::latch::Latches`).
//!
//! **Count before classifying** (Part 7, pending change 7, landed): each pair `k` is tested `d²ₖ < r_coll²`, strictly,
//! against the sim key's threshold ([`CollisionRadius`]). Exactly one pair below it is `collision` with `detail` that
//! pair's R-22 id, pair `k` the side opposite body `k`; two or more is `collision` with `detail = 3`, a triple
//! collision, terminal and non-continuable. Testing "any pair below" first would steal a genuine triple into the binary
//! arm. The comparison is on `d²`, so no `sqrt` reaches the branch (dd_integrator §3.3's comparison-only rule).
//!
//! **The threshold** ([`CollisionRadius::from_r_coll`]). `r_coll` is a length on the sim key, the same value on every
//! pipeline (Part 7). The host squares it in f64 and rounds the square once to f32, as the substep law's thresholds are
//! formed (R-502), so every backend compares the same `d²` with the same bits. For `r_coll > 0` a square below f32's
//! least normal value, subnormal or rounded to 0, is raised to that least normal, `2⁻¹²⁶`, never written as a
//! subnormal a backend may flush (gpu_determinism_note rule 6, R-547) nor as 0, which would turn a positive `r_coll`
//! into `r_coll = 0`. The quantised `d²` is 0 or at least `2⁻⁸⁰` (`square`'s floor), so against `2⁻¹²⁶` exactly
//! `d² = 0` fires, the decision `d² < r_coll²` makes for every such `r_coll` on every `d²` the detector can meet: bodies
//! whose f32 positions coincide collide under any positive `r_coll`, at `t = 0` too (Part 5, "t=0 terminals are real
//! outcomes"). Dispatch refuses an `r_coll` whose f32 square is not finite (`engine::dispatch::validate`), so the
//! threshold is never `+∞` (rule 4). `r_coll = 0` is Part 7's "refuse to regularise": its threshold is 0, and no pair is
//! below it ([`CollisionRadius::none`]).
//!
//! **The latch** ([`DminLatch`]; payload §2, REQ-PAY-047): the least quantised `d²` the run has met, the IC's
//! included, since `d_min = min_t |separation|` runs over the trajectory from `t = 0`, and the R-22 id of the pair that
//! met it, the lowest id of a tie. A later state replaces it only when strictly closer, so the first state to reach
//! the minimum keeps it. It is seeded from the IC's separations, never from an infinity (Part 2c rule 4), and it is a
//! latched fact, never derived from the word (symbolic-dynamics contract, "What is already settled"). The payload
//! holds `dmin_pair`'s sentinel, 3, until the latch is written into it ([`Detector::label`]).
//!
//! **The floor's record** (Part 7, "The floor's record, defined"; R-954, R-973; REQ-EVT-035). The test resolves a
//! separation only to the f32 spacing at the pair's coordinates, so the detector records, per sample, the largest
//! floor over every pair near firing at every state it tests and over the `d_min` state. A pair's floor at a state is
//! [`floor_codes`]'s code, `c + 41` for the floor `2^c`, `c = clamp(e − 23, −40, 104)`, `e` the largest exponent field
//! of the pair's four f32-cast coordinates. A pair is near firing ([`near_firing`]) where the test fires on it or where
//! `d² < t·t`, `t = r ⊕ floor` with `r` the threshold's root ([`CollisionRadius::root`]): `d_quant < r_coll + floor`
//! in squared form, no `sqrt` in the branch, every input normal (R-547). [`Detector::floor_code`] is the code the
//! payload's `coll_floor` stores, the larger of the near-firing maximum and the code of the pair and state the latch
//! holds.

use crate::driver::substep::{pair_d2_quantised, Quantise};
use crate::payload::{
    set_detail, set_dmin_pair, set_state, DETAIL_COLLISION_ALL_THREE_TRIPLE_COLLISION,
    DETAIL_COLLISION_PAIR_0, DETAIL_COLLISION_PAIR_1, DETAIL_COLLISION_PAIR_2, STATE_COLLISION,
};
use crate::real::IeeeBits;
#[cfg(not(target_arch = "spirv"))]
use spirv_std::num_traits::Float;

/// The code of the least floor, `2⁻⁴⁰`: the floor `2^c` is stored as `c + 41`, so codes run from 1 (`2⁻⁴⁰`) to 145
/// (`2¹⁰⁴`), and 0 is "not recorded" (payload §2, `coll_floor_bits`; integrator contract Part 7).
pub const FLOOR_CODE_BIAS: i32 = 41;

/// The least floor's exponent, `−40`: below `2⁻⁴⁰` the test's per-axis square is 0 (`crate::driver::substep::square`).
pub const FLOOR_EXP_MIN: i32 = -40;

/// The greatest floor's exponent, `104`: the f32 spacing at the largest finite exponent, 127.
pub const FLOOR_EXP_MAX: i32 = 104;

/// The floor code of a pair whose largest coordinate exponent field is `e` (the module's note): `clamp(e − 23, −40,
/// 104) + 41`, from 1 to 145.
#[inline]
pub fn floor_code_of_exponent(e: i32) -> u32 {
    ((e - 23).clamp(FLOOR_EXP_MIN, FLOOR_EXP_MAX) + FLOOR_CODE_BIAS) as u32
}

/// The floor `2^(code − 41)` of a code from 1 to 145, formed from its bits.
#[inline]
pub fn floor_of_code(code: u32) -> f32 {
    f32::pow2(code as i32 - FLOOR_CODE_BIAS)
}

/// Each R-22 pair's floor code at the positions `r` (the module's note): each position cast to f32 as the test's `d²`
/// casts it, the largest exponent field of the pair's four coordinates, read from the bits, through
/// [`floor_code_of_exponent`]. Pair `k` is the side opposite body `k`.
#[inline]
pub fn floor_codes<R: Quantise>(r: &[[R; 2]; 3]) -> [u32; 3] {
    let e = |b: usize| {
        let (x, y) = (r[b][0].quantise().exponent(), r[b][1].quantise().exponent());
        if x > y {
            x
        } else {
            y
        }
    };
    let (e0, e1, e2) = (e(0), e(1), e(2));
    let max = |a: i32, b: i32| if a > b { a } else { b };
    [
        floor_code_of_exponent(max(e1, e2)),
        floor_code_of_exponent(max(e2, e0)),
        floor_code_of_exponent(max(e0, e1)),
    ]
}

/// Whether a pair whose quantised `d²` is `d2` and whose floor code is `code` is near firing under `radius` (the
/// module's note): the test fires on it, `d2 < r_coll²`, or `d2 < t·t` with `t = r ⊕ 2^(code − 41)`, `r` the
/// threshold's root, `t·t` by [`IeeeBits::product`]; at `t ≥ 2⁶⁴`, read on its exponent field, near firing without the
/// product, which would overflow.
#[inline]
pub fn near_firing(d2: f32, code: u32, radius: &CollisionRadius) -> bool {
    let t = radius.root() + floor_of_code(code);
    d2 < radius.squared() || t.exponent() >= 64 || d2 < t.product(t)
}

/// The three pairs' position-quantised squared separations, by R-22 pair id: `d2[k]` is pair `k`'s, the side opposite
/// body `k`, `(1, 2), (2, 0), (0, 1)`. Formed once per state ([`Separations::of`]) and read by every consumer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Separations {
    /// `d²ₖ` for pair `k`.
    pub d2: [f32; 3],
}

impl Separations {
    /// The separations of the positions `r`: the substep law's quantised form, per pair
    /// (`crate::driver::substep::pair_d2_quantised`).
    #[inline]
    pub fn of<R: Quantise>(r: &[[R; 2]; 3]) -> Self {
        Separations {
            d2: pair_d2_quantised(r),
        }
    }

    /// The least `d²` and its pair's id, by comparison: pair 1 replaces pair 0 where strictly below it, then pair 2
    /// replaces the lesser where strictly below that, so a tie keeps the lowest id and a NaN in pair 0's place is kept.
    /// Constant indices only (gpu_determinism_note rule 5).
    #[inline]
    pub fn closest(&self) -> (f32, u32) {
        let [d0, d1, d2] = self.d2;
        let (a, ka) = if d1 < d0 {
            (d1, DETAIL_COLLISION_PAIR_1)
        } else {
            (d0, DETAIL_COLLISION_PAIR_0)
        };
        if d2 < a {
            (d2, DETAIL_COLLISION_PAIR_2)
        } else {
            (a, ka)
        }
    }

    /// The least `d²`, the one the substep bucket reads ([`Separations::closest`]'s value).
    #[inline]
    pub fn min(&self) -> f32 {
        self.closest().0
    }
}

/// The sim key's collision threshold as the detector reads it: `r_coll²`, an f32 (the module's note), and its root,
/// `√(r_coll²)` rounded once to f32, which the near-firing test of the floor's record reads ([`near_firing`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionRadius {
    r_coll2: f32,
    root: f32,
}

impl CollisionRadius {
    /// `r_coll = 0`, Part 7's "refuse to regularise": `r_coll² = 0`, which no `d²` is below, so no collision fires.
    #[inline]
    pub fn none() -> Self {
        CollisionRadius {
            r_coll2: 0.0,
            root: 0.0,
        }
    }

    /// The threshold whose `r_coll²` is the f32 `r_coll2`, as `SimUniforms` would carry it to the device, its root
    /// formed on the host: `√r_coll2` in f64, correctly rounded, then rounded once to f32, which is the f32 square
    /// root correctly rounded (53 ≥ 2·24 + 2 bits).
    #[cfg(not(target_arch = "spirv"))]
    pub fn from_squared(r_coll2: f32) -> Self {
        CollisionRadius {
            r_coll2,
            root: Float::sqrt(f64::from(r_coll2)) as f32,
        }
    }

    /// The threshold of `r_coll2` and `root` as the host formed them ([`CollisionRadius::from_squared`]), the form
    /// the device takes them in.
    #[inline]
    pub fn from_parts(r_coll2: f32, root: f32) -> Self {
        CollisionRadius { r_coll2, root }
    }

    /// `√(r_coll²)` rounded to f32: 0 at `r_coll = 0`, else at least `2⁻⁶³`, the root of f32's least normal value.
    #[inline]
    pub fn root(&self) -> f32 {
        self.root
    }

    /// `r_coll²`, the value each pair's `d²` is compared with.
    #[inline]
    pub fn squared(&self) -> f32 {
        self.r_coll2
    }

    /// The threshold of the sim key's `r_coll` (the module's note): for `r_coll > 0`, `r_coll²` formed in f64 and
    /// rounded once to f32, raised to f32's least normal value where it is below it; for `r_coll = 0`, 0. Dispatch has
    /// refused an `r_coll` that is not finite or is negative, or whose f32 square is not finite
    /// (`engine::dispatch::validate`), so the threshold is finite and never subnormal.
    #[cfg(not(target_arch = "spirv"))]
    pub fn from_r_coll(r_coll: f64) -> Self {
        Self::from_squared(if r_coll > 0.0 {
            ((r_coll * r_coll) as f32).max(f32::MIN_POSITIVE)
        } else {
            0.0
        })
    }
}

/// A collision test's verdict: whether it fired, and its `detail`, the pair id or 3 (meaningful only where fired).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Verdict {
    /// Whether at least one pair is inside `r_coll`.
    pub fired: bool,
    /// The pair's R-22 id where exactly one pair is inside, 3 (triple collision) where two or more are; where not
    /// fired, 2, which nothing reads.
    pub detail: u32,
}

/// The collision test on `s` against `radius` (the module's note): the pairs with `d²ₖ < r_coll²` counted, then
/// classified: none, not fired; one, that pair; two or three, the triple collision.
#[inline]
pub fn classify(s: &Separations, radius: &CollisionRadius) -> Verdict {
    let t = radius.squared();
    let [d0, d1, d2] = s.d2;
    let (in0, in1, in2) = (d0 < t, d1 < t, d2 < t);
    let count = u32::from(in0) + u32::from(in1) + u32::from(in2);
    let detail = if count > 1 {
        DETAIL_COLLISION_ALL_THREE_TRIPLE_COLLISION
    } else if in0 {
        DETAIL_COLLISION_PAIR_0
    } else if in1 {
        DETAIL_COLLISION_PAIR_1
    } else {
        DETAIL_COLLISION_PAIR_2
    };
    Verdict {
        fired: count > 0,
        detail,
    }
}

/// The `dmin_pair` latch (the module's note): the least quantised `d²` met so far and its pair's R-22 id.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DminLatch {
    d2: f32,
    pair: u32,
}

impl DminLatch {
    /// The latch of a run whose first state's separations are `s`: their least and its pair.
    #[inline]
    pub fn new(s: &Separations) -> Self {
        let (d2, pair) = s.closest();
        DminLatch { d2, pair }
    }

    /// The latch raised by a later state's separations `s`: replaced where `s`'s least is strictly below the latched
    /// one, or where the latched one is NaN (a NaN IC's; tested on the bits, gpu_determinism_note rule 4); kept
    /// otherwise, a NaN `s` included.
    #[inline]
    pub fn update(&mut self, s: &Separations) {
        if self.replaced_by(s) {
            let (d2, pair) = s.closest();
            self.d2 = d2;
            self.pair = pair;
        }
    }

    /// Whether [`DminLatch::update`] replaces the latch with `s`'s least: strictly below it, or the latched one NaN.
    #[inline]
    pub fn replaced_by(&self, s: &Separations) -> bool {
        s.min() < self.d2 || self.d2.nan_bits()
    }

    /// The least quantised `d²` met so far: `d_min²`, the value the latch writer packs as `d_min = √`
    /// (`crate::driver::latch::Latches`).
    #[inline]
    pub fn d2(&self) -> f32 {
        self.d2
    }

    /// The R-22 id of the pair that met it: `dmin_pair`.
    #[inline]
    pub fn pair(&self) -> u32 {
        self.pair
    }
}

/// The run's detector state (the module's note): the threshold, the last state's separations, the latch and the
/// collision verdict, which stays once fired, and the floor's record: the largest floor code of a pair near firing,
/// and the code of the pair and state the latch holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Detector {
    radius: CollisionRadius,
    last: Separations,
    latch: DminLatch,
    collision: Verdict,
    near: u32,
    latched: u32,
}

/// The largest floor code of the pairs of `s`, with codes `codes`, near firing under `radius`, and `near` where none
/// is larger. Constant indices only (gpu_determinism_note rule 5).
#[inline]
fn near_max(near: u32, s: &Separations, codes: &[u32; 3], radius: &CollisionRadius) -> u32 {
    let raise = |out: u32, d2: f32, code: u32| {
        if code > out && near_firing(d2, code, radius) {
            code
        } else {
            out
        }
    };
    let out = raise(near, s.d2[0], codes[0]);
    let out = raise(out, s.d2[1], codes[1]);
    raise(out, s.d2[2], codes[2])
}

/// The floor code of the pair `pair` (an R-22 id, 0–2) of `codes`, by constant indices (gpu_determinism_note rule 5).
#[inline]
fn code_of(codes: &[u32; 3], pair: u32) -> u32 {
    if pair == DETAIL_COLLISION_PAIR_0 {
        codes[0]
    } else if pair == DETAIL_COLLISION_PAIR_1 {
        codes[1]
    } else {
        codes[2]
    }
}

impl Detector {
    /// The detector of a run from the positions `r` under `radius`: the IC observed, before the first step, so a run
    /// that begins inside `r_coll` is a collision at step 0 (integrator_contract Part 5, "t=0 terminals are real
    /// outcomes") and the latch begins at the IC.
    #[inline]
    pub fn new<R: Quantise>(radius: CollisionRadius, r: &[[R; 2]; 3]) -> Self {
        let s = Separations::of(r);
        let codes = floor_codes(r);
        let latch = DminLatch::new(&s);
        Detector {
            radius,
            last: s,
            latch,
            collision: classify(&s, &radius),
            near: near_max(0, &s, &codes, &radius),
            latched: code_of(&codes, latch.pair()),
        }
    }

    /// Observes the projected state at positions `r` after a `STEP`: its separations formed once, the latch raised and
    /// the collision tested. A fired verdict stays: a collision is terminal (Part 7).
    #[inline]
    pub fn observe<R: Quantise>(&mut self, r: &[[R; 2]; 3]) {
        let s = Separations::of(r);
        let codes = floor_codes(r);
        self.last = s;
        if self.latch.replaced_by(&s) {
            self.latched = code_of(&codes, s.closest().1);
        }
        self.latch.update(&s);
        if !self.collision.fired {
            self.near = near_max(self.near, &s, &codes, &self.radius);
            self.collision = classify(&s, &self.radius);
        }
    }

    /// The floor's record (the module's note): the larger of the largest floor code of a pair near firing at a state
    /// the test read and the code of the pair and state the latch holds, 1 to 145, the code the payload's
    /// `coll_floor` stores.
    #[inline]
    pub fn floor_code(&self) -> u32 {
        if self.near > self.latched {
            self.near
        } else {
            self.latched
        }
    }

    /// The largest floor code of a pair near firing at a state the test read, 0 where none was.
    #[inline]
    pub fn near_code(&self) -> u32 {
        self.near
    }

    /// The floor code of the pair and state the `d_min` latch holds.
    #[inline]
    pub fn latched_code(&self) -> u32 {
        self.latched
    }

    /// Whether a collision has fired, which sets the run's `done` flag.
    #[inline]
    pub fn fired(&self) -> bool {
        self.collision.fired
    }

    /// The collision verdict.
    #[inline]
    pub fn collision(&self) -> Verdict {
        self.collision
    }

    /// The least `d²` of the last state observed, the one the next macro-step's bucket reads.
    #[inline]
    pub fn last_d2(&self) -> f32 {
        self.last.min()
    }

    /// The last state's separations.
    #[inline]
    pub fn last(&self) -> Separations {
        self.last
    }

    /// The `dmin_pair` latch.
    #[inline]
    pub fn latch(&self) -> DminLatch {
        self.latch
    }

    /// The threshold.
    #[inline]
    pub fn radius(&self) -> CollisionRadius {
        self.radius
    }

    /// `packed_a` with what the detector holds written through the generated descriptor setters (payload §2):
    /// `dmin_pair` the latch's pair, and, where a collision fired, `state` `collision` and its `detail`; every other
    /// bit kept.
    #[inline]
    pub fn label(&self, packed_a: u32) -> u32 {
        let w = set_dmin_pair(packed_a, self.latch.pair);
        if self.collision.fired {
            set_detail(set_state(w, STATE_COLLISION), self.collision.detail)
        } else {
            w
        }
    }
}
