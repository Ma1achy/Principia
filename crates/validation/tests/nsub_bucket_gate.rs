//! The `nsub-bucket` gate (TASK-M3-03; REQ-INT-028): its specification, edges and counts, the two legs, the verdict
//! at and past the threshold, the report's sections, and the runner on `fixtures/gates/nsub-bucket/`, where the
//! default table under a changed `r_sub` or `gamma_sub` fails and an `N_max` above the table is refused; the floor
//! pairs, and the sim key whose deciding thresholds fall below `2⁻⁵⁵` refused.

use std::path::{Path, PathBuf};

use kernel::driver::nsub_table::{pack, threshold_bits, threshold_f64, NsubConfig};
use kernel::driver::substep::square;
use kernel::generated::nsub_thresholds::NSUB_TABLE_MAX;
use validation::decode_f32_eps::SplitMix;
use validation::gate::{self, GateConfig, Minimum, RegionCount, Verdict};
use validation::negative_control;
use validation::nsub_bucket::{
    away_from_edges, count, edge, edge_samples, exact_d2, floor_pairs, floor_triple, legs, measure,
    pair_triple, random_d2, spec, triple, unit, Fuzz, Input, Law, Measurement, NsubBucketGate,
    EDGE_BAND, FLOOR_DX, NAME,
};

/// This workspace's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The gate's configuration, as `fixtures/gates/nsub-bucket/gate.json` gives it.
fn config() -> GateConfig {
    gate::config(&root(), NAME).expect("the gate's gate.json")
}

const DEFAULT: Law = Law {
    r_sub: 0.05,
    gamma_sub: 1.5,
    n_max: 64,
};

fn input(config: Law, table: Option<Law>) -> Input {
    Input {
        description: "a test input".to_owned(),
        config,
        table,
        fuzz: Fuzz {
            seed: 9,
            random: 256,
            pairs: 256,
        },
    }
}

// ── The specification and the edges ────────────────────────────────────────────────────────────────────────────

/// `spec` is `clamp(⌈(r_sub/√d²)^γ⌉, 1, N_max)`: at `r_min = r_sub·2^{−2/3}`, `(r_sub/r_min)^1.5 = 2` exactly, so 2 at
/// the edge and 3 just inside; 1 far out; `N_max` at 0 and deep inside; the edge and its band.
fn check_spec(spec: fn(&Law, f64) -> u32) {
    assert_eq!(spec(&DEFAULT, 1.0), 1, "far out");
    assert_eq!(spec(&DEFAULT, 0.0025), 1, "d² = r_sub², N_sub 1");
    assert_eq!(spec(&DEFAULT, 0.0025 * 1.000_001), 1, "just outside r_sub");
    assert_eq!(spec(&DEFAULT, 0.0025 * 0.999_999), 2, "just inside r_sub");
    assert_eq!(spec(&DEFAULT, 0.0), 64, "d² = 0");
    assert_eq!(spec(&DEFAULT, 1e-12), 64, "deep inside");
    let e2 = edge(&DEFAULT, 2);
    assert_eq!(
        spec(&DEFAULT, e2 * (1.0 + 1e-9)),
        2,
        "just outside THR[2]'s edge"
    );
    assert_eq!(
        spec(&DEFAULT, e2 * (1.0 - 1e-9)),
        3,
        "just inside THR[2]'s edge"
    );
}

