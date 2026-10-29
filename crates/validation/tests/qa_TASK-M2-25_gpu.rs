//! QA tests for TASK-M2-25 that need the built kernel (`target/spirv/kernel.wgsl`, written by `cargo xtask
//! build-kernel`), so they run in CI's `gpu-kernel` job: the agreement preset against the kernel decode stage's own
//! descriptors, as the payload stores them. Written from the requirement, not from the implementation.
//!
//! - REQ-COL-006 (`qa_m225_two_swapped_descriptors_show_where_they_are`): a dispatch scramble shows as *spatial*
//!   disagreement: with the kernel decode stage's descriptors written in place but two samples' exchanged, exactly
//!   those two pixels draw the preset's disagreement (red) and every other pixel its agreement (green), on every chart.
//!   Control: no exchange, so no pixel disagrees.
//! - REQ-COL-006, R-628 (`qa_m225_kernel_rho_angle_at_the_lambda_pole`): the kernel decode stage (the built kernel's
//!   WGSL, the third backend of the one source) on the shape sphere's exponential map at `(½, ½)`, where the point is
//!   `n₀`: at the exact `λ̃ = 0` pole `n₀ = (+1, ±0, ±0)`, every sign of the zeros at `φ_f = 0` and `+0`'s at
//!   `φ_f ∈ {0.3, 2}`, the stored `rho_angle` is `+0`'s bits (dd_decoder §3.6: `Φ`'s operands are `±0` there and the
//!   value is the constant 0, on every backend); near it, in the same dispatch, at points moving `n₀` by `ε` along
//!   `y` or `z`, both signs, `|ε|` from `10⁻⁴` down to `6·10⁻¹²` (the map's extent `10⁻⁴`), it is within REQ-DEC-043's
//!   factor of the f64 angle of `Φ`'s vectors at the same point, a whole turn apart counted as none. Control: the angle
//!   read from the stage's own stored positions (the form R-628 replaced), held to the same checks.
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; each chart at the harness's fixed parameters
//! (`validation::decode_preset::uniforms`), inside its square.

use std::path::{Path, PathBuf};

use render::presets::{agreement, DecodeChart};
use validation::decode_preset::Scene;
use validation::gpu::GpuHarness;
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

fn kernel_wgsl() -> String {
    validation::decode_f32_eps::kernel_wgsl(&root()).unwrap_or_else(|e| panic!("{e}"))
}

fn colour_at(image: &render::headless::Image, x: u32, y: u32) -> [f64; 3] {
    let w = image.words(x, y);
    [0, 1, 2].map(|k| f64::from(f32::from_bits(w[k])))
}

// ── REQ-COL-006: a scramble is spatial ──────────────────────────────────────────────────────────────────────────────

/// Is the agreement colour `c` the preset's disagreement (red, no green) or its agreement (green, no red)?
fn red(c: [f64; 3]) -> bool {
    c[0] >= 0.5 - 1e-6 && c[1] == 0.0 && c[2] == 0.0
}
fn green(c: [f64; 3]) -> bool {
    c[0] == 0.0 && c[1] >= 0.15 - 1e-6 && c[1] <= 0.5 + 1e-6 && c[2] == 0.0
}

