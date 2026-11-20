//! The `decode-f32-eps` gate's host-side parts (`validation::decode_f32_eps`; TASK-M2-06, REQ-DEC-043), with no device:
//! - `decode_f32_eps_error`: the measure, `|x₃₂ − x₆₄| / (eps · max(|x₆₄|, 1))`, at and either side of `|x₆₄| = 1`,
//!   and equal non-finite bits differing by 0, any other non-finite pair by `+∞`.
//! - `decode_f32_eps_measure`: each quantity's worst and its sample, each sample's worst, the overall worst, and the
//!   refusals: unequal counts and unequal `DEGENERATE` tags.
//! - `decode_f32_eps_fuzz`: SplitMix64's published sequence, `uniform`'s range, and the fuzzed interior's counts and
//!   bounds; the golden batch is `z = 0`.
//! - `decode_f32_eps_native`: the native f32 decodes are `sample` at f32, point by point, in order.
//! - `decode_f32_eps_inputs`: the fixtures' JSON shape, the physics sim key, the kernel's path, and the gate's name.
//!
//! The GPU parts are `decode_f32_eps_gpu.rs`.

use std::path::Path;

use kernel::chart::Latent;
use kernel::decode::Degenerate;
use kernel::stage::decode::{sample, DecodeOut, DecodeVariant, LatentDefaults};
use validation::decode_f32_eps::{
    decode_native_f32, error, golden, interior, kernel_wgsl, measure, physics_key, values, Batch,
    DecodeF32EpsGate, Fuzz, Input, Points, SplitMix, NAME, QUANTITIES,
};
use validation::gate::Gate;
use validation::negative_control;

type Links = <LatentDefaults as DecodeVariant>::Links;

const EPS: f64 = f32::EPSILON as f64;

// ── The measure ──────────────────────────────────────────────────────────────────────────────────────────────────

fn check_error(e: fn(f64, f64) -> f64) {
    // Below 1 the scale is 1; at 1 and above it is |x64|.
    for (x32, x64, want) in [
        (0.5 + EPS, 0.5, 1.0),
        (1.0 + EPS, 1.0, 1.0),
        (2.0 + 2.0 * EPS, 2.0, 1.0),
        (-2.0 - 4.0 * EPS, -2.0, 2.0),
        (EPS, 0.0, 1.0),
        (0.75, 0.75, 0.0),
    ] {
        let got = e(x32, x64);
        assert!(
            (got - want).abs() <= 1e-9,
            "error({x32:e}, {x64:e}) is {got}, not {want}"
        );
    }
    for (x32, x64, want) in [
        (f64::INFINITY, f64::INFINITY, 0.0),
        (f64::NEG_INFINITY, f64::NEG_INFINITY, 0.0),
        (f64::NAN, f64::NAN, 0.0),
        (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY),
        (f64::INFINITY, 1.0, f64::INFINITY),
        (1.0, f64::NEG_INFINITY, f64::INFINITY),
        (f64::NAN, 0.0, f64::INFINITY),
    ] {
        assert_eq!(
            e(x32, x64).to_bits(),
            want.to_bits(),
            "error({x32:e}, {x64:e})"
        );
    }
}

#[test]
fn decode_f32_eps_error() {
    check_error(error);
}

negative_control!(
    decode_f32_eps_error,
    "a measure relative below 1 must fail",
    expected = "error(5.000001192092896e-1, 5e-1)",
    check_error(|a, b| {
        if a.is_finite() && b.is_finite() {
            (a - b).abs() / (EPS * b.abs())
        } else {
            error(a, b)
        }
    })
);

