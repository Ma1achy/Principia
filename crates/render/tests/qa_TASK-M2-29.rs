//! QA tests for TASK-M2-29 off the GPU, written from REQ-COL-060 and colour_composition §6 ("Debug views as presets",
//! "The tolerance's scale", R-72; RQ-623, decided per R-369, option (b)), not from the implementation.
//!
//! The scale is `S = max(|K₀|, 1) + max(|V₀|, 1) + Σ_{i<j} mᵢmⱼ·wᵢⱼ/ŝᵢⱼ²`, `wᵢⱼ = ‖rᵢ‖∞ + ‖rⱼ‖∞`,
//! `ŝᵢⱼ = max(‖rᵢ − rⱼ‖∞, eps·max(wᵢⱼ, 1))`, and a sample agrees where `|E − E₀| ≤ F·eps·S`, `F` REQ-COL-060's factor
//! (`render::presets::agreement::FACTOR`, provisional, R-71).
//!
//! - `qa_m229_ratio_is_the_definition`: `agreement::conditioning` and `agreement::ratio` against the scale written here
//!   from §6's text, over fuzzed masses on the simplex and positions, coincident pairs and the floor's edges included;
//!   §6's claim that, where `ŝᵢⱼ` is the separation, the sum is within a factor 2 of the Euclidean
//!   `Σ mᵢmⱼ(‖rᵢ‖ + ‖rⱼ‖)/rᵢⱼ²`. Control: the Euclidean form taken as the definition must differ from the max-norm one.
//! - `qa_m229_term_finite_in_every_association` (condition 1; R-436): with masses on `[0, 1]` summing to 1 and
//!   positions below `2⁶²`, coincident pairs included, each pair's term at f32 is finite and at most `2⁴⁴` in each
//!   association a fast-math compiler may pick (`(mm/s)·(w/s)`, `mm·w/(s·s)`, `(mm·w/s)/s`, `mm·(w/(s·s))`). Control:
//!   the term without the floor must be non-finite at the pole.
//! - `qa_m229_f32_energy_flat_under_refinement` (condition 2, condition 4's second control, PIT-3): a binary
//!   approaching collision, its separation halved from `w/2` down to the floor's `4·eps·w`: `E` evaluated at f32 from
//!   the f32-rounded positions against `E₀` evaluated exactly (f64) from the same configuration, as any two f32
//!   evaluations differ beside a collision. Judged by `agreement::ratio`, every sample agrees at every refinement, and
//!   over the sweep's second half, where `κ = w/d ≥ 2¹¹`, the worst factor at each refinement stays within a factor 2
//!   of every other's, the growth of a `1/d` quantity over a single halving (pitfalls §3: flat under refinement).
//!   Control: the per-operand form, `max(|K₀|, 1) + max(|V₀|, 1)` alone, judged the same way, must fail it; and `qa_m229_flatness_alone`'s control,
//!   the per-operand form with the tolerance not asserted, must fail the flatness check.
//!
//! **Domain** (pitfalls §10): one configuration at a time, no trajectory; the binary's `wᵢⱼ` on `[1, 2]`, positions of
//! order 1 as the decode's are under `I = 1`, and its separation within §6's floor's claim, `ŝᵢⱼ ≥ 4·eps·wᵢⱼ`, where the
//! f32 separation keeps at least two relative bits.

use render::presets::agreement::{self, FACTOR};
use validation::negative_control;

const EPS: f64 = f32::EPSILON as f64;

/// A splitmix64 stream, its seed fixed: the fuzzed configurations are the same in every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform on `[0, 1)`.
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform on `[−a, a)`.
    fn sym(&mut self, a: f64) -> f64 {
        a * (2.0 * self.unit() - 1.0)
    }

    /// Masses on the simplex, `M = 1`, each at least `lo`.
    fn masses(&mut self, lo: f64) -> [f64; 3] {
        let a = lo + (1.0 - 3.0 * lo) * self.unit();
        let b = lo + (1.0 - 2.0 * lo - a) * self.unit();
        [a, b, 1.0 - a - b]
    }
}

