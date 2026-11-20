//! QA's tests for TASK-M3-03, written from the requirements it closes, not from the implementation:
//! - REQ-INT-091 (`qa_nsub_table_host_*`): each threshold the host generator emits, for every configuration of
//!   `fixtures/gates/nsub-table/tables.json` and for drawn sim keys, is the f32 nearest the exact
//!   `(r_sub·n^{−1/γ_sub})²`, decided by an oracle of this file's own in exact rational arithmetic (no f64, no libm, no
//!   Python): `γ_sub = a/b` in lowest terms, the exact threshold `x` is the positive root of `x^a = r_sub^{2a}/n^{2b}`, so
//!   an f32 `t` is its correct rounding iff the `a`-th powers of the midpoints either side of `t` bracket that rational.
//!   The fixture's own bits are checked against the same oracle, so the reference script is checked too. Where the
//!   generator's f64 path may legitimately round to the other side (R-502; the requirement's listed exceptions), only
//!   an exact value within a few f64 ulps of an f32 rounding tie is admitted. `N_max = L_max + 1`, `N_max = 0` and
//!   `u32::MAX` are refused with the typed error; `N_max = 1` and `N_max = L_max` are taken, and the table is the same
//!   for every accepted `N_max` (never truncated).
//! - REQ-INT-092 (`qa_nsub_default_table_bitwise`): at `r_sub = 0.05`, `γ_sub = 1.5`, `N_max = 64` the host generator
//!   reproduces the checked-in build-time table bit for bit, and that table is the oracle's.
//! - REQ-INT-028 (`qa_nsub_bucket_*`): `N_sub` equals the specification `clamp(⌈(r_sub/r_min)^{γ_sub}⌉, 1, N_max)`,
//!   evaluated exactly (`⌈y⌉ > n` iff `(r_sub²/d²)^a > n^{2b}`), for every `d²` sampled at and straddling every edge
//!   and drawn across the range, at the default sim key and at sim keys with `r_sub`, `γ_sub` and `N_max` each changed
//!   (a non-power-of-two `N_max` among them); the only `d²` where it may differ are the edges themselves, an f32
//!   threshold rounded below its exact edge, where the f32 rounding of `THR[n]` decides. The CPU-f64 and CPU-f32 legs
//!   take the same `d²`, bit for bit, so the same `N_sub` (0 forks); domain: identical inputs per macro-step
//!   (pitfalls §10), not a trajectory. Control: the default table under a changed `r_sub` or `γ_sub` fails the
//!   specification.
//! - REQ-INT-028 at positions under the square's floor (`qa_nsub_bucket_floor_pairs_*`): the specification is on the
//!   closest approach of the f32 positions, so a pair one of whose axis offsets the quantised `d²`'s floor sets to 0
//!   (an axis square below `2⁻⁸⁰`; dd_integrator §3.3, R-547) still takes the specification's `N_sub`, judged in exact
//!   arithmetic, at `2⁻¹⁹` either side of every deciding threshold of every sim key the generator accepts, across
//!   `r_sub`, `γ_sub` down to 1/4 and `N_max` up to `L_max`. A sim key it refuses is refused with the typed error naming a
//!   deciding threshold, never a truncated table (R-502). Control: a sim key's correctly rounded table whose deciding
//!   thresholds fall far below the floor's reach fails the specification at such a pair.
//! - REQ-INT-094 (`qa_nsub_table_max_bounds`): the refusal bound is the table's length, the table takes `4·L_max` bytes
//!   in 16-byte rows, and `horizon_steps × N_max ≤ 2³² − 1` holds at `L_max`.
//! - REQ-INT-027 (`qa_nsub_live_readout`), REQ-TOOL-033 (`qa_saturated_sticky`), REQ-TOOL-034
//!   (`qa_total_substeps_proxy`).

use num_bigint::BigInt;
use num_rational::BigRational;

use kernel::driver::nsub_table::{
    pack, threshold_bits, threshold_f64, thresholds, NsubConfig, NsubTableError,
};
use kernel::driver::substep::{d2_quantised, n_sub, NsubThresholds, SubstepState};
use kernel::generated::nsub_thresholds::{
    DEFAULT_THRESHOLD_BITS, GAMMA_SUB_DEFAULT, NSUB_TABLE_MAX, NSUB_TABLE_ROWS, N_MAX_DEFAULT,
    R_SUB_DEFAULT,
};
use kernel::payload::{sd_saturated, total_substeps_log2};
use validation::negative_control;

const L: usize = NSUB_TABLE_MAX;

fn cfg(r_sub: f64, gamma_sub: f64, n_max: u32) -> NsubConfig {
    NsubConfig {
        r_sub,
        gamma_sub,
        n_max,
    }
}

// ── Exact arithmetic: the oracle ────────────────────────────────────────────────────────────────────────────────

/// The exact value of a finite f64.
fn exact64(x: f64) -> BigRational {
    assert!(x.is_finite());
    let bits = x.to_bits();
    let neg = bits >> 63 == 1;
    let e = ((bits >> 52) & 0x7FF) as i32;
    let f = bits & ((1u64 << 52) - 1);
    let (m, p) = if e == 0 {
        (f, -1074)
    } else {
        (f | (1u64 << 52), e - 1075)
    };
    let m = BigInt::from(if neg { -(m as i128) } else { m as i128 });
    scale2(BigRational::from_integer(m), p)
}

/// The exact value of a finite f32.
fn exact32(x: f32) -> BigRational {
    exact64(f64::from(x))
}

/// `v · 2^p`.
fn scale2(v: BigRational, p: i32) -> BigRational {
    let two = BigInt::from(1) << p.unsigned_abs() as usize;
    if p >= 0 {
        v * BigRational::from_integer(two)
    } else {
        v / BigRational::from_integer(two)
    }
}