#[test]
fn nsub_bucket_gate_spec() {
    check_spec(spec);
    assert!(
        (edge(&DEFAULT, 1) - 0.0025).abs() < 1e-18,
        "THR[1]'s edge is r_sub²"
    );
    let e = edge(&DEFAULT, 7);
    assert!(
        !away_from_edges(&DEFAULT, e),
        "an edge is not away from the edges"
    );
    assert!(
        !away_from_edges(&DEFAULT, e * (1.0 + 0.5 * EDGE_BAND)),
        "within the band"
    );
    assert!(
        away_from_edges(&DEFAULT, e * (1.0 + 2.0 * EDGE_BAND)),
        "past the band"
    );
    assert!(away_from_edges(&DEFAULT, 10.0), "far out");
    // Exactly at the band: at r_sub = 1/2, THR[1]'s edge is 1/4 and `d²/edge − 1` is 2⁻²⁰ with no rounding; "more than"
    // the band is strict.
    let half = Law {
        r_sub: 0.5,
        ..DEFAULT
    };
    assert!(
        !away_from_edges(&half, 0.25 * (1.0 + EDGE_BAND)),
        "exactly at the band"
    );
    assert!(
        away_from_edges(&half, 0.25 * (1.0 + 2.0 * EDGE_BAND)),
        "past the band at r_sub 1/2"
    );
}

negative_control!(
    nsub_bucket_gate_spec,
    "a specification that rounds instead of taking the ceiling",
    expected = "just inside r_sub",
    check_spec(
        |law, d2| ((law.r_sub / d2.sqrt()).powf(law.gamma_sub).round())
            .clamp(1.0, f64::from(law.n_max)) as u32
    )
);

/// `count` is `min(1 + #{m : d² < THR[m]}, N_max)` on `bits`, strict at an edge.
fn check_count(count: fn(&[u32; NSUB_TABLE_MAX], f32, u32) -> u32) {
    let bits = threshold_bits(&NsubConfig::DEFAULT).expect("generates");
    let t3 = f32::from_bits(bits[2]);
    assert_eq!(count(&bits, t3, 256), 3, "d² = THR[3]");
    assert_eq!(
        count(&bits, f32::from_bits(bits[2] - 1), 256),
        4,
        "just below THR[3]"
    );
    assert_eq!(count(&bits, 0.0, 1000), 257, "d² = 0, every threshold");
    assert_eq!(count(&bits, 0.0, 64), 64, "capped at N_max");
    assert_eq!(count(&bits, 1.0, 64), 1, "no threshold");
}

#[test]
fn nsub_bucket_gate_count() {
    check_count(count);
}

negative_control!(
    nsub_bucket_gate_count,
    "a count with ≤ at the edge",
    expected = "d² = THR[3]",
    check_count(|bits, d2, n_max| {
        let k = bits.iter().filter(|&&b| d2 <= f32::from_bits(b)).count() as u32;
        (k + 1).min(n_max)
    })
);

// ── The legs ────────────────────────────────────────────────────────────────────────────────────────────────────

/// The two legs decide alike on f64 positions and their f32 casts; `exact_d2` is the closest pair's, in f64.
fn check_legs(r: &[[f64; 2]; 3], want: (u32, u32), d2: f64) {
    let t = pack(&threshold_bits(&NsubConfig::DEFAULT).expect("generates"));
    assert_eq!(legs(r, &t, 64), want, "the legs at {r:?}");
    assert_eq!(exact_d2(r), d2, "the exact d² at {r:?}");
}

#[test]
fn nsub_bucket_gate_legs() {
    check_legs(&[[0.0, 0.0], [0.024, 0.032], [5.0, 5.0]], (2, 2), {
        let (x, y) = (f64::from(0.024f32), f64::from(0.032f32));
        x * x + y * y
    });
    check_legs(&[[1.0, 1.0], [5.0, 1.0], [1.0, 1.5]], (1, 1), 0.25);
    check_legs(&[[1.0, 1.0], [1.0, 1.0 + 1e-9], [3.0, 3.0]], (64, 64), 0.0);
}

negative_control!(
    nsub_bucket_gate_legs,
    "a wrong expected decision",
    expected = "the legs at",
    check_legs(&[[1.0, 1.0], [5.0, 1.0], [1.0, 1.5]], (2, 2), 0.25)
);

// ── The draws ───────────────────────────────────────────────────────────────────────────────────────────────────