fn check_swap(swap: bool) {
    let gpu = gpu();
    let wgsl = kernel_wgsl();
    for chart in DecodeChart::ALL {
        let mut scene = Scene::lattice(chart, [8, 8], 6).unwrap();
        scene
            .stage_into(gpu.device(), gpu.queue(), &wgsl, 0)
            .unwrap();
        let grid = scene.context.grid;
        let e0 = |s: &mut Scene, q: u32| {
            let d = *s.set.ic(grid.sample_index(q, 0, 0));
            f64::from(d.K_0) + f64::from(d.V_0)
        };
        // Sample 9 and the sample whose E₀ differs from it most: far beyond the tolerance, so the swap must show.
        let a = 9u32;
        let ea = e0(&mut scene, a);
        let b = (0..grid.quad_count())
            .max_by(|&p, &q| {
                let (dp_, dq) = (
                    (e0(&mut scene, p) - ea).abs(),
                    (e0(&mut scene, q) - ea).abs(),
                );
                dp_.total_cmp(&dq)
            })
            .unwrap();
        let eb = e0(&mut scene, b);
        assert!(
            (ea - eb).abs() > 1e-3 * ea.abs().max(1.0),
            "{chart:?}: no two samples' E₀ differ ({ea}, {eb})"
        );
        if swap {
            let (ia, ib) = (grid.sample_index(a, 0, 0), grid.sample_index(b, 0, 0));
            let da = *scene.set.ic(ia);
            let db = *scene.set.ic(ib);
            *scene.set.ic(ia) = db;
            *scene.set.ic(ib) = da;
        }
        let image = scene
            .draw(gpu.device(), gpu.queue(), &agreement::preset())
            .unwrap();
        let (width, height) = scene.size();
        let mut disagreeing = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let q = grid.cell(x, y).quad;
                let c = colour_at(&image, x, y);
                if red(c) {
                    disagreeing.push(q);
                } else {
                    assert!(
                        green(c),
                        "{chart:?} sample {q}: {c:?} is neither agreement nor disagreement"
                    );
                }
            }
        }
        disagreeing.sort_unstable();
        let mut want = vec![a, b];
        want.sort_unstable();
        assert_eq!(
            disagreeing, want,
            "{chart:?}: the disagreement is not where the descriptors were exchanged"
        );
    }
}

#[test]
fn qa_m225_two_swapped_descriptors_show_where_they_are() {
    check_swap(true);
}

negative_control!(
    qa_m225_two_swapped_descriptors_show_where_they_are,
    "with nothing exchanged no pixel disagrees, so the expected two are missing",
    expected = "the disagreement is not where the descriptors were exchanged",
    check_swap(false)
);

// ── REQ-COL-006, R-628: the kernel decode stage's rho_angle at the λ̃ = 0 pole ─────────────────────────────────────────

/// REQ-DEC-043's factor, in f32 eps.
fn factor() -> f64 {
    validation::gate::config(&root(), "decode-f32-eps")
        .expect("decode-f32-eps's gate.json")
        .threshold
        .value
}

/// The shape sphere on its exponential map about `n0`, frame `e₁ = y`, `e₂ = z`, extent `SCALE`, phase `phase`, else the
/// held parameters: at `(u, v)` its point is `n₀` moved by `ε ≈ (2u − 1)·SCALE` along `y` and `(2v − 1)·SCALE` along
/// `z`, and exactly `n₀` at `(½, ½)`, where the offset is 0.
fn sphere(n0: [f32; 3], phase: f32) -> kernel::chart::ShapeSphere<f32> {
    use kernel::stage::decode::UniformChart;
    let held = kernel::chart::ShapeSphere::<f32>::from_uniform(
        &validation::decode_preset::uniforms(DecodeChart::ShapeSphere).chart,
    );
    kernel::chart::ShapeSphere {
        map: kernel::chart::ShapeMap::Exponential,
        n0,
        e1: [0.0, 1.0, 0.0],
        e2: [0.0, 0.0, 1.0],
        scale: SCALE,
        phase,
        ..held
    }
}

/// The exponential map's extent: `|ε|` runs from `SCALE` down to `SCALE·6·10⁻⁸`, f32's closest `u` to ½.
const SCALE: f32 = 1e-4;

/// The points: `(½, ½)`, the pole, first; then `(½ ± δ, ½)` and `(½, ½ ± δ)`, `2δ` from 1 down to f32's step below ½.
fn points() -> Vec<[f32; 2]> {
    let mut out = vec![[0.5f32, 0.5]];
    let mut steps: Vec<f32> = (0..8).map(|k| 0.5 * 10f32.powi(-k)).collect();
    steps.push(0.5f32 - 0.5f32.next_down().max(0.0));
    for d in steps {
        for u in [0.5 + d, 0.5 - d] {
            out.push([u, 0.5]);
            out.push([0.5, u]);
        }
    }
    out
}

