//! The `nsub-bucket` gate (TASK-M3-03; REQ-INT-028): its specification, edges and counts, the two legs, the verdict
//! at and past the threshold, the report's sections, and the runner on `fixtures/gates/nsub-bucket/`, where the
//! default table under a changed `r_sub` or `gamma_sub` fails and an `N_max` above the table is refused.

use std::path::{Path, PathBuf};

use kernel::driver::nsub_table::{pack, threshold_bits, NsubConfig};
use kernel::generated::nsub_thresholds::NSUB_TABLE_MAX;
use validation::gate::{self, GateConfig, RegionCount, Verdict};
use validation::negative_control;
use validation::nsub_bucket::{
    away_from_edges, count, edge, exact_d2, legs, measure, spec, Fuzz, Input, Law, NsubBucketGate,
    EDGE_BAND, NAME,
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
        m.edge_samples + 256 + 256,
        "every sample decided on both legs"
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