/// The max-norm.
fn norm_inf(v: [f64; 2]) -> f64 {
    v[0].abs().max(v[1].abs())
}

/// The Euclidean norm.
fn norm_2(v: [f64; 2]) -> f64 {
    v[0].hypot(v[1])
}

const PAIRS: [(usize, usize); 3] = [(0, 1), (0, 2), (1, 2)];

/// §6's conditioning sum, written from its text, with `norm` the norm it takes.
fn definition(m: [f32; 3], r: [[f32; 2]; 3], norm: fn([f64; 2]) -> f64) -> f64 {
    let m = m.map(f64::from);
    let r = r.map(|p| p.map(f64::from));
    PAIRS
        .iter()
        .map(|&(i, j)| {
            let w = norm(r[i]) + norm(r[j]);
            let sep = norm([r[i][0] - r[j][0], r[i][1] - r[j][1]]);
            let s = sep.max(EPS * w.max(1.0));
            m[i] * m[j] * w / (s * s)
        })
        .sum()
}

/// The fuzzed configurations: masses on the simplex, positions on `[−2, 2]²`, and each with a pair made coincident,
/// moved under the floor, onto it and just over it.
fn configurations() -> Vec<([f32; 3], [[f32; 2]; 3])> {
    let mut rng = Rng(0x2029_0060);
    let mut out = Vec::new();
    for _ in 0..256 {
        let m = rng.masses(0.0).map(|x| x as f32);
        let r: [[f32; 2]; 3] = [0; 3].map(|_| [rng.sym(2.0) as f32, rng.sym(2.0) as f32]);
        out.push((m, r));
        let mut c = r;
        c[1] = c[0];
        out.push((m, c));
        for steps in [1u32, 8, 16, 24] {
            let mut n = r;
            n[2] = [n[0][0], f32::from_bits(n[0][1].to_bits() + steps)];
            out.push((m, n));
        }
    }
    out
}

/// `agreement::conditioning` and `agreement::ratio` against [`definition`] with `norm`.
fn check_definition(norm: fn([f64; 2]) -> f64) {
    for (m, r) in configurations() {
        let want = definition(m, r, norm);
        let got = agreement::conditioning(m, r);
        assert!(
            (got - want).abs() <= 1e-12 * want.max(f64::MIN_POSITIVE),
            "the conditioning of {m:?}, {r:?} is {got}, which is not the definition's {want}"
        );
        let (k, v, k0, v0) = (0.75f32, -2.5f32, 0.5f32, -2.0f32);
        let scale = 1.0 + 2.0 + want;
        let bound = f64::from(FACTOR) * EPS * scale;
        let diff = ((f64::from(k) + f64::from(v)) - (f64::from(k0) + f64::from(v0))).abs();
        let got = agreement::ratio(k, v, k0, v0, m, r).expect("finite operands");
        assert!(
            (got - diff / bound).abs() <= 1e-12 * (diff / bound),
            "the ratio {got} is not the definition's {}",
            diff / bound
        );
    }
    // Where every ŝ is the separation, the max-norm sum is within a factor 2 of the Euclidean sum of §6's text.
    let mut rng = Rng(0x0623);
    for _ in 0..4096 {
        let m = rng.masses(0.0).map(|x| x as f32);
        let r: [[f32; 2]; 3] = [0; 3].map(|_| [rng.sym(2.0) as f32, rng.sym(2.0) as f32]);
        let euclid: f64 = PAIRS
            .iter()
            .map(|&(i, j)| {
                let (a, b) = (r[i].map(f64::from), r[j].map(f64::from));
                let d = norm_2([a[0] - b[0], a[1] - b[1]]);
                f64::from(m[i]) * f64::from(m[j]) * (norm_2(a) + norm_2(b)) / (d * d)
            })
            .sum();
        let got = agreement::conditioning(m, r);
        assert!(
            got <= 2.0 * euclid && euclid <= 2.0 * got,
            "{m:?}, {r:?}: the term {got} is not within a factor 2 of the Euclidean {euclid}"
        );
    }
}

