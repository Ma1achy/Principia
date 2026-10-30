//! QA tests for TASK-M2-29 on the GPU, fragment only (no built kernel: the payload's `K₀` and `V₀` are written from
//! the f64 `decodeOnly()`, `validation::decode_agreement::store_exact`), written from REQ-COL-060 and colour_composition
//! §6 ("The tolerance's scale", R-72; RQ-623, decided per R-369, option (b), and its four conditions), not from the
//! implementation. Tier N (parity contract §2): the fragment's f32 against an f64 reference or a CPU twin, within a
//! stated tolerance.
//!
//! - `qa_m229_collision_windows_flat` (conditions 2 and 4; REQ-COL-060, PIT-3): the committed gate input
//!   `fixtures/gates/decode-agreement/collision_f64.json`'s windows, `shape_sphere`'s three binary collisions zoomed
//!   from depth 11 to 23: the agreement preset draws every compared sample within REQ-COL-060's factor at every depth,
//!   and each window's worst factor is flat under refinement, varying over the depths by less than a `1/d` quantity
//!   grows in one refinement step (depths two apart: cells 4× smaller). Control: the per-operand form,
//!   `max(|K₀|, 1) + max(|V₀|, 1)` alone, formed here from the preset's own `E` and `E₀` words and the stored `K₀`,
//!   `V₀`, must fail the same input.
//! - `qa_m229_scramble_still_fails` (condition 4; REQ-COL-006's spatial disagreement): with the payload's descriptors
//!   written 37 samples on, every M2 chart's lattice, and every collision window at depth 11, shows disagreement under
//!   the conditioned scale: the conditioning term does not swallow a dispatch fault, even beside a collision.
//!   Control: no scramble, so no chart disagrees.
//! - `qa_m229_wgsl_term_finite_and_twinned` (condition 1; R-436): the preset's WGSL term and comparison, fed the same
//!   `(m, r)` as the CPU twin: a binary halved down to and past coincidence, at position scales `2⁻⁸` to `2⁶⁰`, and
//!   fuzzed configurations. The WGSL term is finite and at most `3·2⁴⁴` at every case, the pole included, within
//!   `10⁻⁵` relative of `agreement::conditioning` (parity contract §4's starting figure for a decode, R-85), and the
//!   ratio within `10⁻⁵` of `agreement::ratio`; a non-finite mass or position makes the sample invalid. Control: the
//!   preset's own WGSL with the separation's floor removed must draw a non-finite term at the pole.
//!
//! **Domain** (pitfalls §10): one decode per sample, no trajectory; each chart at the harness's fixed parameters.

use std::path::{Path, PathBuf};

use render::presets::{agreement, DecodeChart};
use validation::decode_agreement::{runs, store_exact, Input, Run};
use validation::decode_preset::Scene;
use validation::gpu::GpuHarness;
use validation::negative_control;