fn default_bits() -> [u32; NSUB_TABLE_MAX] {
    threshold_bits(&NsubConfig::DEFAULT).expect("generates")
}

/// The draws cover their ranges and nothing past them: `unit` on `[0, 1)` with both halves hit; the edge samples the
/// three f32s at each threshold; the random `d²` within a quarter of the least threshold and four times the greatest,
/// past both ends of the table; the triples' `n` across `1 … L_max`, their pair `√THR[n]` apart within `2⁻¹⁶`, body 0 on
/// `[−1, 1]²` with both signs, body 2 at `(10, 10)` from it, and the pair's direction in every quadrant.
fn check_draws(
    edges: &[(String, f32)],
    random: &[f32],
    units: &[f64],
    triples: &[(u32, [[f64; 2]; 3])],
) {
    let bits = default_bits();
    assert!(
        units.iter().all(|u| (0.0..1.0).contains(u)),
        "a unit draw outside [0, 1)"
    );
    assert!(
        units.iter().any(|&u| u < 0.1) && units.iter().any(|&u| u > 0.9),
        "the unit draws do not cover [0, 1)"
    );
    assert_eq!(
        edges.len(),
        3 * NSUB_TABLE_MAX,
        "three samples per threshold"
    );
    for (i, &b) in bits.iter().enumerate() {
        let got: Vec<u32> = edges[3 * i..3 * i + 3]
            .iter()
            .map(|e| e.1.to_bits())
            .collect();
        assert_eq!(got, [b - 1, b, b + 1], "the samples at THR[{}]", i + 1);
        assert_eq!(edges[3 * i + 1].0, format!("THR[{}] exact", i + 1));
    }
    let (least, greatest) = (
        f32::from_bits(bits[NSUB_TABLE_MAX - 1]),
        f32::from_bits(bits[0]),
    );
    assert!(
        random
            .iter()
            .all(|&d| d >= least / 4.0 * 0.999 && d <= greatest * 4.0 * 1.001),
        "a random d² outside [THR[L_max]/4, 4·THR[1]]"
    );
    assert!(
        random.iter().any(|&d| d < least) && random.iter().any(|&d| d > greatest),
        "the random d² do not pass both ends of the table"
    );
    let mid = random
        .iter()
        .filter(|&&d| d >= least && d <= greatest)
        .count();
    assert!(
        mid > random.len() / 2,
        "the random d² do not fall mostly within the table (log-uniform: about 73%)"
    );
    let n: Vec<u32> = triples.iter().map(|t| t.0).collect();
    assert!(
        n.iter().all(|&k| (1..=NSUB_TABLE_MAX as u32).contains(&k)),
        "an n outside 1 … L_max"
    );
    assert!(
        n.iter().any(|&k| k < 32) && n.iter().any(|&k| k > 224),
        "the triples' n do not cover the table"
    );
    for &(k, r) in triples {
        let thr = f64::from(f32::from_bits(bits[k as usize - 1]));
        let d2 = (r[1][0] - r[0][0]).powi(2) + (r[1][1] - r[0][1]).powi(2);
        assert!(
            (d2 / thr - 1.0).abs() < 2.1 / 65536.0,
            "a pair not at THR[{k}]: {d2} against {thr}"
        );
        assert!(
            r[0].iter().all(|x| (-1.0..1.0).contains(x)),
            "body 0 outside [−1, 1]²"
        );
        assert_eq!(r[2], [r[0][0] + 10.0, r[0][1] + 10.0], "body 2");
    }
    let quadrant = |sx: bool, sy: bool| {
        triples
            .iter()
            .any(|(_, r)| (r[1][0] > r[0][0]) == sx && (r[1][1] > r[0][1]) == sy)
    };
    assert!(
        quadrant(true, true)
            && quadrant(true, false)
            && quadrant(false, true)
            && quadrant(false, false),
        "the pairs' directions do not cover every quadrant"
    );
    assert!(
        triples.iter().any(|(_, r)| r[0][0] < 0.0) && triples.iter().any(|(_, r)| r[0][1] > 0.0),
        "body 0 does not take both signs"
    );
}

