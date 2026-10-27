//! QA tests for TASK-M2-25 that need the built kernel (`target/spirv/kernel.wgsl`, written by `cargo xtask
//! build-kernel`), so they run in CI's `gpu-kernel` job: the agreement preset against the kernel decode stage's own
//! descriptors, as the payload stores them. Written from the requirement, not from the implementation.
//!
//! - REQ-COL-006 (`qa_m225_two_swapped_descriptors_show_where_they_are`): a dispatch scramble shows as *spatial*
//!   disagreement: with the kernel decode stage's descriptors written in place but two samples' exchanged, exactly
//!   those two pixels draw the preset's disagreement (red) and every other pixel its agreement (green), on every chart.
//!   Control: no exchange, so no pixel disagrees.
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
