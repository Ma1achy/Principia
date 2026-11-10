//! The collision detector and the `dmin_pair` latch (integrator_contract Part 7; dd_integrator §3.6; payload §2).
//!
//! **One computation per step, three consumers** (REQ-EVT-004). After every `STEP`, on the projected state, the
//! [`Detector`] forms the three pairs' squared separations once ([`Separations::of`]), in the substep law's
//! position-quantised form (`crate::driver::substep`: each position cast to f32, one subtraction per axis, each square
//! by `square`, one addition; R-547). The collision test reads them, the `dmin_pair` latch reads their least, and the
//! next macro-step's `N_sub` bucket reads that same least through the cadence (`Cadence::bucket_d2`), so no second
//! `d²` of the state is formed. `d_min` is the latch's `√`, packed by TASK-M3-10's latch writer; until then the
//! payload's `d_min` half stays unset.
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

use crate::driver::substep::{pair_d2_quantised, Quantise};
use crate::payload::{
    set_detail, set_dmin_pair, set_state, DETAIL_COLLISION_ALL_THREE_TRIPLE_COLLISION,
    DETAIL_COLLISION_PAIR_0, DETAIL_COLLISION_PAIR_1, DETAIL_COLLISION_PAIR_2, STATE_COLLISION,
};
use crate::real::IeeeBits;

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

/// The sim key's collision threshold as the detector reads it: `r_coll²`, an f32 (the module's note).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionRadius {
    r_coll2: f32,
}

impl CollisionRadius {
    /// `r_coll = 0`, Part 7's "refuse to regularise": `r_coll² = 0`, which no `d²` is below, so no collision fires.
    #[inline]
    pub fn none() -> Self {
        CollisionRadius { r_coll2: 0.0 }
    }

    /// The threshold whose `r_coll²` is the f32 `r_coll2`, as `SimUniforms` would carry it to the device.
    #[inline]
    pub fn from_squared(r_coll2: f32) -> Self {
        CollisionRadius { r_coll2 }
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
        CollisionRadius {
            r_coll2: if r_coll > 0.0 {
                ((r_coll * r_coll) as f32).max(f32::MIN_POSITIVE)
            } else {
                0.0
            },
        }
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
        let (d2, pair) = s.closest();
        if d2 < self.d2 || self.d2.nan_bits() {
            self.d2 = d2;
            self.pair = pair;
        }
    }

    /// The least quantised `d²` met so far: `d_min²`, the value TASK-M3-10's latch writer packs as `d_min = √`.
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
/// collision verdict, which stays once fired.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Detector {
    radius: CollisionRadius,
    last: Separations,
    latch: DminLatch,
    collision: Verdict,
}

impl Detector {
    /// The detector of a run from the positions `r` under `radius`: the IC observed, before the first step, so a run
    /// that begins inside `r_coll` is a collision at step 0 (integrator_contract Part 5, "t=0 terminals are real
    /// outcomes") and the latch begins at the IC.
    #[inline]
    pub fn new<R: Quantise>(radius: CollisionRadius, r: &[[R; 2]; 3]) -> Self {
        let s = Separations::of(r);
        Detector {
            radius,
            last: s,
            latch: DminLatch::new(&s),
            collision: classify(&s, &radius),
        }
    }

    /// Observes the projected state at positions `r` after a `STEP`: its separations formed once, the latch raised and
    /// the collision tested. A fired verdict stays: a collision is terminal (Part 7).
    #[inline]
    pub fn observe<R: Quantise>(&mut self, r: &[[R; 2]; 3]) {
        let s = Separations::of(r);
        self.last = s;
        self.latch.update(&s);
        if !self.collision.fired {
            self.collision = classify(&s, &self.radius);
        }
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