/// dd_decoder §3.6's `rho_angle` of `ρ̃ = r`, `λ̃ = l` at f64, case by case (`π̂ = π` at f64).
fn case_angle64(r: [f64; 2], l: [f64; 2]) -> f64 {
    let pi = std::f64::consts::PI;
    let cross = r[0] * l[1] - r[1] * l[0];
    let dot = r[0] * l[0] + r[1] * l[1];
    if cross == 0.0 {
        return if dot < 0.0 { pi } else { 0.0 };
    }
    let a = cross.atan2(dot);
    if a > pi || a <= -pi {
        pi
    } else {
        a
    }
}

/// The kernel decode stage's `rho_angle` at each of `c`'s [`points`], in one dispatch; with `positional`, the angle of
/// its stored positions' Jacobi vectors instead (the control).
fn kernel_angles(
    gpu: &GpuHarness,
    wgsl: &str,
    c: &kernel::chart::ShapeSphere<f32>,
    positional: bool,
) -> Vec<f32> {
    use kernel::stage::decode::{DecodeUniforms, UniformChart};
    let u = DecodeUniforms {
        chart: c.uniform(),
        ..validation::decode_preset::uniforms(DecodeChart::ShapeSphere)
    };
    let outs = validation::decode_preset::stage(
        gpu.device(),
        gpu.queue(),
        wgsl,
        DecodeChart::ShapeSphere,
        &u,
        &points(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    outs.iter()
        .map(|o| {
            if positional {
                kernel::decode::descriptor::describe(&o.ic).rho_angle
            } else {
                o.descriptor.rho_angle
            }
        })
        .collect()
}

/// The cases, one dispatch each: every sign of the pole's two zeros at `φ_f = 0`, and the `+0` pole at `φ_f = 0.3`
/// and `2`.
fn cases() -> Vec<kernel::chart::ShapeSphere<f32>> {
    let mut out: Vec<_> = [[0.0f32, 0.0], [0.0, -0.0], [-0.0, 0.0], [-0.0, -0.0]]
        .into_iter()
        .map(|z| sphere([1.0, z[0], z[1]], 0.0))
        .collect();
    out.push(sphere([1.0, 0.0, 0.0], 0.3));
    out.push(sphere([1.0, 0.0, 0.0], 2.0));
    out
}

fn check_kernel_pole(positional: bool) {
    use kernel::chart::ChartDecode;
    let gpu = gpu();
    let wgsl = kernel_wgsl();
    let f = factor();
    let tau = std::f64::consts::TAU;
    for c in cases() {
        let got = kernel_angles(&gpu, &wgsl, &c, positional);
        assert_eq!(
            got[0].to_bits(),
            0,
            "n₀ = {:?}, φ_f = {}: the kernel decode stage's rho_angle {:e} at the λ̃ = 0 pole is not the constant +0",
            c.n0,
            c.phase,
            got[0]
        );
        for (uv, g) in points().into_iter().zip(got).skip(1) {
            let k64 = c
                .decode_at::<kernel::generated::links::Defaults, f64>(
                    f64::from(uv[0]),
                    f64::from(uv[1]),
                )
                .config;
            let truth = case_angle64(k64.rho_t, k64.lambda_t);
            let diff = (f64::from(g) - truth).rem_euclid(tau);
            let diff = diff.min(tau - diff);
            let err = diff / (f64::from(f32::EPSILON) * truth.abs().max(1.0));
            assert!(
                err <= f,
                "n₀ = {:?}, φ_f = {} at {uv:?}: the kernel decode stage's rho_angle {g:e} is {err} eps from Φ's f64 \
                 angle {truth:e}, over REQ-DEC-043's {f}",
                c.n0,
                c.phase
            );
        }
    }
}

#[test]
fn qa_m225_kernel_rho_angle_at_the_lambda_pole() {
    check_kernel_pole(false);
}

negative_control!(
    qa_m225_kernel_rho_angle_at_the_lambda_pole,
    "the angle of the stage's stored positions, the form R-628 replaced, must be found at or near the pole",
    expected = "the kernel decode stage's rho_angle",
    check_kernel_pole(true)
);
