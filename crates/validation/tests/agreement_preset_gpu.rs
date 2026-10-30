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
    let outs = scene
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
            // The twin's ratio from the same operands: the fragment's E (`K + V`, written whole) and the stored K₀, V₀,
            // with the decode stage's (m, r) at the sample's point for the fragment's own, which the words don't carry.
            let own = outs[q as usize].ic;
            let twin = agreement::ratio(
                0.0,
                f32::from_bits(w[2]),
                stored.K_0,
                stored.V_0,
                own.m,
                own.r,
            );
            assert_eq!(
                twin.is_some(),
                gpu_ratio.is_some(),
                "pixel ({x}, {y}): the twin and the GPU differ on whether the operands are finite"
            );
            if let (Some(t), Some(g)) = (twin, gpu_ratio) {
                // E₀ summed at f32 on the GPU and at f64 by the twin: at most half an ulp of E₀ apart, under one eps
                // of the scale, 1/FACTOR of the tolerance; the scale's conditioning term from the stage's (m, r), a few
                // ulps from the fragment's, within 10⁻⁵ of the ratio.
                assert!(
                    (t - g).abs() <= 1.0 / f64::from(agreement::FACTOR) + 1e-5 * t,
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

/// One conditioning case: the masses and positions, as f32.
type Case = ([f32; 3], [[f32; 2]; 3]);

/// The conditioning cases: ordinary configurations, a zero separation (the pole), a pair under the floor, positions all
/// at the origin, at `2⁻⁸` and at `2⁶⁰` (under colour_composition §6's `2⁶²`), a zero mass, and a non-finite mass or position in each place.
fn conditioning_cases() -> Vec<(Case, bool)> {
    let third = 1.0f32 / 3.0;
    let tri = [[0.9f32, 0.1], [-0.4, 0.7], [-0.5, -0.8]];
    let mut out = vec![
        (([third; 3], tri), true),
        (
            ([0.5, 0.3, 0.2], [[1.0, 0.0], [-1.0, 0.25], [0.0, -0.75]]),
            true,
        ),
        (([third; 3], [[0.5, 0.5], [0.5, 0.5], [-1.0, -1.0]]), true),
        (
            ([third; 3], [[0.5, 0.5], [0.5 + 1e-9, 0.5], [-1.0, -1.0]]),
            true,
        ),
        (([third; 3], [[0.0, 0.0]; 3]), true),
        (
            (
                [third; 3],
                [
                    [0.003_906_25, 0.0],
                    [0.003_906_25, 1e-12],
                    [-0.007_812_5, 0.0],
                ],
            ),
            true,
        ),
        (
            (
                [0.5, 0.5, 0.0],
                [
                    [1.1529215e18, 0.0],
                    [1.1529215e18, 1.0],
                    [-1.1529215e18, 0.0],
                ],
            ),
            true,
        ),
        (
            ([0.5, 0.5, 0.0], [[0.7, 0.0], [-0.7, 0.0], [0.7, 0.0]]),
            true,
        ),
    ];
    for bad in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        for k in 0..3 {
            let mut m = [third; 3];
            m[k] = bad;
            out.push(((m, tri), false));
        }
        for k in 0..6 {
            let mut r = tri;
            r[k / 2][k % 2] = bad;
            out.push((([third; 3], r), false));
        }
    }
    out
}

/// A view returning, per pixel (its column the case), `agreement_conditioning`'s bits, `agreement_finite_mr`, and
/// `agreement_of`'s valid word and ratio over a FragDecode of the case's `(m, r)` with `K = 0.25`, `V = −0.5` against
/// `K₀ = 0.25`, `V₀ = −0.5 + 2⁻²⁰`.
fn conditioning_view(cases: &[(Case, bool)]) -> String {
    let lit = |x: f32| format!("bitcast<f32>({}u)", x.to_bits());
    let mut arms = String::new();
    for (i, ((m, r), _)) in cases.iter().enumerate() {
        arms.push_str(&format!(
            "        case {i}u: {{ d.m = vec3<f32>({}, {}, {}); d.r[0] = vec2<f32>({}, {}); \
             d.r[1] = vec2<f32>({}, {}); d.r[2] = vec2<f32>({}, {}); }}\n",
            lit(m[0]),
            lit(m[1]),
            lit(m[2]),
            lit(r[0][0]),
            lit(r[0][1]),
            lit(r[1][0]),
            lit(r[1][1]),
            lit(r[2][0]),
            lit(r[2][1])
        ));
    }
    format!(
        "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {{\n    var d: FragDecode;\n    \
         switch (u32(l.screen.pixel.x)) {{\n{arms}        default: {{}}\n    }}\n    \
         d.d.K_0 = {k}u;\n    d.d.V_0 = {v}u;\n    \
         let a = agreement_of(d, {k}u, {v0}u);\n    \
         let ok = agreement_finite_mr(d.m, d.r);\n    \
         let c = select(0.0, agreement_conditioning(d.m, d.r), ok);\n    \
         return vec4<u32>(bitcast<u32>(c), select(0u, 1u, ok), select(0u, 1u, a.valid), bitcast<u32>(a.ratio));\n}}\n",
        k = 0.25f32.to_bits(),
        v = (-0.5f32).to_bits(),
        v0 = (-0.5f32 + 2f32.powi(-20)).to_bits(),
    )
}

/// The WGSL conditioning term and `agreement_of` against their CPU twins, case by case: the term within 10⁻⁵ of the
/// twin's and finite wherever `(m, r)` is, the validity `(m, r)`'s finiteness by its bits, and the ratio the twin's;
/// `twin` is the CPU twin the GPU is held to (the control gives one that drops the floor).
fn check_conditioning(twin: fn([f32; 3], [[f32; 2]; 3]) -> f64) {
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let cases = conditioning_cases();
    let n = cases.len() as u32;
    let scene = Scene::lattice(DecodeChart::Latent, [n, 1], 8).unwrap();
    let words = agreement::words();
    let text = words.view();
    let prelude = &text[..text.rfind("fn view(").expect("the words preset has a view")];
    let image = scene
        .draw_view(
            gpu.device(),
            gpu.queue(),
            &format!("{prelude}{}", conditioning_view(&cases)),
            render::bind::ViewOutput::Words,
        )
        .unwrap();
    for (i, ((m, r), finite)) in cases.iter().enumerate() {
        let w = image.words(i as u32, 0);
        assert_eq!(
            w[1] != 0,
            *finite,
            "case {i}: (m, r)'s finiteness read wrong"
        );
        assert_eq!(
            w[2] != 0,
            *finite,
            "case {i}: agreement_of's validity is not (m, r)'s"
        );
        if !finite {
            continue;
        }
        let got = f32::from_bits(w[0]);
        assert!(
            got.is_finite() && got >= 0.0,
            "case {i}: the term {got} is not finite"
        );
        let want = twin(*m, *r);
        // Relative to the twin's, or exactly 0 with it; a non-finite twin never matches (its quotient is NaN).
        let close = if want == 0.0 {
            got == 0.0
        } else {
            ((f64::from(got) - want) / want).abs() <= 1e-5
        };
        assert!(
            close,
            "case {i}: the WGSL term {got} is not the twin's {want}"
        );
        let ratio = agreement::ratio(0.25, -0.5, 0.25, -0.5 + 2f32.powi(-20), *m, *r).unwrap();
        let g = f64::from(f32::from_bits(w[3]));
        assert!(
            (g - ratio).abs() <= 1e-5 * ratio,
            "case {i}: agreement_of's ratio {g} is not the twin's {ratio}"
        );
    }
}

#[test]
fn agreement_preset_conditioning() {
    check_conditioning(agreement::conditioning);
}

/// The conditioning term without its floor, `Σ mᵢmⱼ·wᵢⱼ/‖rᵢ − rⱼ‖∞²`: infinite at the pole.
#[cfg(feature = "controls")]
fn unfloored(m: [f32; 3], r: [[f32; 2]; 3]) -> f64 {
    let n = |v: [f64; 2]| v[0].abs().max(v[1].abs());
    let r = r.map(|p| p.map(f64::from));
    let pair = |i: usize, j: usize| {
        let w = n(r[i]) + n(r[j]);
        let s = n([r[i][0] - r[j][0], r[i][1] - r[j][1]]);
        f64::from(m[i]) * f64::from(m[j]) * w / (s * s)
    };
    pair(0, 1) + pair(0, 2) + pair(1, 2)
}

negative_control!(
    agreement_preset_conditioning,
    "a twin without the separation's floor must differ at the pole",
    expected = "is not the twin's",
    check_conditioning(unfloored)
);

/// The gate through its trait on a collision window against the exact value (`"stored": "f64"`): `shape_sphere`'s
/// binary collision at `u = 0.844518` on its equator, 4 × 4 cells at depths 13 and 19, passes by the conditioned scale
/// and fails by the per-operand one, on every backend; `form` is the form expected to fail (the control expects the
/// conditioned one to).
fn check_window_gate(form: &str) {
    use validation::gate::Gate;
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = validation::gate::config(&root, "decode-agreement").unwrap();
    let window = serde_json::json!([{"chart": "shape_sphere", "centre": [0.844518, 0.5], "depths": [13, 19]}]);
    let verdict = |f: &str| {
        let input = serde_json::json!({"description": "w", "side": [4, 4], "stored": "f64", "windows": window, "form": f});
        DecodeAgreementGate.run(&config, "w.json", &input).unwrap()
    };
    let failing = verdict(form);
    assert_eq!(
        failing.verdict,
        Verdict::Fail,
        "the {form} form passes beside the collision: {:?}",
        failing.scatter
    );
    let other = if form == "per_operand" {
        "conditioned"
    } else {
        "per_operand"
    };
    let passing = verdict(other);
    assert_eq!(
        passing.verdict,
        Verdict::Pass,
        "{:?}",
        passing.negative_results
    );
    assert!(
        passing.scatter.iter().any(
            |l| l.starts_with("window 0, shape_sphere at (0.844518, 0.5)")
                && l.contains("by depth: 13: ")
        ),
        "{:?}",
        passing.scatter
    );
}

#[test]
fn agreement_preset_window_gate() {
    check_window_gate("per_operand");
}

negative_control!(
    agreement_preset_window_gate,
    "the conditioned form failing beside the collision must be found",
    expected = "form passes beside the collision",
    check_window_gate("conditioned")
);

/// `measure` reads the store the input names: each sample's per-operand scale is formed from the decode stage's `K₀`
/// and `V₀` under `"stored": "stage"` and from the f64 decode's under `"f64"`, and the two differ beside the collision;
/// `swap` names the stores the other way round (the control).
fn check_measure_store(swap: bool) {
    use validation::decode_agreement::{measure, runs, store_exact, Input};
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let wgsl = wgsl();
    let parse = |stored: &str| -> Input {
        serde_json::from_value(serde_json::json!({
            "description": "m", "side": [4, 4], "stored": stored,
            "windows": [{"chart": "shape_sphere", "centre": [0.844518, 0.5], "depths": [19]}]
        }))
        .unwrap()
    };
    let run = runs(&parse("stage")).unwrap().remove(0);
    let scale = |k: f32, v: f32| {
        f64::from(f32::EPSILON) * (f64::from(k).abs().max(1.0) + f64::from(v).abs().max(1.0))
    };
    let mut staged = run.scene([4, 4]).unwrap();
    staged
        .stage_into(gpu.device(), gpu.queue(), &wgsl, 0)
        .unwrap();
    let mut exact = run.scene([4, 4]).unwrap();
    store_exact(&mut exact).unwrap();
    let (stage_name, f64_name) = if swap {
        ("f64", "stage")
    } else {
        ("stage", "f64")
    };
    let scales = |scene: &mut Scene| -> Vec<f64> {
        (0..16u32)
            .map(|q| {
                let d = *scene.set.ic(q);
                scale(d.K_0, d.V_0)
            })
            .collect()
    };
    let (want_stage, want_f64) = (scales(&mut staged), scales(&mut exact));
    for (name, want) in [(stage_name, &want_stage), (f64_name, &want_f64)] {
        let samples = measure(&gpu, &wgsl, &run, &parse(name)).unwrap();
        for (q, s) in samples.iter().enumerate() {
            assert_eq!(
                s.scale, want[q],
                "sample {q}: the {name} input's scale is not formed from its store"
            );
        }
    }
    let differ = want_stage
        .iter()
        .zip(&want_f64)
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        differ > 0,
        "the two stores agree everywhere: the test cannot tell them apart"
    );
}

#[test]
fn agreement_preset_measure_store() {
    check_measure_store(false);
}

negative_control!(
    agreement_preset_measure_store,
    "the stores read the other way round must be found",
    expected = "is not formed from its store",
    check_measure_store(true)
);