#[test]
fn qa_m229_ratio_is_the_definition() {
    check_definition(norm_inf);
}

negative_control!(
    qa_m229_ratio_is_the_definition,
    "the Euclidean form taken as the definition must be told apart from §6's max-norm",
    expected = "which is not the definition's",
    check_definition(norm_2)
);

/// One pair's term at f32 in each association, `floor` whether `ŝ` carries §6's floor; each must be finite and at
/// most `2⁴⁴`.
fn check_associations(floor: bool) {
    let mut rng = Rng(0x0436);
    let mut cases: Vec<([f32; 2], [f32; 2], f32)> = Vec::new();
    for _ in 0..4096 {
        let mm = {
            let m = rng.masses(0.0);
            (m[0] * m[1]) as f32
        };
        let scale = 2f64.powi((rng.unit() * 124.0) as i32 - 62);
        let a = [rng.sym(scale) as f32, rng.sym(scale) as f32];
        let b = [rng.sym(scale) as f32, rng.sym(scale) as f32];
        cases.push((a, b, mm));
        // Coincident, and one ulp apart.
        cases.push((a, a, mm));
        cases.push((a, [a[0], f32::from_bits(a[1].to_bits() ^ 1)], mm));
    }
    // The extremes §6 names: the largest mᵢmⱼ, 1/4, at w = 1, w = 0 and positions just below 2⁶².
    let big = f32::from_bits((2f32.powi(62)).to_bits() - 1);
    for p in [[0.5f32, 0.0], [0.0, 0.0], [big, -big], [-big, big]] {
        cases.push((p, p, 0.25));
    }
    cases.push(([big, big], [-big, -big], 0.25));
    for (a, b, mm) in cases {
        let n = |v: [f32; 2]| v[0].abs().max(v[1].abs());
        let w = n(a) + n(b);
        let sep = n([a[0] - b[0], a[1] - b[1]]);
        let s = if floor {
            sep.max(f32::EPSILON * w.max(1.0))
        } else {
            sep
        };
        for (form, t) in [
            ("(mm/s)·(w/s)", (mm / s) * (w / s)),
            ("mm·w/(s·s)", mm * w / (s * s)),
            ("(mm·w/s)/s", (mm * w / s) / s),
            ("mm·(w/(s·s))", mm * (w / (s * s))),
        ] {
            assert!(
                t.is_finite() && (0.0..=2f32.powi(44)).contains(&t),
                "{a:?}, {b:?}, mm = {mm}: the term {form} = {t} is not finite and within 2⁴⁴"
            );
        }
    }
}

#[test]
fn qa_m229_term_finite_in_every_association() {
    check_associations(true);
}

negative_control!(
    qa_m229_term_finite_in_every_association,
    "the term without the separation's floor must be caught non-finite at the pole",
    expected = "is not finite and within",
    check_associations(false)
);

/// `V = −Σ mᵢmⱼ/rᵢⱼ` at f64 from f64 positions.
fn potential64(m: [f64; 3], r: [[f64; 2]; 3]) -> f64 {
    -PAIRS
        .iter()
        .map(|&(i, j)| m[i] * m[j] / norm_2([r[i][0] - r[j][0], r[i][1] - r[j][1]]))
        .sum::<f64>()
}

/// `V` at f32 throughout, from f32 masses and positions.
fn potential32(m: [f32; 3], r: [[f32; 2]; 3]) -> f32 {
    -PAIRS
        .iter()
        .map(|&(i, j)| {
            let (dx, dy) = (r[i][0] - r[j][0], r[i][1] - r[j][1]);
            m[i] * m[j] / (dx * dx + dy * dy).sqrt()
        })
        .sum::<f32>()
}

/// The refinement sweep: halvings of the binary's separation from `w/2` to `4·eps·w`.
const HALVINGS: std::ops::RangeInclusive<i32> = 1..=21;