fn powr(v: &BigRational, k: u32) -> BigRational {
    let mut out = BigRational::from_integer(BigInt::from(1));
    for _ in 0..k {
        out *= v;
    }
    out
}

fn int(n: u64) -> BigRational {
    BigRational::from_integer(BigInt::from(n))
}

/// `γ_sub` as `a/b` in lowest terms, from its exact f64 value (small `a`, `b` only).
fn gamma_ratio(gamma_sub: f64) -> (u32, u32) {
    let g = exact64(gamma_sub);
    let (a, b) = (g.numer().clone(), g.denom().clone());
    let a: u32 = a.try_into().expect("γ_sub's numerator is small");
    let b: u32 = b.try_into().expect("γ_sub's denominator is small");
    assert!(a <= 12 && b <= 64, "γ_sub {gamma_sub} is not a small ratio");
    (a, b)
}

/// The exact substep law of a sim key, at its f64 inputs as the generator receives them.
struct Law {
    r2: BigRational,
    a: u32,
    b: u32,
    r_sub: f64,
    gamma_sub: f64,
}

impl Law {
    fn new(r_sub: f64, gamma_sub: f64) -> Law {
        let (a, b) = gamma_ratio(gamma_sub);
        let r = exact64(r_sub);
        Law {
            r2: &r * &r,
            a,
            b,
            r_sub,
            gamma_sub,
        }
    }

    /// `R_n = x_n^a`, `x_n = (r_sub·n^{−1/γ_sub})²` the exact edge: `r_sub^{2a}/n^{2b}`.
    fn edge_pow(&self, n: u32) -> BigRational {
        powr(&self.r2, self.a) / powr(&int(u64::from(n)), 2 * self.b)
    }

    /// The f32 nearest the exact edge `n` (ties to even), searched from a rough start by bracketing the root with the
    /// `a`-th powers of the f32 midpoints.
    fn correct_bits(&self, n: u32) -> u32 {
        let target = self.edge_pow(n);
        let rough = (self.r_sub * f64::from(n).powf(-1.0 / self.gamma_sub)).powi(2) as f32;
        let mut c = rough.to_bits();
        for _ in 0..64 {
            let lo = powr(&mid(c - 1, c), self.a);
            let hi = powr(&mid(c, c + 1), self.a);
            if target < lo {
                c -= 1;
            } else if target > hi {
                c += 1;
            } else if target == lo {
                return if c.is_multiple_of(2) { c } else { c - 1 };
            } else if target == hi {
                return if c.is_multiple_of(2) { c } else { c + 1 };
            } else {
                return c;
            }
        }
        panic!("the oracle did not converge at n = {n}");
    }

    /// Whether the exact edge `n` is within `ulps64` f64 ulps (relative `ulps64·2⁻⁵²`) of the f32 rounding tie between
    /// `c` and its neighbour `other`: where an f64 evaluation off by that much may round to either f32.
    fn near_tie(&self, n: u32, c: u32, other: u32, ulps64: u32) -> bool {
        let m = mid(c.min(other), c.max(other));
        let eps = BigRational::new(BigInt::from(ulps64), BigInt::from(1u64 << 52));
        let one = int(1);
        let lo = powr(&(&m * (&one - &eps)), self.a);
        let hi = powr(&(&m * (&one + &eps)), self.a);
        let t = self.edge_pow(n);
        lo <= t && t <= hi
    }

    /// The specification at the f32 `d2 > 0`: `clamp(⌈(r_sub/√d²)^{γ_sub}⌉, 1, n_max)`, exactly. With
    /// `y = (r_sub²/d²)^{a/(2b)}`, `⌈y⌉ = 1 + #{n ≥ 1 : y > n}` and `y > n` iff `(r_sub²/d²)^a > n^{2b}`, monotone in `n`.
    fn spec(&self, d2: f32, n_max: u32) -> u32 {
        if d2 == 0.0 {
            return n_max;
        }
        if d2.is_infinite() {
            return 1;
        }
        self.spec_at(&exact32(d2), n_max)
    }

    /// The specification at the exact `d2 > 0`.
    fn spec_at(&self, d2: &BigRational, n_max: u32) -> u32 {
        let ya = powr(&(&self.r2 / d2), self.a);
        let above = |n: u32| ya > powr(&int(u64::from(n)), 2 * self.b);
        // The largest k in 0 … n_max − 1 with y > k (k = 0 always holds), by bisection.
        let (mut lo, mut hi) = (0u32, n_max - 1);
        while lo < hi {
            let m = lo + (hi - lo).div_ceil(2);
            if above(m) {
                lo = m;
            } else {
                hi = m - 1;
            }
        }
        (lo + 1).min(n_max)
    }

    /// Whether the f32 `t` lies below the exact edge `n`.
    fn below_edge(&self, t: f32, n: u32) -> bool {
        powr(&exact32(t), self.a) < self.edge_pow(n)
    }
}

/// The exact midpoint of the f32s with bits `x` and `y`.
fn mid(x: u32, y: u32) -> BigRational {
    (exact32(f32::from_bits(x)) + exact32(f32::from_bits(y))) / int(2)
}

// ── REQ-INT-091: the host generator against the exact oracle and the fixture ───────────────────────────────────

struct FixtureConfig {
    config: NsubConfig,
    bits: Vec<u32>,
    listed: Vec<usize>,
}