const EPS: f64 = f32::EPSILON as f64;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// The committed gate input `name`, its gate-runner-only `expected` field dropped.
fn gate_input(name: &str) -> Input {
    let path = root().join("fixtures/gates/decode-agreement").join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    value
        .as_object_mut()
        .expect("an input is an object")
        .remove("expected");
    serde_json::from_value(value).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// One drawn sample: the preset's ratio when valid, and the per-operand factor from its `E`, `E₀` words and the stored
/// `K₀`, `V₀`.
struct Drawn {
    ratio: Option<f64>,
    per_operand: f64,
}

/// Draws `scene` with the f64 decode's `K₀` and `V₀` stored, written `shift` samples on.
fn draw(gpu: &GpuHarness, scene: &mut Scene, shift: u32) -> Vec<Drawn> {
    store_exact(scene).unwrap();
    let n = scene.context.grid.quad_count();
    let grid = scene.context.grid;
    if shift != 0 {
        let ds: Vec<_> = (0..n)
            .map(|q| *scene.set.ic(grid.sample_index(q, 0, 0)))
            .collect();
        for q in 0..n {
            *scene.set.ic(grid.sample_index((q + shift) % n, 0, 0)) = ds[q as usize];
        }
    }
    let words = scene.agreement(gpu.device(), gpu.queue()).unwrap();
    (0..n)
        .zip(&words)
        .map(|(q, w)| {
            let d = *scene.set.ic(grid.sample_index(q, 0, 0));
            let e = f64::from(f32::from_bits(w[2]));
            let e0 = f64::from(f32::from_bits(w[3]));
            let scale = EPS * (f64::from(d.K_0).abs().max(1.0) + f64::from(d.V_0).abs().max(1.0));
            Drawn {
                ratio: (w[1] != 0).then(|| f64::from(f32::from_bits(w[0]))),
                per_operand: (e - e0).abs() / scale,
            }
        })
        .collect()
}

// ── Conditions 2 and 4: the collision windows, flat under refinement ───────────────────────────────────────────────

/// The windows, each sample judged by the preset's own ratio, over §6's conditioning-aware scale, where `conditioned`,
/// else by the per-operand scale alone.
fn check_windows(conditioned: bool) {
    let form = if conditioned {
        "conditioned"
    } else {
        "per-operand"
    };
    let gpu = gpu();
    let input = gate_input("collision_f64.json");
    let factor = f64::from(agreement::FACTOR);
    let all = runs(&input).unwrap();
    assert_eq!(
        all.len(),
        21,
        "the collision input is not 3 windows of 7 depths"
    );
    for k in 0..input.windows.len() {
        let mine: Vec<&Run> = all
            .iter()
            .filter(|r| matches!(r.window, Some((i, _, _)) if i == k))
            .collect();
        let mut worst = Vec::new();
        for run in &mine {
            let mut scene = run.scene(input.side).unwrap();
            let drawn = draw(&gpu, &mut scene, 0);
            let compared: Vec<f64> = drawn
                .iter()
                .filter_map(|d| {
                    d.ratio.map(|r| {
                        if conditioned {
                            r * factor
                        } else {
                            d.per_operand
                        }
                    })
                })
                .collect();
            assert!(
                compared.len() * 2 >= drawn.len(),
                "{}: only {} of {} samples compared",
                run.label(),
                compared.len(),
                drawn.len()
            );
            let w = compared.iter().copied().fold(0.0, f64::max);
            assert!(
                w <= factor,
                "{form}: {} needs {w} eps, which exceeds the tolerance's {factor}",
                run.label()
            );
            worst.push((run.depth, w));
        }
        println!("{form}: window {k}, worst factor by depth {worst:.3?}");
        let hi = worst.iter().map(|w| w.1).fold(0.0, f64::max);
        let lo = worst.iter().map(|w| w.1).fold(f64::INFINITY, f64::min);
        assert!(
            hi <= 4.0 * lo,
            "{form}: window {k}'s worst factor is not flat under refinement: {worst:?}"
        );
    }
}

#[test]
fn qa_m229_collision_windows_flat() {
    check_windows(true);
}

negative_control!(
    qa_m229_collision_windows_flat,
    "the per-operand form must fail the collision input",
    expected = "which exceeds the tolerance",
    check_windows(false)
);

// ── Condition 4: the scramble still fails ───────────────────────────────────────────────────────────────────────────

fn check_scramble(shift: u32) {
    let gpu = gpu();
    for chart in DecodeChart::ALL {
        let mut scene = Scene::lattice(chart, [32, 32], 5).unwrap();
        let drawn = draw(&gpu, &mut scene, shift);
        let disagree = drawn
            .iter()
            .filter(|d| d.ratio.is_some_and(|r| r > 1.0))
            .count();
        println!("{}: {disagree} of 1024 disagree", chart.section());
        assert!(
            disagree > 0,
            "the scramble on {} shows no disagreement",
            chart.section()
        );
    }
    let input = gate_input("collision_f64.json");
    for run in runs(&input).unwrap().iter().filter(|r| r.depth == 11) {
        let mut scene = run.scene(input.side).unwrap();
        let drawn = draw(&gpu, &mut scene, shift);
        let disagree = drawn
            .iter()
            .filter(|d| d.ratio.is_some_and(|r| r > 1.0))
            .count();
        println!("{}: {disagree} disagree", run.label());
        assert!(
            disagree > 0,
            "the scramble on {} shows no disagreement",
            run.label()
        );
    }
}

#[test]
fn qa_m229_scramble_still_fails() {
    check_scramble(37);
}

negative_control!(
    qa_m229_scramble_still_fails,
    "with no scramble, no chart may disagree",
    expected = "shows no disagreement",
    check_scramble(0)
);

// ── Condition 1: the WGSL term, finite at the pole, and its CPU twin ────────────────────────────────────────────────

/// One case: masses and positions, and whether they are finite.
type Case = ([f32; 3], [[f32; 2]; 3], bool);

/// The cases: a binary halved from separation 1 to `2⁻⁴⁰` and then coincident, at position scales `2⁻⁸`, 1, `2²⁰` and
/// `2⁶⁰`; fuzzed configurations; and a non-finite mass or position.
fn cases() -> Vec<Case> {
    let m = [0.45f32, 0.35, 0.2];
    let mut out = Vec::new();
    for scale in [2f32.powi(-8), 1.0, 2f32.powi(20), 2f32.powi(60)] {
        let base = [0.75 * scale, -0.3 * scale];
        let third = [-0.6 * scale, 0.9 * scale];
        for h in (0..=40).step_by(4) {
            let d = scale * 2f32.powi(-h);
            out.push((m, [base, [base[0] + d, base[1] - 0.5 * d], third], true));
        }
        out.push((m, [base, base, third], true));
    }
    // Every body at the origin, and a zero mass at a coincidence.
    out.push((m, [[0.0; 2]; 3], true));
    out.push(([0.5, 0.5, 0.0], [[0.3, 0.1], [0.4, 0.2], [0.3, 0.1]], true));
    let mut s: u64 = 0x2029;
    let mut unit = || {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((s >> 40) as f32) / (1u64 << 24) as f32
    };
    for _ in 0..24 {
        let a = unit();
        let b = (1.0 - a) * unit();
        let r = [0; 3].map(|_| [4.0 * unit() - 2.0, 4.0 * unit() - 2.0]);
        out.push(([a, b, 1.0 - a - b], r, true));
    }
    let tri = [[0.9f32, 0.1], [-0.4, 0.7], [-0.5, -0.8]];
    for bad in [f32::INFINITY, f32::NAN] {
        out.push(([0.5, bad, 0.2], tri, false));
        out.push((m, [tri[0], [tri[1][0], bad], tri[2]], false));
    }
    out
}

/// A words view: per pixel (its column the case), the term's bits, `agreement_of`'s valid word and its ratio's bits,
/// for `K = 0.25`, `V = −0.5` against `K₀ = 0.25`, `V₀ = −0.5 + 2⁻²⁰`.
fn view(cases: &[Case]) -> String {
    let lit = |x: f32| format!("bitcast<f32>({}u)", x.to_bits());
    let mut arms = String::new();
    for (i, (m, r, _)) in cases.iter().enumerate() {
        arms.push_str(&format!(
            "        case {i}u: {{ d.m = vec3<f32>({}, {}, {}); d.r[0] = vec2<f32>({}, {}); d.r[1] = vec2<f32>({}, {}); \
             d.r[2] = vec2<f32>({}, {}); }}\n",
            lit(m[0]),
            lit(m[1]),
            lit(m[2]),
            lit(r[0][0]),
            lit(r[0][1]),
            lit(r[1][0]),
            lit(r[1][1]),
            lit(r[2][0]),
            lit(r[2][1]),
        ));
    }
    format!(
        "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {{
    var d: FragDecode;
    switch (u32(l.screen.pixel.x)) {{
{arms}        default: {{}}
    }}
    d.d.K_0 = {k}u;
    d.d.V_0 = {v}u;
    let a = agreement_of(d, {k}u, {v0}u);
    return vec4<u32>(bitcast<u32>(agreement_conditioning(d.m, d.r)), select(0u, 1u, a.valid), bitcast<u32>(a.ratio), 0u);
}}
",
        k = 0.25f32.to_bits(),
        v = (-0.5f32).to_bits(),
        v0 = (-0.5f32 + 2f32.powi(-20)).to_bits(),
    )
}

