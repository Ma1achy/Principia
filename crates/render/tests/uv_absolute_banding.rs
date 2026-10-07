//! R-395's first half, the M1 gate's check of REQ-TOOL-019 (deep_zoom §1, "Banding, absolute and quad-local"): the
//! absolute UV coordinate `c + h·(2t − 1)`, formed in f32, does not band up to the deep-zoom switchover,
//! `ℓ_switch = 20` (R-90). The coordinate is the fragment's own, `ctx.chart.slice_uv` (R-394), drawn through the module
//! harness over one quad of N × N one-pixel tiles, whose pixel centres read `quad.uv = t = (i + ½)/N`.
//!
//! - `uv_absolute_no_banding_to_l_switch`: at N = 8 and N = 16, the named tiers' N (memory_tiers §4), for every depth
//!   ℓ from 1 to ℓ_switch, in the quad at the depth-ℓ cell nearest `u ≈ 0.6` on both axes (the binade [0.5, 1), where
//!   f32's ulp is coarsest), every row's and column's adjacent deltas pass REQ-TOOL-152's criterion
//!   (`render::coords::banded`, its bound proposed under R-71), or adjacent samples collapse to one coordinate, where
//!   R-90's switchover fires first. At these N the f32 sum is exact until it collapses, so the one collapse is N = 16
//!   at ℓ = 20 (R-395), and the test holds the sweep to it, so no collapse can stand in for a pass.
//!
//! Its negative control (R-176) is the non-dyadic fixture R-395 names, N = 6 at ℓ = 19, whose departure exceeds the
//! proposed bound before any collapse: the check can fail. It only shows that; RQ-258, open, asks what the bound should
//! be at a non-dyadic N, and the sweep covers N = 8 and N = 16 until it is ruled.

use render::bind::{self, Context, ViewOutput};
use render::coords::{banded, banding_departure, BANDING_BOUND};
use render::headless::{self, Draw, Target};
use render::raster::Grid;
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::synthetic::Synthetic;

/// The deep-zoom switchover's upper bound, `ℓ_switch = 20` (R-90).
const L_SWITCH: u32 = 20;

/// The named tiers' N (memory_tiers §4's table: 8 for Potato and Low, 16 from Medium up).
const TIER_N: [u32; 2] = [8, 16];

/// Where the sweep sits on each axis: in the binade [0.5, 1).
const NEAR: f64 = 0.6;

/// The (N, ℓ) at which adjacent samples collapse in the sweep (R-395: at N = 8 the sum is exact to ℓ = 20; at N = 16
/// to ℓ = 19, collapsing at ℓ = 20).
const COLLAPSES: [(u32, u32); 1] = [(16, 20)];

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

fn context(grid: Grid) -> Context {
    Context {
        dt_macro: 0.01,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 1000,
        z: [0.0; 8],
        grid,
        chart_id: 0,
        time: 0.0,
        ensemble_spread: 0.0,
        out_of_chart: false,
        valid_sample_count: grid.n * grid.n,
    }
}

/// The view: `ctx.chart.slice_uv`'s bits.
const VIEW: &str = "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    return vec4<u32>(bitcast<vec2<u32>>(l.chart.slice_uv), 0u, 0u);
}";

/// The depth-`depth` cell nearest [`NEAR`]: `⌊0.6 · 2^ℓ⌋`.
fn cell(depth: u32) -> u64 {
    (NEAR * (1u64 << depth) as f64) as u64
}