/// The f32 and f64 decodes of three interior points of one chart.
fn pairs() -> (Vec<DecodeOut<f32>>, Vec<DecodeOut<f64>>) {
    let chart = Latent {
        z0: [0.3, -0.2, 0.1, 0.4, -0.5, 0.2, 0.6, -0.1],
        q1: [0.2; 8],
        q2: [-0.1; 8],
    };
    let wide = Latent {
        z0: chart.z0.map(f64::from),
        q1: chart.q1.map(f64::from),
        q2: chart.q2.map(f64::from),
    };
    let points = [[0.2f32, 0.7], [0.5, 0.5], [0.9, 0.1]];
    let narrow = points
        .iter()
        .map(|p| sample::<_, Links, f32>(&chart, p[0], p[1]))
        .collect();
    let wide = points
        .iter()
        .map(|p| sample::<_, Links, f64>(&wide, f64::from(p[0]), f64::from(p[1])))
        .collect();
    (narrow, wide)
}

type Measure = fn(
    &[DecodeOut<f32>],
    &[DecodeOut<f64>],
) -> Result<validation::decode_f32_eps::Measurement, String>;

fn check_measure(m: Measure) {
    let (mut gpu, cpu) = pairs();
    // Sample 1's m0 off by 3 eps, sample 2's V_0 by 5 eps of its magnitude: the worsts, at their samples.
    gpu[1].ic.m[0] = (cpu[1].ic.m[0] + 3.0 * EPS) as f32;
    let v0 = cpu[2].descriptor.V_0;
    gpu[2].descriptor.V_0 = (v0 + 5.0 * EPS * v0.abs().max(1.0)) as f32;
    let got = m(&gpu, &cpu).expect("the pairs");
    assert_eq!(got.samples, 3, "the sample count");
    let k_m0 = QUANTITIES.iter().position(|q| *q == "m0").unwrap();
    let k_v0 = QUANTITIES.iter().position(|q| *q == "V_0").unwrap();
    let want = |x32: f64, x64: f64| error(x32, x64);
    let e_m0 = want(f64::from(gpu[1].ic.m[0]), cpu[1].ic.m[0]);
    assert!(
        got.worst[k_m0].eps >= e_m0 && got.worst[k_m0].at == 1,
        "m0's worst is {:?}, not {e_m0} at sample 1",
        got.worst[k_m0]
    );
    let e_v0 = want(f64::from(gpu[2].descriptor.V_0), v0);
    assert!(
        got.worst[k_v0].eps >= e_v0 && got.worst[k_v0].at == 2,
        "V_0's worst is {:?}, not {e_v0} at sample 2",
        got.worst[k_v0]
    );
    for (i, s) in got.per_sample.iter().enumerate() {
        let w = values(&gpu[i])
            .into_iter()
            .zip(values(&cpu[i]))
            .map(|(a, b)| error(a, b))
            .fold(0.0, f64::max);
        assert_eq!(*s, w, "sample {i}'s worst");
    }
    let (max, k) = got.max();
    let top = got.worst.iter().map(|w| w.eps).fold(0.0, f64::max);
    assert_eq!(max, top, "the overall worst");
    assert_eq!(got.worst[k].eps, top, "the overall worst's quantity");
    // A tie keeps the first: two quantities equal and largest give the lower index.
    let mut tied = got.clone();
    tied.worst[3].eps = 1e9;
    tied.worst[20].eps = 1e9;
    assert_eq!(tied.max(), (1e9, 3), "a tie's quantity");
    // An equal error later does not move a quantity's sample.
    let twice = [cpu[0], cpu[0]];
    let narrow = [gpu[0], gpu[0]];
    let same = m(&narrow, &twice).expect("two equal pairs");
    assert!(
        same.worst.iter().all(|w| w.at == 0 || w.eps == 0.0),
        "an equal later error moved a worst: {:?}",
        same.worst
    );
    // The refusals.
    let short = m(&gpu[..2], &cpu);
    assert!(
        short
            .as_ref()
            .is_err_and(|e| e.contains("2 GPU decodes against 3 f64 decodes")),
        "unequal counts are not refused: {short:?}"
    );
    gpu[2].degenerate = Some(Degenerate::M01Tiny);
    let tagged = m(&gpu, &cpu);
    assert!(
        tagged
            .as_ref()
            .is_err_and(|e| e.contains("sample 2: the GPU tags Some(M01Tiny)")),
        "unequal tags are not refused: {tagged:?}"
    );
}