fn fixture() -> Vec<FixtureConfig> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/gates/nsub-table/tables.json"
    );
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("read the fixture"))
            .expect("the fixture is JSON");
    assert_eq!(v["l_max"].as_u64(), Some(L as u64), "the fixture's l_max");
    let hex = |s: &serde_json::Value| {
        u32::from_str_radix(
            s.as_str().expect("a hex string").trim_start_matches("0x"),
            16,
        )
        .expect("hex")
    };
    v["configs"]
        .as_array()
        .expect("configs")
        .iter()
        .map(|c| FixtureConfig {
            config: cfg(
                c["r_sub"].as_f64().expect("r_sub"),
                c["gamma_sub"].as_f64().expect("gamma_sub"),
                c["n_max"].as_u64().expect("n_max") as u32,
            ),
            bits: c["bits"]
                .as_array()
                .expect("bits")
                .iter()
                .map(hex)
                .collect(),
            listed: c["exceptions"]
                .as_array()
                .expect("exceptions")
                .iter()
                .map(|e| e["n"].as_u64().expect("n") as usize)
                .collect(),
        })
        .collect()
}

/// The f64 ulps of the generator's admitted error band: an f64 evaluation of `(r_sub·n^{−1/γ})²` in four rounded
/// steps errs by at most `ln n/γ + 3.5` f64 ulps (the rounded exponent amplified by `ln n/γ`, `pow`'s sub-ulp error, the
/// product, the square, its rounding); at `n = L_max = 256` and the least `γ` drawn here, 0.5, that is 14.6, doubled
/// and rounded up to 32. Only an entry this close to an f32 tie may round to the other side (R-502).
const TIE_BAND_F64_ULPS: u32 = 32;

/// `bits` (THR[1..=L]) are, entry for entry, the oracle's correctly rounded bits; an entry in `listed` may instead be
/// the other neighbour, and only where the exact value is within the tie band.
fn check_against_oracle(what: &str, c: &NsubConfig, bits: &[u32], listed: &[usize]) {
    assert_eq!(bits.len(), L, "{what}: {} entries, not L_max", bits.len());
    let law = Law::new(c.r_sub, c.gamma_sub);
    for (i, &got) in bits.iter().enumerate() {
        let n = i as u32 + 1;
        let want = law.correct_bits(n);
        if got == want {
            continue;
        }
        let neighbour = got == want + 1 || got == want - 1;
        assert!(
            listed.contains(&(i + 1)) && neighbour && law.near_tie(n, want, got, TIE_BAND_F64_ULPS),
            "{what}: THR[{n}] at r_sub {} γ_sub {} is {got:#010X}, not the correctly rounded {want:#010X}",
            c.r_sub,
            c.gamma_sub
        );
    }
}

/// The fixture's bits and the generator's both match the oracle for every configuration of the fixture, and the
/// fixture holds the default and `r_sub`, `γ_sub` and `N_max` each changed, a non-power-of-two `N_max` among them.
fn check_fixture(configs: &[FixtureConfig]) {
    let d = NsubConfig::DEFAULT;
    let has = |p: &dyn Fn(&NsubConfig) -> bool| configs.iter().any(|f| p(&f.config));
    assert!(has(&|c| *c == d), "the fixture lacks the default sim key");
    assert!(
        has(&|c| c.r_sub != d.r_sub && c.gamma_sub == d.gamma_sub && c.n_max == d.n_max),
        "the fixture lacks r_sub changed alone"
    );
    assert!(
        has(&|c| c.r_sub == d.r_sub && c.gamma_sub != d.gamma_sub && c.n_max == d.n_max),
        "the fixture lacks gamma_sub changed alone"
    );
    assert!(
        has(&|c| c.r_sub == d.r_sub
            && c.gamma_sub == d.gamma_sub
            && c.n_max != d.n_max
            && !c.n_max.is_power_of_two()),
        "the fixture lacks a non-power-of-two N_max changed alone"
    );
    for f in configs {
        check_against_oracle("the fixture", &f.config, &f.bits, &f.listed);
        let got = threshold_bits(&f.config).expect("a fixture configuration generates");
        check_against_oracle("the generator", &f.config, &got, &f.listed);
        // A listed entry is still the generator's own rounding of an f64 value: one of the two neighbours.
        for (i, (&g, &w)) in got.iter().zip(&f.bits).enumerate() {
            if !f.listed.contains(&(i + 1)) {
                assert_eq!(g, w, "the generator's THR[{}] is not the fixture's", i + 1);
            }
        }
    }
}

#[test]
fn qa_nsub_table_host_correctly_rounded() {
    check_fixture(&fixture());
}

negative_control!(
    qa_nsub_table_host_correctly_rounded,
    "a fixture entry one ulp off fails the oracle (REQ-INT-091's control)",
    expected = "not the correctly rounded",
    {
        let mut f = fixture();
        f[1].bits[37] += 1;
        check_fixture(&f);
    }
);

/// Drawn sim keys beyond the fixture: `r_sub` across four decades and `γ_sub = a/b`, small ratios around the default.
fn check_drawn(generate: fn(&NsubConfig) -> Result<[u32; L], NsubTableError>) {
    let gammas = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 2.5, 3.0, 4.0];
    let mut s = 0x9E37_79B9_7F4A_7C15u64;
    for k in 0..24 {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let u = (s >> 11) as f64 / (1u64 << 53) as f64;
        let r_sub = 10f64.powf(-3.0 + 3.0 * u);
        let gamma_sub = gammas[k % gammas.len()];
        let c = cfg(r_sub, gamma_sub, 64);
        let Ok(bits) = generate(&c) else {
            // A sim key whose deciding thresholds the square's floor could reach may be refused; none drawn here is.
            panic!("the drawn sim key r_sub {r_sub} γ_sub {gamma_sub} is refused");
        };
        // Any entry off the correct rounding must be at a tie; admit it as listed and let the band decide.
        let all: Vec<usize> = (1..=L).collect();
        check_against_oracle("the generator (drawn)", &c, &bits, &all);
    }
}

#[test]
fn qa_nsub_table_host_drawn_sim_keys() {
    check_drawn(threshold_bits);
}

