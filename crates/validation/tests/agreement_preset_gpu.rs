//! The agreement preset `|E(fragment-decode) − E₀|` (REQ-COL-006; R-133, R-297, R-436) on the GPU: the fragment's
//! generated decode against the kernel decode stage's descriptor, as the payload stores it.
//!
//! In validation, not render, since it reads the built kernel (`target/spirv/kernel.wgsl`) and runs in CI's `gpu-kernel`
//! job (`.config/nextest.toml`), which tests validation, xtask and engine.
//!
//! - `agreement_preset_healthy`: on every M2 chart, an 8 × 8 lattice whose descriptors the kernel decode stage wrote,
//!   every sample compared and within the tolerance, and `E₀` the f32 sum of the stored `K₀ + V₀`, bit for bit. Control:
//!   a dispatch scramble, each descriptor written nine samples on, disagrees.
//! - `agreement_preset_twin`: the colour view against its CPU twin (`agreement::colour` over `agreement::ratio`) on a
//!   lattice with non-finite stored operands (`V₀ = ±∞`, `K₀ = NaN`, each by its bits) and one stored `K₀` moved far
//!   off: the non-finite ones draw the hatch, the moved one red, the rest green, each pixel the twin's colour. Control:
//!   a twin that compares the non-finite samples instead of hatching them differs.
//! - `agreement_preset_stage_into_places`: `Scene::stage_into` writes sample `q`'s descriptor at sample
//!   `(q + shift) mod count`. Control: the placement read with another shift.
//! - `agreement_preset_gate_runs`: the `decode-agreement` gate through its trait on 4 × 4 lattices: healthy passes,
//!   a poisoned `V₀` is hatched and the input passes, a scramble fails, an input without its fields is refused.
//!   Control: the scramble expected to pass.

use std::path::{Path, PathBuf};

use render::presets::{agreement, DecodeChart};
use validation::decode_agreement::DecodeAgreementGate;
use validation::decode_preset::Scene;
use validation::gate::report::Verdict;
use validation::gpu::GpuHarness;
use validation::negative_control;

fn wgsl() -> String {
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    validation::decode_f32_eps::kernel_wgsl(&root).unwrap_or_else(|e| panic!("{e}"))
}

/// Every chart's lattice, descriptors written `shift` samples on: each sample compared, within the tolerance.
fn check_healthy(shift: u32) {
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let wgsl = wgsl();
    for chart in DecodeChart::ALL {
        let mut scene = Scene::lattice(chart, [8, 8], 3).unwrap();
        scene
            .stage_into(gpu.device(), gpu.queue(), &wgsl, shift)
            .unwrap();
        let words = scene.agreement(gpu.device(), gpu.queue()).unwrap();
        for (i, w) in (0u32..).zip(&words) {
            assert_eq!(w[1], 1, "{chart:?} sample {i}: not compared");
            let ratio = f32::from_bits(w[0]);
            assert!(
                ratio <= 1.0,
                "{chart:?} sample {i}: the fragment decode disagrees with the stage's, ratio {ratio}"
            );
            let stored = *scene.set.ic(i);
            assert_eq!(
                w[3],
                (stored.K_0 + stored.V_0).to_bits(),
                "{chart:?} sample {i}: E₀ is not the stored K₀ + V₀"
            );
        }
    }
}

#[test]
fn agreement_preset_healthy() {
    check_healthy(0);
}

negative_control!(
    agreement_preset_healthy,
    "a dispatch scramble must show as disagreement",
    expected = "disagrees with the stage's",
    check_healthy(9)
);

