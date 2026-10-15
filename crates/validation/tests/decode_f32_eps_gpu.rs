//! The `decode-f32-eps` gate on the GPU (`validation::decode_f32_eps`; TASK-M2-06, REQ-DEC-043), its kernel the WGSL
//! `cargo xtask build-kernel` writes; CI runs this file in the `gpu-kernel` job:
//! - `decode_f32_eps_verdict`: on a small fuzzed interior the gate passes at a threshold equal to its worst error and
//!   fails at one of its per-sample errors, counting the samples beyond; the report gives the worst, its quantity, the
//!   per-sample median and p99, the region count, a recorded measurement and the perturbation.
//! - `decode_f32_eps_refusals`: a perturbation naming no latent component or changing nothing, and a fuzz drawing no
//!   samples, are refused before any decode.
//! - `decode_f32_eps_fixtures`: the runner on `fixtures/gates/decode-f32-eps/` gives every input its expected outcome:
//!   the golden IC and the fuzzed interior pass at 256 eps, the perturbed reference fails (pitfalls §3).

use std::path::{Path, PathBuf};

use validation::decode_f32_eps::{
    decode_both, interior, kernel_wgsl, measure, DecodeF32EpsGate, Fuzz, Input, Perturb, Points,
    Recorded, NAME, QUANTITIES,
};
use validation::gate::{self, GateConfig, GateReport, RegionCount, Verdict};
use validation::gpu::GpuHarness;
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn wgsl() -> String {
    kernel_wgsl(&root()).expect("the kernel's WGSL")
}

/// The gate's configuration, its threshold set to `value`.
fn config(value: f64) -> GateConfig {
    let mut config = gate::config(&root(), NAME).unwrap();
    config.threshold.value = value;
    config
}

const FUZZ: Fuzz = Fuzz {
    seed: 5,
    charts: 4,
    per_chart: 64,
};

fn input(perturb: Option<Perturb>, recorded: Option<Recorded>) -> Input {
    Input {
        description: "a test input".to_owned(),
        points: Points::Interior(FUZZ),
        perturb,
        recorded,
    }
}

type Judge = fn(&GateConfig, &str, &Input, &str) -> Result<GateReport, String>;

fn check_verdict(judge: Judge) {
    let wgsl = wgsl();
    let h = GpuHarness::new().expect("a GPU device");
    let (gpu, cpu) = decode_both(&h, &wgsl, &interior(FUZZ), None).expect("the decodes");
    let m = measure(&gpu, &cpu).expect("the measurement");
    let (max, k) = m.max();
    let n = m.samples;
    assert_eq!(n, 256, "the fuzz's samples");
    let mut sorted = m.per_sample.clone();
    sorted.sort_by(f64::total_cmp);
    let recorded = Recorded {
        platform: "here".to_owned(),
        max_eps: 1.25,
    };
    let pass = judge(&config(max), "x.json", &input(None, Some(recorded)), &wgsl).unwrap();
    assert_eq!(
        pass.verdict,
        Verdict::Pass,
        "the interior fails at its own worst"
    );
    let text = pass.scatter.join("\n") + "\n" + &pass.negative_results.join("\n");
    for want in [
        format!(
            "worst {max:.2} eps on {} at sample {}; per-sample worst: median {:.2}, p99 {:.2}, over {n} samples",
            QUANTITIES[k], m.worst[k].at, sorted[127], sorted[253]
        ),
        format!("0 of {n} samples beyond {max} eps; margin 1.0× over the worst"),
        "recorded on here: worst 1.25 eps".to_owned(),
        "every pair's DEGENERATE tag is the same on the GPU and at f64".to_owned(),
    ] {
        assert!(text.contains(&want), "the report lacks `{want}`:\n{text}");
    }
    assert_eq!(
        pass.regions.as_ref().map(|r| r.count.clone()),
        Some(RegionCount::Recorded(256)),
        "the region count"
    );
    // At the 101st smallest per-sample error, the samples strictly above it fail.
    let at = sorted[100];
    let over = m.per_sample.iter().filter(|&&e| e > at).count();
    let fail = judge(&config(at), "x.json", &input(None, None), &wgsl).unwrap();
    assert_eq!(
        fail.verdict,
        Verdict::Fail,
        "the interior passes below its worst"
    );
    let line = format!(
        "{over} of {n} samples beyond {at} eps; the worst, {max:.2} eps on {}, fails",
        QUANTITIES[k]
    );
    assert!(
        fail.negative_results.contains(&line),
        "the report lacks `{line}`: {:?}",
        fail.negative_results
    );
    // A perturbed reference fails at 256 eps and says so.
    let p = Perturb {
        component: 2,
        by: 1e-4,
    };
    let perturbed = judge(&config(256.0), "x.json", &input(Some(p), None), &wgsl).unwrap();
    assert_eq!(
        perturbed.verdict,
        Verdict::Fail,
        "a perturbed reference passes"
    );
    assert!(
        perturbed
            .negative_results
            .iter()
            .any(|l| l.contains("z₀[2] changed by 1e-4")),
        "the report does not name the perturbation: {:?}",
        perturbed.negative_results
    );
}