#[test]
fn decode_f32_eps_measure() {
    check_measure(measure);
}

negative_control!(
    decode_f32_eps_measure,
    "a measure blind to the tags must fail",
    expected = "unequal tags are not refused",
    check_measure(|g, c| {
        let g: Vec<DecodeOut<f32>> = g
            .iter()
            .zip(c)
            .map(|(g, c)| DecodeOut {
                degenerate: c.degenerate,
                ..*g
            })
            .collect();
        measure(&g, c)
    })
);

// ── The fuzz ─────────────────────────────────────────────────────────────────────────────────────────────────────

fn check_fuzz(draw: fn(Fuzz) -> Vec<Batch>) {
    // SplitMix64's published first outputs from seed 0.
    let mut r = SplitMix::new(0);
    assert_eq!(r.next_u64(), 0xe220_a839_7b1d_cdaf, "SplitMix64(0)'s first");
    assert_eq!(
        r.next_u64(),
        0x6e78_9e6a_a1b9_65f4,
        "SplitMix64(0)'s second"
    );
    let mut r = SplitMix::new(7);
    let draws: Vec<f32> = (0..4096).map(|_| r.uniform(-0.5, 2.0)).collect();
    assert!(
        draws.iter().all(|x| (-0.5..=2.0).contains(x)),
        "a uniform(-0.5, 2) draw outside its range"
    );
    let lo = draws.iter().copied().fold(f32::INFINITY, f32::min);
    let hi = draws.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        lo < -0.4 && hi > 1.9,
        "uniform(-0.5, 2) does not span its range: [{lo}, {hi}]"
    );
    let f = Fuzz {
        seed: 3,
        charts: 5,
        per_chart: 7,
    };
    let batches = draw(f);
    assert_eq!(batches.len(), 5, "the charts");
    assert!(
        batches.iter().all(|(_, p)| p.len() == 7),
        "the points per chart"
    );
    let span = |xs: &mut dyn Iterator<Item = f32>| {
        xs.fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), x| {
            (a.min(x), b.max(x))
        })
    };
    let (z_lo, z_hi) = span(&mut batches.iter().flat_map(|(c, _)| c.z0));
    let (q_lo, q_hi) = span(
        &mut batches
            .iter()
            .flat_map(|(c, _)| c.q1.into_iter().chain(c.q2)),
    );
    let (p_lo, p_hi) = span(
        &mut batches
            .iter()
            .flat_map(|(_, p)| p.iter().flatten().copied()),
    );
    assert!(
        (-1.0..-0.3).contains(&z_lo) && (0.3..=1.0).contains(&z_hi),
        "z0 on [{z_lo}, {z_hi}], not within and across [-1, 1]"
    );
    assert!(
        (-0.5..-0.15).contains(&q_lo) && (0.15..=0.5).contains(&q_hi),
        "q on [{q_lo}, {q_hi}], not within and across [-1/2, 1/2]"
    );
    assert!(
        (0.0..0.2).contains(&p_lo) && (0.8..=1.0).contains(&p_hi),
        "(u, v) on [{p_lo}, {p_hi}], not within and across [0, 1]"
    );
    assert_eq!(draw(f), batches, "a seed names its draws");
    let golden = golden();
    assert_eq!(golden.len(), 1, "the golden batch");
    let (chart, points) = &golden[0];
    assert!(
        chart.z0 == [0.0; 8] && chart.q1 == [0.0; 8] && chart.q2 == [0.0; 8],
        "the golden chart is not 0"
    );
    assert_eq!(points, &vec![[0.5, 0.5]], "the golden point");
}

#[test]
fn decode_f32_eps_fuzz() {
    check_fuzz(interior);
}