negative_control!(
    qa_nsub_table_host_drawn_sim_keys,
    "a generator through a single-precision pow, rounding before the square, is not correctly rounded",
    expected = "not the correctly rounded",
    {
        fn f32_path(c: &NsubConfig) -> Result<[u32; L], NsubTableError> {
            let mut out = [0u32; L];
            for (i, b) in out.iter_mut().enumerate() {
                let s = (c.r_sub as f32) * ((i + 1) as f32).powf(-1.0 / c.gamma_sub as f32);
                *b = (s * s).to_bits();
            }
            Ok(out)
        }
        check_drawn(f32_path);
    }
);

fn check_refusals(at: u32, want: Result<(), NsubTableError>) {
    let got = threshold_bits(&cfg(R_SUB_DEFAULT, GAMMA_SUB_DEFAULT, at)).map(|_| ());
    assert_eq!(got, want, "N_max {at}: {got:?}, wanted {want:?}");
}

#[test]
fn qa_nsub_table_host_refusals() {
    let l = L as u32;
    check_refusals(0, Err(NsubTableError::NMaxBelowOne));
    check_refusals(l + 1, Err(NsubTableError::NMaxAboveTable { n_max: l + 1 }));
    check_refusals(
        u32::MAX,
        Err(NsubTableError::NMaxAboveTable { n_max: u32::MAX }),
    );
    check_refusals(1, Ok(()));
    check_refusals(l, Ok(()));
    // Never truncated: every accepted N_max gets the whole table, the same one.
    let full = threshold_bits(&cfg(R_SUB_DEFAULT, GAMMA_SUB_DEFAULT, l)).expect("L_max is taken");
    for n_max in [1, 2, 7, 50, 64, 100, 255] {
        let t = threshold_bits(&cfg(R_SUB_DEFAULT, GAMMA_SUB_DEFAULT, n_max)).expect("taken");
        assert_eq!(
            t, full,
            "the table at N_max {n_max} is not the table at L_max"
        );
    }
}

negative_control!(
    qa_nsub_table_host_refusals,
    "N_max = L_max is not refused, so expecting its refusal fails",
    expected = "wanted Err(NMaxAboveTable",
    check_refusals(
        NSUB_TABLE_MAX as u32,
        Err(NsubTableError::NMaxAboveTable {
            n_max: NSUB_TABLE_MAX as u32
        })
    )
);

// ── REQ-INT-092: the default sim key reproduces the build-time table ────────────────────────────────────────────

fn check_default(build_time: &[u32; L]) {
    assert_eq!(
        (R_SUB_DEFAULT, GAMMA_SUB_DEFAULT, N_MAX_DEFAULT),
        (0.05, 1.5, 64),
        "the default sim key"
    );
    let host = threshold_bits(&cfg(0.05, 1.5, 64)).expect("the default generates");
    for (i, (h, b)) in host.iter().zip(build_time).enumerate() {
        assert_eq!(
            h,
            b,
            "THR[{}]: the host {h:#010X}, the build-time table {b:#010X}",
            i + 1
        );
    }
    check_against_oracle(
        "the build-time table",
        &NsubConfig::DEFAULT,
        build_time,
        &[],
    );
}

#[test]
fn qa_nsub_default_table_bitwise() {
    check_default(&DEFAULT_THRESHOLD_BITS);
}

negative_control!(
    qa_nsub_default_table_bitwise,
    "the build-time table with one bit flipped in its first 64 entries fails (REQ-INT-092's control)",
    expected = "the build-time table 0x",
    {
        let mut t = DEFAULT_THRESHOLD_BITS;
        t[41] ^= 1 << 3;
        check_default(&t);
    }
);

// ── REQ-INT-028: the tree against the exact specification; the two legs ────────────────────────────────────────

/// The sim keys: the default, and `r_sub`, `γ_sub` and `N_max` each changed (50 and 7 not powers of two), and all
/// three changed.
fn sim_keys() -> Vec<NsubConfig> {
    vec![
        cfg(0.05, 1.5, 64),
        cfg(0.08, 1.5, 64),
        cfg(0.05, 2.0, 64),
        cfg(0.05, 1.5, 50),
        cfg(0.05, 1.5, NSUB_TABLE_MAX as u32),
        cfg(0.02, 1.0, 100),
        cfg(0.1, 3.0, 7),
        cfg(0.3, 1.25, 33),
    ]
}

/// `d²` at and either side (±1, ±2 ulps) of every threshold of `bits`, plus 0, `+∞` and a log-uniform spread from a
/// sixteenth of the least threshold to sixteen times the greatest.
fn samples(bits: &[u32; L]) -> Vec<f32> {
    let mut out = vec![0.0, f32::INFINITY];
    for &b in bits {
        for d in [-2i64, -1, 0, 1, 2] {
            out.push(f32::from_bits((i64::from(b) + d) as u32));
        }
    }
    let lo = (f64::from(f32::from_bits(bits[L - 1])) / 16.0).log2();
    let hi = (f64::from(f32::from_bits(bits[0])) * 16.0).log2();
    for i in 0..512 {
        out.push((lo + (hi - lo) * (f64::from(i) + 0.5) / 512.0).exp2() as f32);
    }
    out
}

/// At every `d2`, the tree over `table` (generated from `table_key`) under the sim key `key` equals the exact
/// specification of `key`, except at an f32 threshold below its exact edge, the one `d²` where the threshold's
/// rounding decides.
fn check_spec(key: &NsubConfig, table: &NsubThresholds, bits: &[u32; L], d2s: &[f32]) {
    let law = Law::new(key.r_sub, key.gamma_sub);
    for &d2 in d2s {
        let got = n_sub(d2, table, key.n_max);
        let want = law.spec(d2, key.n_max);
        if got == want {
            continue;
        }
        let edge = bits
            .iter()
            .take(key.n_max as usize - 1)
            .position(|&b| b == d2.to_bits())
            .map(|i| i as u32 + 1);
        let excused = edge.is_some_and(|n| law.below_edge(d2, n) && got == n && want == n + 1);
        assert!(
            excused,
            "r_sub {} γ_sub {} N_max {}: at d² {d2:e} N_sub is {got}, the specification {want}",
            key.r_sub, key.gamma_sub, key.n_max
        );
    }
}