/// One quad of `n` × `n` samples at the depth-`depth` cell `(cell, cell)`: `ctx.chart.slice_uv` at each sample,
/// `[j][i]` for the sample in column `i` and row `j`, Y-up.
fn absolute(h: &GpuHarness, n: u32, depth: u32) -> Vec<Vec<[f32; 2]>> {
    let grid = Grid::new([1, 1], n, 0, 1).expect("the sweep's grid");
    let set = Synthetic::flat_at(grid, depth, [cell(depth); 2]);
    let ctx = context(grid);
    let bytes = set.bytes();
    let bound = bind::upload(h.device(), &bytes.payload(), &ctx);
    let module = bind::module(VIEW, ViewOutput::Words).unwrap_or_else(|e| panic!("{e}"));
    let (width, height) = grid.target();
    let image = headless::render(
        h.device(),
        h.queue(),
        &Draw {
            module: &module,
            entry: bind::ENTRY,
            layouts: &bound.layout_refs(),
            groups: &bound.group_refs(),
        },
        Target {
            width,
            height,
            format: wgpu::TextureFormat::Rgba32Uint,
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let mut out = vec![vec![[0.0; 2]; n as usize]; n as usize];
    for y in 0..height {
        for x in 0..width {
            let [i, j] = grid.cell(x, y).tile_xy;
            let w = image.words(x, y);
            out[j as usize][i as usize] = [f32::from_bits(w[0]), f32::from_bits(w[1])];
        }
    }
    out
}

/// What the quad's samples show: the largest departure of its rows' and columns' adjacent deltas from the exact step,
/// or a collapse, adjacent samples on one coordinate.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Outcome {
    Departure(f64),
    Collapsed,
}

/// The rows' `u` and the columns' `v` of `samples` ([`absolute`]), each run against the exact step `2h/N`: a collapse if
/// any two adjacent samples are one f32, otherwise the largest departure, which must pass REQ-TOOL-152's criterion.
/// Every sample lies in [0.5, 1)².
fn check_quad(n: u32, depth: u32, samples: &[Vec<[f32; 2]>]) -> Outcome {
    let step = 2.0 * (-f64::from(depth) - 1.0).exp2() / f64::from(n);
    let at = |i: usize, j: usize, a: usize| samples[j][i][a];
    for row in samples {
        for s in row {
            assert!(
                s.iter().all(|c| (0.5..1.0).contains(c)),
                "N = {n}, ℓ = {depth}: the sample {s:?} is outside the binade [0.5, 1)"
            );
        }
    }
    let runs: Vec<Vec<f32>> = (0..n as usize)
        .flat_map(|k| {
            [
                (0..n as usize).map(|i| at(i, k, 0)).collect(),
                (0..n as usize).map(|j| at(k, j, 1)).collect(),
            ]
        })
        .collect();
    if runs.iter().any(|r| r.windows(2).any(|w| w[0] == w[1])) {
        return Outcome::Collapsed;
    }
    let mut worst: f64 = 0.0;
    for run in &runs {
        let run: Vec<f64> = run.iter().copied().map(f64::from).collect();
        let departure = banding_departure(&run, step);
        assert!(
            !banded(&run, step),
            "N = {n}, ℓ = {depth}: the absolute coordinate's adjacent deltas depart from the step {step:e} by \
             {departure} > {BANDING_BOUND}: banded before ℓ_switch = {L_SWITCH}, with no collapse to fire R-90's \
             switchover"
        );
        worst = worst.max(departure);
    }
    Outcome::Departure(worst)
}

#[test]
fn uv_absolute_no_banding_to_l_switch() {
    let h = gpu();
    let mut collapses = Vec::new();
    for n in TIER_N {
        for depth in 1..=L_SWITCH {
            let outcome = check_quad(n, depth, &absolute(&h, n, depth));
            println!(
                "N = {n:2}, ℓ = {depth:2}, cell {:7}: {outcome:?}",
                cell(depth)
            );
            if outcome == Outcome::Collapsed {
                collapses.push((n, depth));
            }
        }
    }
    assert_eq!(
        collapses, COLLAPSES,
        "the sweep's collapses (N, ℓ) are not R-395's: a collapse where the sum should be exact hides the criterion"
    );
}

negative_control!(
    uv_absolute_no_banding_to_l_switch,
    "the non-dyadic fixture, N = 6 at ℓ = 19, departs past the proposed bound before any collapse (RQ-258, open)",
    expected = "banded before ℓ_switch",
    {
        let (n, depth) = (6, 19);
        check_quad(n, depth, &absolute(&gpu(), n, depth));
    }
);