/// A named edge sample, as `edge_samples` gives it.
type EdgeSample = (String, f32);

/// A triple at a threshold, as `pair_triple` gives it.
type Triple = (u32, [[f64; 2]; 3]);

/// The edge samples, random `d²`, unit draws and triples from one seed.
fn draws() -> (Vec<EdgeSample>, Vec<f32>, Vec<f64>, Vec<Triple>) {
    let bits = default_bits();
    let mut rng = SplitMix::new(3);
    let units = (0..256).map(|_| unit(&mut rng)).collect();
    let random = random_d2(&bits, &mut rng, 512);
    let triples = (0..512).map(|_| pair_triple(&bits, &mut rng)).collect();
    (edge_samples(&bits), random, units, triples)
}

/// `pair_triple`'s draw from `seed`, restated: the five unit draws in its order, `n`, the scale, body 0 and the angle.
fn restated_triple(seed: u64) -> (u32, [[f64; 2]; 3]) {
    let bits = default_bits();
    let mut rng = SplitMix::new(seed);
    let u: Vec<f64> = (0..5).map(|_| unit(&mut rng)).collect();
    let n = 1 + (u[0] * 256.0) as u32 % 256;
    let scale = 1.0 + (2.0 * u[1] - 1.0) / 65536.0;
    let s = (f64::from(f32::from_bits(bits[n as usize - 1])) * scale * scale).sqrt();
    let at = [2.0 * u[2] - 1.0, 2.0 * u[3] - 1.0];
    let angle = std::f64::consts::TAU * u[4];
    (
        n,
        [
            at,
            [at[0] + s * angle.cos(), at[1] + s * angle.sin()],
            [at[0] + 10.0, at[1] + 10.0],
        ],
    )
}

/// `pair_triple` from each of `seeds` is its restatement, bit for bit.
fn check_restated(seeds: &[u64], draw: fn(u64) -> (u32, [[f64; 2]; 3])) {
    for &seed in seeds {
        assert_eq!(
            draw(seed),
            restated_triple(seed),
            "pair_triple from seed {seed}"
        );
    }
}

fn drawn(seed: u64) -> (u32, [[f64; 2]; 3]) {
    pair_triple(&default_bits(), &mut SplitMix::new(seed))
}

#[test]
fn nsub_bucket_gate_pair_triple() {
    check_restated(&[1, 2, 3, 4, 5], drawn);
}

negative_control!(
    nsub_bucket_gate_pair_triple,
    "a draw with body 1 moved by 10⁻⁹",
    expected = "pair_triple from seed",
    check_restated(&[1, 2, 3, 4, 5], |seed| {
        let (n, mut r) = drawn(seed);
        r[1][0] += 1e-9;
        (n, r)
    })
);

#[test]
fn nsub_bucket_gate_draws() {
    let (e, r, u, t) = draws();
    check_draws(&e, &r, &u, &t);
    assert_eq!(
        triple([1.0, 2.0], 0.25, 0.0),
        [[1.0, 2.0], [1.5, 2.0], [11.0, 12.0]],
        "triple along x"
    );
    let r3 = triple([1.0, 2.0], 0.25, std::f64::consts::FRAC_PI_2);
    assert!(
        (r3[1][0] - 1.0).abs() < 1e-15 && (r3[1][1] - 2.5).abs() < 1e-15,
        "triple's pair: {r3:?}"
    );
}

negative_control!(
    nsub_bucket_gate_draws,
    "random d² drawn from a range ending at the table's ends",
    expected = "the random d² do not pass both ends of the table",
    {
        let (e, _, u, t) = draws();
        let bits = default_bits();
        let r: Vec<f32> = (0..64).map(|i| f32::from_bits(bits[i * 4])).collect();
        check_draws(&e, &r, &u, &t);
    }
);