#[test]
fn qa_nsub_bucket_matches_the_exact_specification() {
    for key in sim_keys() {
        let bits = threshold_bits(&key).expect("the sim key generates");
        let table = thresholds(&key).expect("the sim key generates");
        check_spec(&key, &table, &bits, &samples(&bits));
        validation::prop::run(&proptest::num::f32::POSITIVE, |d2| {
            check_spec(&key, &table, &bits, &[d2]);
            Ok(())
        });
    }
}

negative_control!(
    qa_nsub_bucket_matches_the_exact_specification,
    "the default table used under r_sub 0.08 fails the specification (REQ-INT-028's control)",
    expected = "the specification",
    {
        let key = cfg(0.08, 1.5, 64);
        let bits = DEFAULT_THRESHOLD_BITS;
        let table = thresholds(&NsubConfig::DEFAULT).expect("default");
        check_spec(&key, &table, &bits, &samples(&bits));
    }
);

mod gamma_control {
    use super::*;
    negative_control!(
        qa_nsub_bucket_matches_the_exact_specification,
        "the default table used under gamma_sub 2 fails the specification (REQ-INT-028's control)",
        expected = "the specification",
        {
            let key = cfg(0.05, 2.0, 64);
            let bits = DEFAULT_THRESHOLD_BITS;
            let table = thresholds(&NsubConfig::DEFAULT).expect("default");
            check_spec(&key, &table, &bits, &samples(&bits));
        }
    );
}

/// The relative error the position-quantised `d²` may carry against the exact squared separation of the f32
/// positions: one subtraction, one square, one addition, each rounded once (dd_integrator §3.3), `≤ 4·2⁻²⁴` to first
/// order; doubled for the second-order terms.
const D2_REL: f64 = 8.0 / (1u64 << 24) as f64;

/// The positions `r` (f64) and their f32 casts give the same `d²` bits and so the same `N_sub` on the two legs; the
/// `d²` is the closest pair's (to [`D2_REL`]), and the same under any order of the bodies.
fn check_legs(
    leg64: fn(&[[f64; 2]; 3]) -> f32,
    r: &[[f64; 2]; 3],
    table: &NsubThresholds,
    n_max: u32,
) {
    let r32 = r.map(|p| p.map(|x| x as f32));
    let a = leg64(r);
    let b = d2_quantised::<f32>(&r32);
    assert_eq!(
        a.to_bits(),
        b.to_bits(),
        "fork: d² f64 leg {a:e}, f32 leg {b:e} at {r:?}"
    );
    assert_eq!(
        n_sub(a, table, n_max),
        n_sub(b, table, n_max),
        "fork in N_sub at {r:?}"
    );
    let q = r32.map(|p| p.map(exact32));
    let pair = |i: usize, j: usize| {
        let dx = &q[i][0] - &q[j][0];
        let dy = &q[i][1] - &q[j][1];
        &dx * &dx + &dy * &dy
    };
    let exact = [pair(0, 1), pair(1, 2), pair(2, 0)]
        .into_iter()
        .min()
        .expect("three pairs");
    let diff = (exact32(a) - &exact) / &exact;
    let rel = if diff < int(0) { -diff } else { diff };
    let bound = exact64(D2_REL);
    assert!(rel <= bound, "d² {a:e} is not the closest pair's at {r:?}");
    for p in [[1, 0, 2], [2, 1, 0], [0, 2, 1], [1, 2, 0], [2, 0, 1]] {
        let s = [r32[p[0]], r32[p[1]], r32[p[2]]];
        assert_eq!(
            d2_quantised::<f32>(&s).to_bits(),
            b.to_bits(),
            "d² depends on the bodies' order at {r:?}"
        );
    }
}

/// Position triples whose closest pair straddles every threshold of the default table: the pair `√THR[n]·(1 + δ)`
/// apart for `δ` in a few f32 ulps either side, at drawn origins and angles, the closest pair cycled among the three.
fn straddling_triples(bits: &[u32; L]) -> Vec<[[f64; 2]; 3]> {
    let mut out = Vec::new();
    let mut s = 0x2545_F491_4F6C_DD1Du64;
    let mut unit = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    for (i, &b) in bits.iter().enumerate() {
        for d in [-3.0, -1.0, 0.0, 1.0, 3.0] {
            let sep =
                f64::from(f32::from_bits(b)).sqrt() * (1.0 + d * f64::EPSILON * 2f64.powi(29));
            let (x0, y0, th) = (
                2.0 * unit() - 1.0,
                2.0 * unit() - 1.0,
                std::f64::consts::TAU * unit(),
            );
            let p = [x0, y0];
            let q = [x0 + sep * th.cos(), y0 + sep * th.sin()];
            let far = [x0 + 3.0, y0 - 2.0];
            out.push(match i % 3 {
                0 => [p, q, far],
                1 => [far, p, q],
                _ => [q, far, p],
            });
        }
    }
    out
}

#[test]
fn qa_nsub_bucket_two_legs_no_fork() {
    for key in sim_keys() {
        let bits = threshold_bits(&key).expect("generates");
        let table = thresholds(&key).expect("generates");
        for r in straddling_triples(&bits) {
            check_legs(d2_quantised::<f64>, &r, &table, key.n_max);
        }
    }
}

negative_control!(
    qa_nsub_bucket_two_legs_no_fork,
    "an f64 leg that forms d² from the f64 positions and casts it last, not from f32 positions, forks from the f32 leg",
    expected = "fork: d²",
    {
        fn unquantised(r: &[[f64; 2]; 3]) -> f32 {
            let d = |a: [f64; 2], b: [f64; 2]| (b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2);
            d(r[1], r[2]).min(d(r[2], r[0])).min(d(r[0], r[1])) as f32
        }
        let table = thresholds(&NsubConfig::DEFAULT).expect("default");
        for r in straddling_triples(&DEFAULT_THRESHOLD_BITS) {
            check_legs(unquantised, &r, &table, 64);
        }
    }
);