/// The colour view against its CPU twin; `honest` false makes the twin compare the non-finite samples instead.
fn check_twin(honest: bool) {
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let mut scene = Scene::lattice(DecodeChart::Latent, [8, 8], 3).unwrap();
    scene
        .stage_into(gpu.device(), gpu.queue(), &wgsl(), 0)
        .unwrap();
    let poisoned = [3u32, 20, 41];
    scene.set.ic(3).V_0 = f32::NEG_INFINITY;
    scene.set.ic(20).V_0 = f32::INFINITY;
    scene.set.ic(41).K_0 = f32::NAN;
    let moved = 50u32;
    scene.set.ic(moved).K_0 += 1.0;
    let words = scene.agreement(gpu.device(), gpu.queue()).unwrap();
    let image = scene
        .draw(gpu.device(), gpu.queue(), &agreement::preset())
        .unwrap();
    let grid = scene.context.grid;
    let (width, height) = scene.size();
    let (mut hatched, mut red, mut green) = (0, 0, 0);
    for y in 0..height {
        for x in 0..width {
            let q = grid.cell(x, y).quad;
            let w = words[q as usize];
            let stored = *scene.set.ic(q);
            let gpu_ratio = (w[1] != 0).then(|| f64::from(f32::from_bits(w[0])));
            // The twin's ratio from the same operands: the fragment's E (`K + V`, written whole) and the stored K₀, V₀.
            let twin = agreement::ratio(0.0, f32::from_bits(w[2]), stored.K_0, stored.V_0);
            assert_eq!(
                twin.is_some(),
                gpu_ratio.is_some(),
                "pixel ({x}, {y}): the twin and the GPU differ on whether the operands are finite"
            );
            if let (Some(t), Some(g)) = (twin, gpu_ratio) {
                // E₀ summed at f32 on the GPU and at f64 by the twin: at most half an ulp of E₀ apart, under one eps
                // of the scale, 1/FACTOR of the tolerance.
                assert!(
                    (t - g).abs() <= 1.0 / f64::from(agreement::FACTOR) + 1e-6 * t,
                    "sample {q}: the twin's ratio {t} is not the GPU's {g}"
                );
            }
            let expected = if honest {
                gpu_ratio
            } else {
                Some(gpu_ratio.unwrap_or(0.0))
            };
            let want = agreement::colour(expected, [f64::from(x) + 0.5, f64::from(y) + 0.5]);
            let got = image.words(x, y);
            for k in 0..3 {
                let g = f64::from(f32::from_bits(got[k]));
                assert!(
                    (g - want[k]).abs() <= 1e-5,
                    "pixel ({x}, {y}) sample {q}: channel {k} is {g}, the twin's {}: the hatch or the ramp differs",
                    want[k]
                );
            }
            match expected {
                None => hatched += 1,
                Some(r) if r > 1.0 => red += 1,
                Some(_) => green += 1,
            }
            assert_eq!(
                expected.is_none(),
                poisoned.contains(&q),
                "sample {q}: hatched where its operands are finite, or compared where one is not"
            );
            if q == moved {
                assert!(expected.is_some_and(|r| r > 1.0), "the moved K₀ agrees");
            }
        }
    }
    assert_eq!((hatched, red, green), (3, 1, 60));
}

#[test]
fn agreement_preset_twin() {
    check_twin(true);
}

negative_control!(
    agreement_preset_twin,
    "a twin that compares non-finite operands instead of hatching them must differ",
    expected = "the hatch or the ramp differs",
    check_twin(false)
);

/// The decode stage's descriptors land where `stage_into` says: sample `q`'s at `(q + shift) mod count`; `claimed` is
/// the shift the placement is checked against (the control gives another).
fn check_placement(shift: u32, claimed: u32) {
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let mut scene = Scene::lattice(DecodeChart::ShapeSphere, [3, 2], 2).unwrap();
    let outs = scene
        .stage_into(gpu.device(), gpu.queue(), &wgsl(), shift)
        .unwrap();
    assert_eq!(outs.len(), 6);
    for (q, o) in (0u32..).zip(&outs) {
        let at = (q + claimed) % 6;
        assert_eq!(
            *scene.set.ic(at),
            o.descriptor,
            "sample {q}'s descriptor is not at sample {at}"
        );
    }
}

#[test]
fn agreement_preset_stage_into_places() {
    check_placement(4, 4);
}

negative_control!(
    agreement_preset_stage_into_places,
    "a placement read with another shift must be found",
    expected = "descriptor is not at sample",
    check_placement(4, 2)
);

/// The gate through its trait on small inputs: a healthy lattice passes, a scramble fails, a poisoned sample is hatched;
/// `scrambled` is the verdict the scramble is expected to give (the control expects a pass).
fn check_gate(scrambled: Verdict) {
    use validation::gate::Gate;
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = validation::gate::config(&root, "decode-agreement").unwrap();
    let gate = DecodeAgreementGate;
    assert_eq!(gate.name(), "decode-agreement");
    let run = |json: serde_json::Value| gate.run(&config, "t.json", &json).unwrap();
    let healthy = run(serde_json::json!({"description": "h", "side": [4, 4], "depth": 2}));
    assert_eq!(
        healthy.verdict,
        Verdict::Pass,
        "{:?}",
        healthy.negative_results
    );
    assert_eq!(healthy.gate, "decode-agreement");
    assert_eq!(healthy.negative_results.len(), 7);
    let poisoned =
        run(serde_json::json!({"description": "p", "side": [4, 4], "depth": 2, "poison": [5]}));
    assert_eq!(
        poisoned.verdict,
        Verdict::Pass,
        "{:?}",
        poisoned.negative_results
    );
    assert!(
        poisoned.negative_results[0].ends_with("0 with the hatch misplaced; 1 hatched"),
        "{:?}",
        poisoned.negative_results
    );
    assert!(poisoned
        .negative_results
        .iter()
        .any(|l| l.contains("samples [5]")));
    let scramble =
        run(serde_json::json!({"description": "s", "side": [4, 4], "depth": 2, "shift": 5}));
    assert_eq!(scramble.verdict, scrambled, "the scramble's verdict");
    assert!(scramble
        .negative_results
        .iter()
        .any(|l| l.contains("5 samples on")));
    let refused = gate.run(&config, "t.json", &serde_json::json!({"side": [4, 4]}));
    assert!(refused.unwrap_err().starts_with("refused: "));
}

#[test]
fn agreement_preset_gate_runs() {
    check_gate(Verdict::Fail);
}

negative_control!(
    agreement_preset_gate_runs,
    "a scramble taken for a pass must be found",
    expected = "the scramble's verdict",
    check_gate(Verdict::Pass)
);