#[test]
fn decode_f32_eps_verdict() {
    check_verdict(DecodeF32EpsGate::judge);
}

negative_control!(
    decode_f32_eps_verdict,
    "a gate whose report drops the recorded measurement must fail",
    expected = "the report lacks `recorded on here",
    check_verdict(|c, n, i, w| {
        let mut i = i.clone();
        i.recorded = None;
        DecodeF32EpsGate::judge(c, n, &i, w)
    })
);

fn check_refusals(judge: Judge) {
    let fuzz = |charts, per_chart| Input {
        points: Points::Interior(Fuzz {
            seed: 1,
            charts,
            per_chart,
        }),
        ..input(None, None)
    };
    let perturb = |component, by| input(Some(Perturb { component, by }), None);
    for (what, i, says) in [
        ("component 8", perturb(8, 1e-4), "names no latent component"),
        (
            "a change of 0",
            perturb(0, 0.0),
            "names no latent component",
        ),
        (
            "a NaN change",
            perturb(0, f64::NAN),
            "names no latent component",
        ),
        (
            "an infinite change",
            perturb(7, f64::INFINITY),
            "names no latent component",
        ),
        ("no charts", fuzz(0, 4), "draws no samples"),
        ("no points", fuzz(4, 0), "draws no samples"),
    ] {
        // No WGSL: a refusal comes before any decode.
        let got = judge(&config(256.0), "x.json", &i, "");
        assert!(
            got.as_ref()
                .is_err_and(|e| e.starts_with("refused:") && e.contains(says)),
            "{what} is not refused: {:?}",
            got.map(|r| r.verdict)
        );
    }
    // Component 7 and a change of the smallest subnormal are an input.
    let edge = perturb(7, f64::from_bits(1));
    let got = judge(
        &config(256.0),
        "x.json",
        &Input {
            points: Points::Golden,
            ..edge
        },
        &wgsl(),
    );
    assert!(
        got.is_ok(),
        "component 7 by a subnormal is refused: {got:?}",
        got = got.map(|r| r.verdict)
    );
}

#[test]
fn decode_f32_eps_refusals() {
    check_refusals(DecodeF32EpsGate::judge);
}

negative_control!(
    decode_f32_eps_refusals,
    "a gate refusing nothing must fail",
    expected = "component 8 is not refused",
    check_refusals(|_, _, _, _| Err("no".to_owned()))
);

/// The runner on `root`'s `fixtures/gates/decode-f32-eps/` gives every input its expected outcome.
fn check_fixtures(root: &Path) {
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("decode_f32_eps_gate");
    let result = gate::run(root, NAME, &out);
    assert!(
        result.is_ok(),
        "the gate's fixtures do not give their expected outcomes: {result:?}"
    );
}

#[test]
fn decode_f32_eps_fixtures() {
    check_fixtures(&root());
}

negative_control!(
    decode_f32_eps_fixtures,
    "the perturbed fixture expected to pass must fail the run",
    expected = "do not give their expected outcomes",
    {
        let copy = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("decode_f32_eps_copy");
        let dir = copy.join("fixtures/gates").join(NAME);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(copy.join("plan")).unwrap();
        std::fs::copy(
            root().join("plan/requirements.yaml"),
            copy.join("plan/requirements.yaml"),
        )
        .unwrap();
        for entry in std::fs::read_dir(root().join("fixtures/gates").join(NAME)).unwrap() {
            let path = entry.unwrap().path();
            std::fs::copy(&path, dir.join(path.file_name().unwrap())).unwrap();
        }
        let perturbed = dir.join("perturbed.json");
        let text = std::fs::read_to_string(&perturbed)
            .unwrap()
            .replace(r#""expected": "fail""#, r#""expected": "pass""#);
        std::fs::write(&perturbed, text).unwrap();
        check_fixtures(&copy)
    }
);