negative_control!(
    decode_f32_eps_fuzz,
    "an interior with its q drawn on [-1, 1] must fail",
    expected = "not within and across [-1/2, 1/2]",
    check_fuzz(|f| {
        interior(f)
            .into_iter()
            .map(|(mut c, p)| {
                c.q1 = c.q1.map(|x| x * 2.0);
                (c, p)
            })
            .collect()
    })
);

// ── The native decode ────────────────────────────────────────────────────────────────────────────────────────────

fn check_native(native: fn(&[Batch]) -> Vec<DecodeOut<f32>>) {
    let batches = interior(Fuzz {
        seed: 11,
        charts: 3,
        per_chart: 4,
    });
    let want: Vec<DecodeOut<f32>> = batches
        .iter()
        .flat_map(|(c, p)| p.iter().map(|uv| sample::<_, Links, f32>(c, uv[0], uv[1])))
        .collect();
    assert_eq!(native(&batches), want, "the native decodes");
}

#[test]
fn decode_f32_eps_native() {
    check_native(decode_native_f32);
}

negative_control!(
    decode_f32_eps_native,
    "native decodes of the first chart only must fail",
    expected = "the native decodes",
    check_native(|b| decode_native_f32(&b[..1]))
);

// ── The inputs ───────────────────────────────────────────────────────────────────────────────────────────────────

fn check_inputs(key: fn() -> engine::contract::sim_config::SimConfig) {
    let input: Input = serde_json::from_str(
        r#"{ "description": "d", "points": { "interior": { "seed": 1, "charts": 2, "per_chart": 3 } },
             "perturb": { "component": 4, "by": 0.5 }, "recorded": { "platform": "p", "max_eps": 1.5 } }"#,
    )
    .expect("an input");
    assert_eq!(
        input.points,
        Points::Interior(Fuzz {
            seed: 1,
            charts: 2,
            per_chart: 3
        }),
        "the points"
    );
    let golden: Input =
        serde_json::from_str(r#"{ "description": "d", "points": "golden" }"#).expect("golden");
    assert!(
        golden.points == Points::Golden && golden.perturb.is_none() && golden.recorded.is_none(),
        "the golden input"
    );
    assert!(
        serde_json::from_str::<Input>(r#"{ "description": "d", "points": "golden", "extra": 1 }"#)
            .is_err(),
        "an unknown field is not refused"
    );
    assert_eq!(
        key().kernel_variant,
        engine::contract::sim_config::KernelVariant::Physics,
        "the sim key's kernel"
    );
    let missing = kernel_wgsl(Path::new("/nonexistent-principia-root"));
    assert!(
        missing
            .as_ref()
            .is_err_and(|e| e.starts_with("refused: cannot read")),
        "a missing kernel is not refused: {missing:?}"
    );
    assert!(
        DecodeF32EpsGate::root().join("Cargo.toml").is_file()
            && DecodeF32EpsGate::root()
                .join("fixtures/gates")
                .join(NAME)
                .is_dir(),
        "the gate's root is not the workspace"
    );
    assert_eq!(DecodeF32EpsGate.name(), "decode-f32-eps", "the gate's name");
    let config = validation::gate::config(&DecodeF32EpsGate::root(), NAME).expect("the config");
    let bad = DecodeF32EpsGate.run(&config, "x.json", &serde_json::json!({ "points": 3 }));
    assert!(
        bad.as_ref()
            .is_err_and(|e| e.contains("x.json is not a decode-f32-eps input")),
        "a malformed input is not refused: {:?}",
        bad.map(|_| ())
    );
}

#[test]
fn decode_f32_eps_inputs() {
    check_inputs(physics_key);
}

negative_control!(
    decode_f32_eps_inputs,
    "a sim key of the bring-up kernel must fail",
    expected = "the sim key's kernel",
    check_inputs(|| engine::contract::sim_config::SimConfig {
        kernel_variant: engine::contract::sim_config::KernelVariant::BringUp,
        ..physics_key()
    })
);