// ── REQ-INT-028: pairs under the square's floor ─────────────────────────────────────────────────────────────────

/// The relative band about an exact edge within which a pair's `N_sub` is not judged: twice the roundings a correct
/// tree may carry against the exact comparison, `u = 2⁻²⁴` f32's unit roundoff: `THR[n]`'s rounding (`u`), the
/// quantised `d²`'s (the subtractions from a body at the origin exact; each split-product square `3u`, R-547, and the
/// add `u`: `7u` at most), `8u` in all, doubled.
const FLOOR_BAND: f64 = 1.0 / (1u64 << 20) as f64;

/// Each pair's exact `d²` sits this far (relative) either side of the deciding threshold it straddles: outside
/// [`FLOOR_BAND`] of the exact edge, which is within `u` of the f32 threshold.
const FLOOR_OFFSET: f64 = 1.0 / (1u64 << 19) as f64;

/// Axis offsets whose square is below `2⁻⁸⁰`, which the quantised `d²` sets to 0 (dd_integrator §3.3, R-547): the f32
/// just under `2⁻⁴⁰` and `2⁻⁴¹`.
const FLOOR_DXS: [f32; 2] = [f32::from_bits(0x2B7F_FFFF), f32::from_bits(0x2B00_0000)];

/// The position triples, as f32, whose closest pair is a floor offset along one axis and, along the other, the rest of
/// `THR[n]·(1 ± FLOOR_OFFSET)`, for every deciding threshold `n < N_max`, with the pair cycled among the three bodies
/// and the axes swapped; the third body is 10 away. One body of the pair is at the origin, so each subtraction is exact.
fn floor_triples(bits: &[u32; L], n_max: u32) -> Vec<(u32, [[f32; 2]; 3])> {
    let mut out = Vec::new();
    for n in 1..n_max {
        let thr = f64::from(f32::from_bits(bits[n as usize - 1]));
        for side in [-1.0, 1.0] {
            let target = thr * (1.0 + side * FLOOR_OFFSET);
            for (k, &dx) in FLOOR_DXS.iter().enumerate() {
                let dy = (target - f64::from(dx).powi(2)).sqrt() as f32;
                for swap in [false, true] {
                    let o = [0.0f32, 0.0];
                    let p = if swap { [dy, dx] } else { [dx, dy] };
                    let far = [10.0f32, 10.0];
                    let r = match (n as usize + k + usize::from(swap)) % 3 {
                        0 => [o, p, far],
                        1 => [far, o, p],
                        _ => [p, far, o],
                    };
                    out.push((n, r));
                }
            }
        }
    }
    out
}

/// At every floor triple of `bits`, the tree over `table` under `key` takes the exact specification's `N_sub` at the
/// closest pair's exact `d²`, and the f64 leg the f32 leg's `d²`, bit for bit.
fn check_floor(key: &NsubConfig, table: &NsubThresholds, bits: &[u32; L]) {
    let law = Law::new(key.r_sub, key.gamma_sub);
    let band = exact64(FLOOR_BAND);
    let one = int(1);
    for (n, r) in floor_triples(bits, key.n_max) {
        let d2 = d2_quantised::<f32>(&r);
        let d2_64 = d2_quantised::<f64>(&r.map(|p| p.map(f64::from)));
        assert_eq!(d2.to_bits(), d2_64.to_bits(), "fork: d² at {r:?}");
        let q = r.map(|p| p.map(exact32));
        let pair = |i: usize, j: usize| {
            let dx = &q[i][0] - &q[j][0];
            let dy = &q[i][1] - &q[j][1];
            &dx * &dx + &dy * &dy
        };
        let exact = [pair(0, 1), pair(1, 2), pair(2, 0)]
            .into_iter()
            .min()
            .expect("three pairs");
        // The pair is outside the band of every exact edge near the threshold it straddles (the edges are far apart).
        let lo = powr(&(&exact * (&one - &band)), law.a);
        let hi = powr(&(&exact * (&one + &band)), law.a);
        for m in n.saturating_sub(1).max(1)..=(n + 1).min(key.n_max) {
            let e = law.edge_pow(m);
            assert!(
                !(lo <= e && e <= hi),
                "the floor pair at THR[{n}] is within the band of edge {m}: {r:?}"
            );
        }
        let got = n_sub(d2, table, key.n_max);
        let want = law.spec_at(&exact, key.n_max);
        assert_eq!(
            got, want,
            "r_sub {} γ_sub {} N_max {}: the floor pair at THR[{n}] {r:?} takes N_sub {got}, the specification {want}",
            key.r_sub, key.gamma_sub, key.n_max
        );
    }
}

/// Sim keys from the default's neighbourhood down to deciding thresholds near the floor's reach: `r_sub` over three
/// decades, `γ_sub` from 1/4 to the default, `N_max` from 7 to `L_max`.
fn floor_keys() -> Vec<NsubConfig> {
    let mut out = Vec::new();
    for r_sub in [0.001, 0.05, 0.3] {
        for gamma_sub in [0.25, 0.5, 1.0, 1.5] {
            for n_max in [7, 56, 64, L as u32] {
                out.push(cfg(r_sub, gamma_sub, n_max));
            }
        }
    }
    out
}