/// A measurement's records: a decision on each leg, a fork, an edge sample and its difference, a judgement.
fn check_records(m: &Measurement) {
    assert_eq!(m.decisions, 3, "decisions");
    assert_eq!(m.forks, 1, "forks");
    assert_eq!(
        m.first_fork.as_deref(),
        Some("c: f64 leg 3, f32 leg 4"),
        "the first fork"
    );
    assert_eq!(m.n_sub_range, (3, 64), "the range of N_sub");
    assert_eq!(m.saturated, 1, "decisions at N_max");
    assert_eq!((m.edge_samples, m.edge_mismatches), (2, 1), "edge samples");
    assert!(m
        .first_edge_mismatch
        .as_deref()
        .is_some_and(|s| s.starts_with("f: d² ")));
    assert_eq!(
        (m.judged, m.mismatches, m.near_edge),
        (2, 1, 1),
        "judgements"
    );
    assert!(m
        .first_mismatch
        .as_deref()
        .is_some_and(|s| s.contains("N_sub 5, the specification 1")));
    assert_eq!(
        m.failures(),
        3,
        "failures: one fork, one edge difference, one from the specification"
    );
}

fn records() -> Measurement {
    let mut m = Measurement::default();
    m.decide("a", (5, 5), 64);
    m.decide("b", (64, 64), 64);
    m.decide("c", (3, 4), 64);
    m.edge("e", 1.0, 2, 2);
    m.edge("f", 1.0, 2, 3);
    m.judge("g", &DEFAULT, 1.0, 1);
    m.judge("h", &DEFAULT, 1.0, 5);
    m.judge("i", &DEFAULT, edge(&DEFAULT, 9), 9);
    m
}

#[test]
fn nsub_bucket_gate_records() {
    check_records(&records());
}

negative_control!(
    nsub_bucket_gate_records,
    "a measurement with one fewer fork",
    expected = "forks",
    {
        let mut m = records();
        m.forks = 0;
        check_records(&m);
    }
);

// ── The measurement, the verdict and the report ─────────────────────────────────────────────────────────────────

#[test]
fn nsub_bucket_gate_measure() {
    let m = measure(&input(DEFAULT, None)).expect("measures");
    assert_eq!(
        m.edge_samples,
        3 * NSUB_TABLE_MAX as u32,
        "three samples per edge"
    );
    assert_eq!(m.edge_mismatches, 0);
    assert_eq!(
        m.decisions,
        m.edge_samples + 256 + 256 + 2 * 63,
        "every sample decided on both legs, two floor pairs per deciding threshold"
    );
    assert_eq!(m.forks, 0);
    assert_eq!(m.mismatches, 0);
    assert_eq!(
        m.judged + m.near_edge,
        m.decisions,
        "each sample judged or counted near an edge"
    );
    assert!(
        m.near_edge >= m.edge_samples,
        "every edge sample is near an edge"
    );
    assert_eq!(m.n_sub_range, (1, 64), "N_sub across 1 … N_max");
    assert!(
        m.saturated > 0 && m.saturated < m.decisions,
        "some decisions at N_max, not all"
    );
    let control = measure(&input(
        Law {
            r_sub: 0.08,
            ..DEFAULT
        },
        Some(DEFAULT),
    ))
    .expect("measures");
    assert!(
        control.mismatches > 0,
        "the default table under r_sub 0.08 differs from the specification"
    );
    assert!(control.first_mismatch.is_some());
    assert!(
        measure(&input(
            Law {
                n_max: 257,
                ..DEFAULT
            },
            None
        ))
        .is_err_and(|e| e.contains("above the threshold table's maximum length")),
        "N_max 257 is refused"
    );
    assert!(
        measure(&input(
            DEFAULT,
            Some(Law {
                gamma_sub: 0.0,
                ..DEFAULT
            })
        ))
        .is_err(),
        "a refused table's sim key is refused"
    );
}