/// The floor in the preset's WGSL, and the line without it.
const FLOORED: &str = "let s = max(agreement_norm(ri - rj), AGREEMENT_EPS * max(w, 1.0));";
const UNFLOORED: &str = "let s = agreement_norm(ri - rj);";

fn check_wgsl(floor: bool) {
    let gpu = gpu();
    let cases = cases();
    let scene = Scene::lattice(DecodeChart::Latent, [cases.len() as u32, 1], 8).unwrap();
    let words = agreement::words();
    let text = words.view();
    let mut prelude =
        text[..text.rfind("fn view(").expect("the words preset has a view")].to_owned();
    assert!(
        prelude.contains(FLOORED),
        "the preset's WGSL has no separation floor to remove"
    );
    if !floor {
        prelude = prelude.replace(FLOORED, UNFLOORED);
    }
    let image = scene
        .draw_view(
            gpu.device(),
            gpu.queue(),
            &format!("{prelude}{}", view(&cases)),
            render::bind::ViewOutput::Words,
        )
        .unwrap();
    for (i, (m, r, finite)) in cases.iter().enumerate() {
        let w = image.words(i as u32, 0);
        assert_eq!(
            w[1] != 0,
            *finite,
            "case {i} ({m:?}, {r:?}): validity is not (m, r)'s finiteness"
        );
        if !finite {
            continue;
        }
        let got = f64::from(f32::from_bits(w[0]));
        assert!(
            got.is_finite() && (0.0..=3.0 * 2f64.powi(44)).contains(&got),
            "case {i} ({m:?}, {r:?}): the WGSL term {got} is not finite and within 3·2⁴⁴"
        );
    }
    // The twin, once every term is shown finite.
    for (i, (m, r, finite)) in cases.iter().enumerate() {
        if !finite {
            continue;
        }
        let w = image.words(i as u32, 0);
        let got = f64::from(f32::from_bits(w[0]));
        let want = agreement::conditioning(*m, *r);
        assert!(
            (got - want).abs() <= 1e-5 * want,
            "case {i} ({m:?}, {r:?}): the WGSL term {got} is not the twin's {want}"
        );
        let ratio = agreement::ratio(0.25, -0.5, 0.25, -0.5 + 2f32.powi(-20), *m, *r).unwrap();
        let g = f64::from(f32::from_bits(w[2]));
        assert!(
            (g - ratio).abs() <= 1e-5 * ratio,
            "case {i}: the WGSL ratio {g} is not the twin's {ratio}"
        );
    }
}

#[test]
fn qa_m229_wgsl_term_finite_and_twinned() {
    check_wgsl(true);
}

negative_control!(
    qa_m229_wgsl_term_finite_and_twinned,
    "the WGSL term without the separation's floor must be caught non-finite at the pole",
    expected = "is not finite and within",
    check_wgsl(false)
);