/// The first halving of the sweep's second half, where `κ = w/d ≥ 2¹¹` and the collision dominates the f32 error.
const DEEP: usize = 11;

/// Per halving, the worst factor over the fuzzed binaries, judged by `agreement::ratio`, §6's conditioning-aware scale,
/// where `conditioned`, else by the per-operand scale alone, `max(|K₀|, 1) + max(|V₀|, 1)`; asserts each sample agrees
/// (where `tolerance`) and the worst is flat.
fn check_refinement(conditioned: bool, tolerance: bool) {
    let form = if conditioned {
        "conditioned"
    } else {
        "per-operand"
    };
    let mut rng = Rng(0x2029_0623);
    let mut worst = vec![0f64; HALVINGS.count()];
    for _ in 0..512 {
        let m = rng.masses(0.05);
        let m32 = m.map(|x| x as f32);
        // Body 0 with ‖r₀‖∞ on [½, 1], as the decode's positions are of order 1 under I = 1: w ≥ 1, so the floor is
        // eps·w and the sweep stays above it.
        let r0 = {
            let big = (0.5 + 0.5 * rng.unit()) * if rng.unit() < 0.5 { -1.0 } else { 1.0 };
            let small = rng.sym(big.abs());
            if rng.unit() < 0.5 {
                [big, small]
            } else {
                [small, big]
            }
        };
        let r2 = [rng.sym(1.0), rng.sym(1.0)];
        let theta = std::f64::consts::TAU * rng.unit();
        let k = 0.1 + rng.unit();
        for (slot, h) in HALVINGS.enumerate() {
            // Body 1 at the separation d from body 0, d = w·2⁻ʰ, w its pair's max-norm sum (≈ 2‖r₀‖∞).
            let w = 2.0 * norm_inf(r0);
            let d = w * 2f64.powi(-h);
            let r = [r0, [r0[0] + d * theta.cos(), r0[1] + d * theta.sin()], r2];
            let r32 = r.map(|p| p.map(|x| x as f32));
            let v0 = potential64(m, r) as f32;
            let v = potential32(m32, r32);
            let k32 = k as f32;
            let factor = if conditioned {
                agreement::ratio(k32, v, k32, v0, m32, r32).expect("finite operands")
                    * f64::from(FACTOR)
            } else {
                let diff =
                    ((f64::from(k32) + f64::from(v)) - (f64::from(k32) + f64::from(v0))).abs();
                diff / (EPS * (f64::from(k32).abs().max(1.0) + f64::from(v0).abs().max(1.0)))
            };
            assert!(
                !tolerance || factor <= f64::from(FACTOR),
                "{form}: at separation w·2^-{h} the factor {factor} exceeds the tolerance's {FACTOR}"
            );
            worst[slot] = worst[slot].max(factor);
        }
    }
    println!("{form}: worst factor per halving {worst:.3?}");
    // Flat where the collision dominates, κ = w/d ≥ 2¹¹, the sweep's second half: over its ten halvings the worst
    // varies by less than a 1/d quantity grows in one.
    let deep = &worst[DEEP - 1..];
    let hi = deep.iter().copied().fold(0.0, f64::max);
    let lo = deep.iter().copied().fold(f64::INFINITY, f64::min);
    assert!(
        hi <= 2.0 * lo,
        "{form}: the worst factor is not flat under refinement, {lo} to {hi}: {worst:?}"
    );
}

#[test]
fn qa_m229_f32_energy_flat_under_refinement() {
    check_refinement(true, true);
}

negative_control!(
    qa_m229_f32_energy_flat_under_refinement,
    "the per-operand form must fail beside a collision",
    expected = "exceeds the tolerance",
    check_refinement(false, true)
);

/// The flatness check on its own, the tolerance not asserted: the per-operand form's worst grows as `1/d`.
#[test]
fn qa_m229_flatness_alone() {
    check_refinement(true, false);
}

negative_control!(
    qa_m229_flatness_alone,
    "the per-operand form's growth under refinement must be caught by the flatness check",
    expected = "is not flat under refinement",
    check_refinement(false, false)
);