negative_control!(
    nsub_bucket_gate_measure,
    "the control's measurement read as clean",
    expected = "the default table under r_sub 0.08 is clean",
    {
        let control = measure(&input(
            Law {
                r_sub: 0.08,
                ..DEFAULT
            },
            Some(DEFAULT),
        ))
        .expect("measures");
        assert_eq!(
            control.mismatches, 0,
            "the default table under r_sub 0.08 is clean"
        );
    }
);

// ── The floor pairs and the least deciding threshold ───────────────────────────────────────────────────────────

/// The sim key just inside the least deciding threshold: at γ_sub 0.25, THR[55] ≥ 2⁻⁵⁵ > THR[56].
const FLOOR: Law = Law {
    r_sub: 0.05,
    gamma_sub: 0.25,
    n_max: 56,
};

/// The floor pairs of `law` over `bits`, measured alone.
fn floor_measure(law: &Law, bits: &[u32; NSUB_TABLE_MAX]) -> Measurement {
    let mut m = Measurement::default();
    floor_pairs(&mut m, law, bits, &pack(bits));
    m
}

/// `law`'s thresholds rounded once to f32, without the host generator's refusals.
fn unchecked_bits(law: &Law) -> [u32; NSUB_TABLE_MAX] {
    let mut bits = [0; NSUB_TABLE_MAX];
    for (i, b) in bits.iter_mut().enumerate() {
        *b = (threshold_f64(law.r_sub, law.gamma_sub, i as u32 + 1) as f32).to_bits();
    }
    bits
}

#[test]
fn nsub_bucket_gate_floor() {
    assert_eq!(FLOOR_DX.to_bits(), 0x2B7F_FFFF);
    assert_eq!(square(FLOOR_DX), 0.0, "FLOOR_DX is floored");
    assert_ne!(square(f32::from_bits(0x2C00_0000)), 0.0, "2⁻⁴⁰ is not");
    let bits = threshold_bits(&FLOOR.config()).expect("the sim key is taken");
    assert_eq!(
        bits,
        unchecked_bits(&FLOOR),
        "an accepted table is the one rounding"
    );
    let r = floor_triple(&bits, 3, -1.0);
    assert_eq!(r[0], [0.0, 0.0]);
    assert_eq!(r[1][0], f64::from(FLOOR_DX));
    let thr = f64::from(f32::from_bits(bits[2]));
    assert_eq!(r[1][1], (thr * (1.0 - 1.0 / 32768.0)).sqrt());
    assert_eq!(r[2], [10.0, 10.0]);
    assert_eq!(
        floor_triple(&bits, 3, 1.0)[1][1],
        (thr * (1.0 + 1.0 / 32768.0)).sqrt()
    );
    for law in [FLOOR, DEFAULT] {
        let bits = threshold_bits(&law.config()).expect("the sim key is taken");
        let m = floor_measure(&law, &bits);
        assert_eq!(
            m.decisions,
            2 * (law.n_max - 1),
            "two pairs per deciding threshold"
        );
        assert_eq!(
            m.judged, m.decisions,
            "every floor pair is judged, none near an edge"
        );
        assert_eq!(
            (m.forks, m.mismatches),
            (0, 0),
            "{law:?}: {:?}",
            m.first_mismatch
        );
        // Both sides of each edge: below THR[n] is n + 1, above it n, so 1 … N_max, and N_max once, below THR[N_max − 1].
        assert_eq!(
            m.n_sub_range,
            (1, law.n_max),
            "{law:?}: both sides of every edge"
        );
        assert_eq!(m.saturated, 1, "{law:?}: N_max only below THR[N_max − 1]");
    }
    let whole = measure(&input(FLOOR, None)).expect("measures");
    assert_eq!(whole.failures(), 0, "{:?}", whole.first_mismatch);
    for n_max in [57, 256] {
        assert!(
            measure(&input(Law { n_max, ..FLOOR }, None))
                .is_err_and(|e| e.contains("THR[56]") && e.contains("below 2^-55")),
            "N_max {n_max} at gamma_sub 0.25 is refused"
        );
    }
    let c = config();
    assert_eq!(
        c.min_regions,
        Minimum::Declared {
            value: NSUB_TABLE_MAX as u32,
            requirement: "REQ-INT-094".to_owned()
        },
        "the region minimum is L_max (REQ-INT-094)"
    );
    let report = NsubBucketGate::judge(&c, "test.json", &input(FLOOR, None)).expect("judges");
    assert!(report
        .regions
        .as_ref()
        .is_some_and(|r| r.flags().is_empty()));
    assert!(report
        .negative_results
        .iter()
        .any(|l| l.contains("the count suffices, provisionally")));
}