#[test]
fn qa_nsub_bucket_floor_pairs_match_the_specification() {
    let (mut taken, mut refused) = (0, 0);
    let mut least = f32::INFINITY;
    for key in floor_keys() {
        match threshold_bits(&key) {
            Ok(bits) => {
                check_floor(&key, &pack(&bits), &bits);
                taken += 1;
                if key.n_max >= 2 {
                    least = least.min(f32::from_bits(bits[key.n_max as usize - 2]));
                }
            }
            Err(NsubTableError::OutOfRange { n, bits }) => {
                // Refused, never truncated: the error names a deciding threshold and its generated bits.
                assert!(
                    (1..key.n_max).contains(&n),
                    "{key:?} is refused at THR[{n}], which does not decide N_sub"
                );
                let thr = threshold_f64(key.r_sub, key.gamma_sub, n) as f32;
                assert_eq!(
                    bits,
                    thr.to_bits(),
                    "{key:?}: the refusal's bits are not THR[{n}]'s"
                );
                refused += 1;
            }
            Err(e) => panic!("{key:?} is refused with {e:?}"),
        }
    }
    // The sweep reaches both sides: keys taken with a deciding threshold within 2⁸ of the floor's 2⁻⁸⁰ reach … 2⁻⁵⁰,
    // and keys refused.
    assert!(taken > 0 && refused > 0, "taken {taken}, refused {refused}");
    assert!(
        least < 2f32.powi(-50),
        "no taken sim key decides near the floor: least deciding threshold {least:e}"
    );
}

negative_control!(
    qa_nsub_bucket_floor_pairs_match_the_specification,
    "r_sub 0.05 γ_sub 1/4 at N_max L_max, its correctly rounded table taken unchecked: thresholds near 2⁻⁷² let the \
     floor alone decide N_sub, against the specification",
    expected = "the specification",
    {
        let key = cfg(0.05, 0.25, L as u32);
        let law = Law::new(key.r_sub, key.gamma_sub);
        let mut bits = [0u32; L];
        for (i, b) in bits.iter_mut().enumerate() {
            *b = law.correct_bits(i as u32 + 1);
        }
        check_floor(&key, &pack(&bits), &bits);
    }
);

// ── REQ-INT-094: the maximum length's bounds ────────────────────────────────────────────────────────────────────

/// `horizon_steps` at most, from the integrator contract Part 5 (`t_end_step` an exact u16).
const HORIZON_STEPS: u64 = 65_535;

fn check_bounds(l_max: usize, rows: usize, uniform_bytes: usize) {
    assert!(
        HORIZON_STEPS * l_max as u64 <= u64::from(u32::MAX),
        "horizon_steps × L_max = {} exceeds 2³² − 1",
        HORIZON_STEPS * l_max as u64
    );
    assert_eq!(
        rows,
        l_max.div_ceil(4),
        "the table's rows are not ⌈L_max/4⌉"
    );
    assert_eq!(
        uniform_bytes,
        16 * rows,
        "the table is not 16-byte rows of four u32"
    );
    assert_eq!(
        uniform_bytes,
        4 * l_max,
        "the table does not take 4·L_max bytes"
    );
}

#[test]
fn qa_nsub_table_max_bounds() {
    check_bounds(L, NSUB_TABLE_ROWS, std::mem::size_of::<NsubThresholds>());
    // The largest N_max taken is L_max, and its run's total fits the u32 exactly.
    let table =
        thresholds(&cfg(R_SUB_DEFAULT, GAMMA_SUB_DEFAULT, L as u32)).expect("L_max is taken");
    let mut s = SubstepState::default();
    for _ in 0..HORIZON_STEPS {
        s.macro_step(0.0, &table, L as u32);
    }
    assert_eq!(
        u64::from(s.total_substeps),
        HORIZON_STEPS * L as u64,
        "the exact total at horizon_steps × L_max"
    );
}

negative_control!(
    qa_nsub_table_max_bounds,
    "an L_max of 65538 breaks horizon_steps × N_max ≤ 2³² − 1",
    expected = "exceeds 2³² − 1",
    check_bounds(65_538, 65_538 / 4 + 1, 4 * 65_538)
);

// ── REQ-INT-027: the live readout ───────────────────────────────────────────────────────────────────────────────

/// A walk of `d²` down through the table and back, away from the edges (geometric midpoints between thresholds).
fn walk(bits: &[u32; L], n_max: u32) -> Vec<f32> {
    let top = (n_max as usize + 2).min(L);
    let mut down: Vec<f32> = (1..top)
        .map(|i| (f32::from_bits(bits[i - 1]) * f32::from_bits(bits[i])).sqrt())
        .collect();
    down.insert(0, f32::from_bits(bits[0]) * 4.0);
    down.push(f32::from_bits(bits[L - 1]) / 4.0);
    let up: Vec<f32> = down.iter().rev().copied().collect();
    let mut out = down;
    out.extend(up.into_iter().step_by(3));
    out
}

/// After each macro-step the live readout is the `N_sub` that macro-step took, which is the specification's at its
/// `d²`; `readout` reads it.
fn check_readout(key: &NsubConfig, readout: fn(&SubstepState, u32) -> u32) {
    let bits = threshold_bits(key).expect("generates");
    let table = thresholds(key).expect("generates");
    let law = Law::new(key.r_sub, key.gamma_sub);
    let mut s = SubstepState::default();
    let mut previous = 0;
    for d2 in walk(&bits, key.n_max) {
        let used = s.macro_step(d2, &table, key.n_max);
        let want = law.spec(d2, key.n_max);
        assert_eq!(
            used, want,
            "the macro-step at d² {d2:e} took {used}, the specification {want}"
        );
        let live = readout(&s, previous);
        assert_eq!(
            live, used,
            "the live readout {live} is not the {used} the macro-step at d² {d2:e} used"
        );
        previous = used;
    }
}

#[test]
fn qa_nsub_live_readout() {
    for key in sim_keys() {
        check_readout(&key, |s, _| s.live_n_sub);
    }
}

negative_control!(
    qa_nsub_live_readout,
    "a readout one macro-step behind is not the value the macro-step used",
    expected = "the live readout",
    check_readout(&NsubConfig::DEFAULT, |_, previous| previous)
);

