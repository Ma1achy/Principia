//! The `invariant-gradient` gate on the GPU (`validation::invariant_gradient`; TASK-M2-12, REQ-VAL-015), its kernel the
//! WGSL `cargo xtask build-kernel` writes; CI runs this file in the `gpu-kernel` job:
//! - `invariant_gradient_verdict`: the decode stage's `E₀` on both charts passes at a threshold just above its worst
//!   measure and fails just below it; the report gives the three measures, the tags compared and the
//!   infeasible count, 0.
//! - `invariant_gradient_fixtures`: the runner on `fixtures/gates/invariant-gradient/` gives every input its expected
//!   outcome: both charts pass at 256 eps, the perturbed construction fails (pitfalls §3).

use std::path::{Path, PathBuf};

use validation::decode_f32_eps::kernel_wgsl;
use validation::gate::{self, GateConfig, GateReport, Verdict};
use validation::invariant_gradient::{Frozen, Input, InvariantGradientGate, Which, NAME};
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The gate's configuration, its threshold set to `value`.
fn config(value: f64) -> GateConfig {
    let mut config = gate::config(&root(), NAME).expect("gate.json");
    config.threshold.value = value;
    config
}

fn input(chart: Which) -> Input {
    Input {
        description: "a test input".to_owned(),
        chart,
        frozen: vec![
            Frozen {
                z_cfg: [0.0, 0.0],
                z_mass: [0.0, 0.0],
            },
            Frozen {
                z_cfg: [0.3, -0.5],
                z_mass: [0.7, -0.4],
            },
        ],
        grid: 16,
        perturb: None,
    }
}

/// The report's worst measure, read back from its line.
fn worst(report: &GateReport) -> f64 {
    let line = report
        .scatter
        .iter()
        .find(|l| l.starts_with("E₀ against U + K(t)"))
        .expect("the measures' line");
    line.split(" eps")
        .filter_map(|part| part.rsplit(' ').next()?.parse::<f64>().ok())
        .fold(0.0, f64::max)
}

type Judge = fn(&GateConfig, &str, &Input, &str) -> Result<GateReport, String>;

fn check_verdict(judge: Judge) {
    let wgsl = kernel_wgsl(&root()).expect("the kernel's WGSL");
    for chart in [Which::LzE, Which::LzK] {
        let first = judge(&config(256.0), "x.json", &input(chart), &wgsl).expect("the judge");
        assert_eq!(first.verdict, Verdict::Pass, "{chart:?} fails at 256 eps");
        let max = worst(&first);
        assert!(
            max > 0.0,
            "{chart:?}: the worst measure is 0; the measure reads nothing"
        );
        // The report prints two decimals, so the bounds sit 1 % either side of the worst it prints.
        let at = judge(&config(max * 1.01), "x.json", &input(chart), &wgsl).expect("the judge");
        assert_eq!(
            at.verdict,
            Verdict::Pass,
            "{chart:?} fails just above its own worst"
        );
        let below = judge(&config(max * 0.99), "x.json", &input(chart), &wgsl).expect("the judge");
        assert_eq!(
            below.verdict,
            Verdict::Fail,
            "{chart:?} passes below its worst, {max}"
        );
        let text = first.negative_results.join("\n");
        for want in [
            "0 DEGENERATE tags differ",
            "infeasible pixels on the grids: 0 at f64, 0 natively at f32",
        ] {
            assert!(
                text.contains(want),
                "{chart:?}: the report lacks `{want}`: {text}"
            );
        }
    }
}

#[test]
fn invariant_gradient_verdict() {
    check_verdict(InvariantGradientGate::judge);
}

/// A judge that passes every input.
#[cfg(feature = "controls")]
fn always_pass(c: &GateConfig, n: &str, i: &Input, w: &str) -> Result<GateReport, String> {
    let mut r = InvariantGradientGate::judge(c, n, i, w)?;
    r.verdict = Verdict::Pass;
    Ok(r)
}

negative_control!(
    invariant_gradient_verdict,
    "a judge passing every input passes below the worst",
    expected = "passes below its worst",
    check_verdict(always_pass)
);

/// The runner on `root`'s `fixtures/gates/invariant-gradient/` gives every input its expected outcome.
fn check_fixtures(root: &Path) {
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("invariant_gradient_gate");
    let result = gate::run(root, NAME, &out);
    assert!(
        result.is_ok(),
        "the gate's fixtures do not give their expected outcomes: {result:?}"
    );
}

#[test]
fn invariant_gradient_fixtures() {
    check_fixtures(&root());
}

negative_control!(
    invariant_gradient_fixtures,
    "the perturbed fixture expected to pass must fail the run",
    expected = "do not give their expected outcomes",
    {
        let copy = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("invariant_gradient_copy");
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