negative_control!(
    nsub_bucket_gate_floor,
    "the physics review's sim key, r_sub 0.05, γ_sub 0.25, N_max 256, its table made without the refusal: the floor \
     pairs differ from the specification",
    expected = "the floor decides nothing",
    {
        let law = Law {
            n_max: 256,
            ..FLOOR
        };
        let m = floor_measure(&law, &unchecked_bits(&law));
        assert_eq!(m.mismatches, 0, "the floor decides nothing");
    }
);

/// The verdict on `input` against a threshold of `value`.
fn verdict(input: &Input, value: f64) -> Verdict {
    let mut c = config();
    c.threshold.value = value;
    NsubBucketGate::judge(&c, "test.json", input)
        .expect("judges")
        .verdict
}

#[test]
fn nsub_bucket_gate_verdict() {
    assert_eq!(
        config().threshold.value,
        0.0,
        "0 forks, 0 differences (REQ-INT-028)"
    );
    assert_eq!(verdict(&input(DEFAULT, None), 0.0), Verdict::Pass);
    let control = input(
        Law {
            r_sub: 0.08,
            ..DEFAULT
        },
        Some(DEFAULT),
    );
    let failures = f64::from(measure(&control).expect("measures").mismatches);
    assert_eq!(verdict(&control, 0.0), Verdict::Fail);
    assert_eq!(
        verdict(&control, failures),
        Verdict::Pass,
        "at the threshold"
    );
    assert_eq!(
        verdict(&control, failures - 1.0),
        Verdict::Fail,
        "one past it"
    );
    let report = NsubBucketGate::judge(&config(), "test.json", &control).expect("judges");
    assert_eq!(
        report.regions.as_ref().map(|r| r.count.clone()),
        Some(RegionCount::Recorded(256))
    );
    assert!(report.negative_results[0].starts_with("domain: identical inputs per step"));
    assert!(report
        .scatter
        .iter()
        .any(|l| l.contains("the control: another sim key's table")));
    assert!(report
        .scatter
        .iter()
        .any(|l| l.starts_with("first difference from the specification")));
    report.markdown().expect("the report has every section");
}

negative_control!(
    nsub_bucket_gate_verdict,
    "the control judged as passing",
    expected = "assertion",
    assert_eq!(
        verdict(
            &input(
                Law {
                    r_sub: 0.08,
                    ..DEFAULT
                },
                Some(DEFAULT)
            ),
            0.0
        ),
        Verdict::Pass
    )
);

#[test]
fn nsub_bucket_gate_runner() {
    let out = std::env::temp_dir().join(format!("nsub-bucket-gate-{}", std::process::id()));
    let result = gate::run(&root(), NAME, &out);
    let _ = std::fs::remove_dir_all(&out);
    result.expect("every input gives its expected outcome");
}

negative_control!(
    nsub_bucket_gate_runner,
    "an unknown gate",
    expected = "no gate `nsub-buckets`",
    {
        let out = std::env::temp_dir().join("nsub-bucket-gate-control");
        gate::run(&root(), "nsub-buckets", &out).unwrap();
    }
);