// ── REQ-TOOL-033: the sticky saturated bit ──────────────────────────────────────────────────────────────────────

/// The run over `d2s`: after each step, the flag as `flag` reads it, against whether `N_sub == N_max` has occurred,
/// `N_sub` counted from the table; the descriptor's bit 5 carries it and keeps the other bits.
fn check_sticky(key: &NsubConfig, d2s: &[f32], flag: fn(&SubstepState, u32, u32) -> bool) {
    let bits = threshold_bits(key).expect("generates");
    let table = thresholds(key).expect("generates");
    let mut s = SubstepState::default();
    let mut occurred = false;
    for &d2 in d2s {
        let n = s.macro_step(d2, &table, key.n_max);
        // N_sub from the table by counting, `min(1 + #{m : d² < THR[m]}, N_max)`, apart from the tree.
        let counted =
            (1 + bits.iter().filter(|&&b| d2 < f32::from_bits(b)).count() as u32).min(key.n_max);
        occurred |= counted == key.n_max;
        let got = flag(&s, n, key.n_max);
        assert_eq!(
            got, occurred,
            "N_max {}: after d² {d2:e} (N_sub {n}) saturated is {got}, N_sub == N_max occurred: {occurred}",
            key.n_max
        );
        let word = s.packed_a(0xA5A5_A5A5 & !(1 << 5));
        assert_eq!(sd_saturated(word), occurred, "descriptor bit 5");
        assert_eq!(
            word & !(1 << 5),
            0xA5A5_A5A5 & !(1 << 5),
            "the other descriptor bits"
        );
    }
}

/// `d²` for `N_sub = N_max − 1` (at `THR[N_max − 1]` exactly), then `N_max` (one ulp below it), then a power-of-two
/// bucket below `N_max`, then far past the cap, then relaxed to 1.
fn cap_sequence(key: &NsubConfig) -> Vec<f32> {
    let bits = threshold_bits(key).expect("generates");
    let m = key.n_max as usize;
    let mut v = vec![f32::from_bits(bits[0]) * 4.0];
    if m >= 2 {
        let p2 = 1usize << (usize::BITS - 1 - (m - 1).leading_zeros());
        if p2 >= 2 && p2 < m {
            v.push(f32::from_bits(bits[p2 - 1]));
        }
        v.push(f32::from_bits(bits[m - 2]));
        v.push(f32::from_bits(bits[m - 2] - 1));
    }
    v.extend([
        f32::from_bits(bits[0]) * 4.0,
        0.0,
        f32::from_bits(bits[0]) * 4.0,
    ]);
    v
}

#[test]
fn qa_saturated_sticky() {
    for n_max in [1, 2, 7, 33, 50, 64, 100, NSUB_TABLE_MAX as u32] {
        let key = cfg(R_SUB_DEFAULT, GAMMA_SUB_DEFAULT, n_max);
        check_sticky(&key, &cap_sequence(&key), |s, _, _| s.saturated);
    }
}

negative_control!(
    qa_saturated_sticky,
    "a flag that is the last step's N_sub == N_max, not sticky, clears on relaxing",
    expected = "saturated is false",
    {
        let key = cfg(R_SUB_DEFAULT, GAMMA_SUB_DEFAULT, 50);
        check_sticky(&key, &cap_sequence(&key), |_, n, m| n == m);
    }
);

mod power_of_two_control {
    use super::*;
    negative_control!(
        qa_saturated_sticky,
        "a flag set when N_sub reaches the power of two below a non-power-of-two N_max sets too early",
        expected = "saturated is true",
        {
            let key = cfg(R_SUB_DEFAULT, GAMMA_SUB_DEFAULT, 50);
            check_sticky(&key, &cap_sequence(&key), |s, _, _| {
                s.saturated || s.live_n_sub >= 32
            });
        }
    );
}

// ── REQ-TOOL-034: total_substeps and its log proxy ──────────────────────────────────────────────────────────────

/// `⌊log₂ t⌋` by shifting, 0 for `t ≤ 1`.
fn floor_log2(t: u32) -> u32 {
    let mut k = 0;
    let mut v = t;
    while v > 1 {
        v >>= 1;
        k += 1;
    }
    k
}

fn check_proxy(proxy: fn(u32) -> u32) {
    let mut at = vec![0u32, 1, 2, 3, u32::MAX];
    for k in 1..32 {
        let p = 1u32 << k;
        at.extend([p - 1, p, p + 1]);
    }
    for t in at {
        assert_eq!(proxy(t), floor_log2(t), "the proxy at total {t}");
    }
}

/// `total_substeps` is `Σ N_sub` exactly, over a run of `d²` across the buckets; the proxy at the run's total agrees.
fn check_total(key: &NsubConfig) {
    let bits = threshold_bits(key).expect("generates");
    let table = thresholds(key).expect("generates");
    let mut s = SubstepState::default();
    let mut sum = 0u64;
    for d2 in walk(&bits, key.n_max).into_iter().cycle().take(5000) {
        sum += u64::from(s.macro_step(d2, &table, key.n_max));
        assert_eq!(
            u64::from(s.total_substeps),
            sum,
            "total_substeps is not Σ N_sub"
        );
    }
    assert_eq!(
        total_substeps_log2(s.total_substeps),
        floor_log2(s.total_substeps),
        "the proxy at the run's total"
    );
}

#[test]
fn qa_total_substeps_proxy() {
    check_proxy(total_substeps_log2);
    for key in sim_keys() {
        check_total(&key);
    }
}

negative_control!(
    qa_total_substeps_proxy,
    "a proxy taken as 32 − clz, ⌈log₂⌉ at a power of two, fails",
    expected = "the proxy at total",
    check_proxy(|t| if t > 1 { 32 - t.leading_zeros() } else { 0 })
);
