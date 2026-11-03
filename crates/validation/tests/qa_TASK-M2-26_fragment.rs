//! QA tests for TASK-M2-26 on the GPU, the fragment compiled as the backend compiles it (R-297), written from the
//! requirement (REQ-RENDER-025; inverse-encode contract Part 8 and Part 4), not from the implementation. Neither needs
//! the built kernel: the preset's encode and decodes are the checked-in generated snippets.
//!
//! - REQ-RENDER-025, Part 4's T2 rule (`qa_m226_latent_slices_dark_and_within_eps_phys`): on latent slices other than
//!   the DECODE preset's (seeded random `z₀`, `q₁`, `q₂`, inside the controls' clamps), every sample the CPU finds
//!   *interior* by Part 4's factor of two (every control's margin at least 2, `validation::roundtrip::margins`) is
//!   untagged and its fragment residual within `ε_phys` (ratio ≤ 1); every untagged sample anywhere is within `ε_phys`,
//!   on three further slices reaching past every control's clamp too, which show the clamp tag;
//!   and the rendered colour preset is dark everywhere but at tagged samples: no pixel has red, and each non-black pixel
//!   is a tagged sample. Most samples are interior, so the check bites. Control: the fragment encodes the decoded state
//!   with its momenta scaled by 1.001 (a broken inverse), and the interior goes past `ε_phys`.
//! - REQ-RENDER-025, "feasibility edges … tagged expected" (`qa_m226_le_feasibility_edge_is_tagged`): on an `(L_z, E)`
//!   chart at frozen controls of its own over `s ∈ [0, 1.5]`, every sample past the warp's edge `s = 1` (where
//!   `|L_z| = |2s − 1|·L_max > L_max`, so no state exists) is tagged feasibility and drawn in a tag colour without red;
//!   every sample inside `s < 1` is not feasibility-tagged, and is untagged within `ε_phys` or tagged. Control: the
//!   class's feasibility test cut, so the infeasible side is not tagged.
//!
//! **Domain** (pitfalls §10): the stateless trip `z → D → E → D` at single chart points; no trajectory.

use kernel::chart::invariant::Invariant;
use kernel::chart::{InvariantLE, Latent};
use kernel::decode::Ic;
use kernel::generated::links::Defaults;
use kernel::stage::decode::{sample_with, UniformChart};
use render::bind::ViewOutput;
use render::presets::roundtrip::{self, Class};
use render::presets::DecodeChart;
use validation::decode_preset::Scene;
use validation::gpu::GpuHarness;
use validation::negative_control;

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// SplitMix64: the same slices on every run.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn in_range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * ((self.next() >> 40) as f32 / (1u64 << 24) as f32)
    }
}

/// `side` × `side` one-pixel samples over `chart`, its `u` axis spanning `[0, extent]`.
fn scene(chart: DecodeChart, side: u32, extent: f64) -> Scene {
    let depth = 20u32;
    let cells: Vec<[u64; 2]> = (0..side)
        .flat_map(|j| (0..side).map(move |i| [i, j]))
        .map(|ij| {
            let span = [extent, 1.0];
            [0, 1].map(|k| {
                ((f64::from(ij[k]) + 0.5) * span[k] * f64::from(depth).exp2() / f64::from(side))
                    as u64
            })
        })
        .collect();
    Scene::new(chart, [side, side], 1, depth, &cells).unwrap()
}

/// One sample's words-view reading: (ratio, class).
struct Read {
    ratio: f64,
    class: Class,
}

/// The words view `view` over `scene`, per sample, and the colour preset `colour_view`'s pixel per sample.
fn read(
    gpu: &GpuHarness,
    scene: &Scene,
    words_view: &str,
    colour_view: &str,
) -> Vec<(Read, [f32; 3])> {
    let words = scene
        .draw_view(gpu.device(), gpu.queue(), words_view, ViewOutput::Words)
        .unwrap();
    let colour = scene
        .draw_view(gpu.device(), gpu.queue(), colour_view, ViewOutput::Colour)
        .unwrap();
    let grid = scene.context.grid;
    let (w, h) = scene.size();
    let mut out: Vec<Option<(Read, [f32; 3])>> = (0..grid.quad_count()).map(|_| None).collect();
    for y in 0..h {
        for x in 0..w {
            let q = grid.cell(x, y).quad as usize;
            let wd = words.words(x, y);
            let c = colour.words(x, y);
            let class = Class::from_code(wd[1])
                .unwrap_or_else(|| panic!("class code {} is no class", wd[1]));
            out[q] = Some((
                Read {
                    ratio: f64::from(f32::from_bits(wd[0])),
                    class,
                },
                [
                    f32::from_bits(c[0]),
                    f32::from_bits(c[1]),
                    f32::from_bits(c[2]),
                ],
            ));
        }
    }
    out.into_iter()
        .map(|o| o.expect("a sample drew no pixel"))
        .collect()
}

/// `view` with `edit` applied to the trip, checked to have applied.
fn edited(view: &str, from: &str, to: &str) -> String {
    let out = view.replace(from, to);
    assert_ne!(out, view, "the control's edit did not apply");
    out
}

fn widen(x: &Ic<f32>) -> Ic<f64> {
    Ic {
        m: x.m.map(f64::from),
        r: x.r.map(|v| v.map(f64::from)),
        p: x.p.map(|v| v.map(f64::from)),
    }
}

// ── Latent slices ────────────────────────────────────────────────────────────────────────────────────────────────────

fn check_latent(edit: Option<(&str, &str)>) {
    let gpu = gpu();
    let (mut words, mut colour) = (
        roundtrip::words().view().to_owned(),
        roundtrip::preset().view().to_owned(),
    );
    if let Some((from, to)) = edit {
        words = edited(&words, from, to);
        colour = edited(&colour, from, to);
    }
    let mut rng = Rng(0x0343_2026);
    let side = 32u32;
    let mut report = Vec::new();
    let mut clamped = 0usize;
    for slice in 0..9 {
        // Slices 6 to 8 reach 20 along random directions, past every control's clamp (near ±13.8 at ε = 10⁻⁶).
        let wide = if slice < 6 { 1.0 } else { 20.0 };
        let z0: [f32; 8] = std::array::from_fn(|_| rng.in_range(-1.5, 1.5));
        let q1: [f32; 8] = std::array::from_fn(|_| wide * rng.in_range(-1.0, 1.0));
        let q2: [f32; 8] = std::array::from_fn(|_| wide * rng.in_range(-1.0, 1.0));
        let mut s = scene(DecodeChart::Latent, side, 1.0);
        let chart = Latent { z0, q1, q2 };
        s.uniforms.chart = chart.uniform();
        let reads = read(&gpu, &s, &words, &colour);
        let (mut interior, mut worst, mut tagged) = (0usize, 0.0f64, 0usize);
        for (q, (r, rgb)) in reads.iter().enumerate() {
            let p = s.point(q as u32);
            let x = sample_with::<Latent<f32>, Defaults, f32>(&chart, p[0], p[1], None).ic;
            let m = validation::roundtrip::margins(&widen(&x));
            if m.iter().all(|v| *v >= 2.0) {
                interior += 1;
                assert_eq!(
                    r.class,
                    Class::Untagged,
                    "slice {slice} sample {q} at {p:?}: interior (margins {m:?}) but tagged"
                );
            }
            if r.class == Class::Untagged {
                worst = worst.max(r.ratio);
                assert!(
                    r.ratio <= 1.0,
                    "slice {slice} sample {q} at {p:?}: untagged residual {} ε_phys, past ε_phys",
                    r.ratio
                );
            } else {
                tagged += 1;
            }
            if r.class == Class::Clamp {
                clamped += 1;
            }
            assert_eq!(
                rgb[0], 0.0,
                "slice {slice} sample {q}: a red pixel {rgb:?} ({:?})",
                r.class
            );
            if rgb.iter().any(|v| *v != 0.0) {
                assert_ne!(
                    r.class,
                    Class::Untagged,
                    "slice {slice} sample {q}: lit {rgb:?} but untagged"
                );
            }
        }
        assert!(
            slice >= 6 || interior * 2 > reads.len(),
            "slice {slice}: only {interior} of {} samples interior, too few to bite",
            reads.len()
        );
        report.push(format!(
            "slice {slice}: {interior} interior, {tagged} tagged, worst {worst:.3} ε_phys"
        ));
    }
    println!("qa_m226_latent: {}", report.join("; "));
    assert!(clamped > 0, "the wide slices show no clamp boundary");
}

#[test]
fn qa_m226_latent_slices_dark_and_within_eps_phys() {
    check_latent(None);
}

negative_control!(
    qa_m226_latent_slices_dark_and_within_eps_phys,
    "an encode of momenta scaled by 1.001 breaks the inverse: the interior goes past ε_phys",
    expected = "past ε_phys",
    check_latent(Some((
        "let e = frag_encode(d.m, d.r, d.p);",
        "let e = frag_encode(d.m, d.r, array<vec2<f32>, 3>(d.p[0] * 1.001, d.p[1] * 1.001, d.p[2] * 1.001));",
    )))
);

// ── The (L_z, E) feasibility edge ────────────────────────────────────────────────────────────────────────────────────

fn check_le(edit: Option<(&str, &str)>) {
    let gpu = gpu();
    let (mut words, mut colour) = (
        roundtrip::words().view().to_owned(),
        roundtrip::preset().view().to_owned(),
    );
    if let Some((from, to)) = edit {
        words = edited(&words, from, to);
        colour = edited(&colour, from, to);
    }
    let mut s = scene(DecodeChart::InvariantLE, 40, 1.5);
    let held = InvariantLE::<f32>::from_uniform(&s.uniforms.chart).0;
    s.uniforms.chart = InvariantLE(Invariant {
        z_cfg: [-0.6, 0.5],
        z_mass: [0.1, 0.4],
        ..held
    })
    .uniform();
    let reads = read(&gpu, &s, &words, &colour);
    let (mut outside, mut inside) = (0usize, 0usize);
    for (q, (r, rgb)) in reads.iter().enumerate() {
        let p = s.point(q as u32);
        if p[0] > 1.0 + 1e-3 {
            outside += 1;
            assert_eq!(
                r.class,
                Class::Feasibility,
                "sample {q} at {p:?}: past s = 1, not tagged feasibility"
            );
            assert!(
                rgb[0] == 0.0 && rgb.iter().any(|v| *v > 0.0),
                "sample {q} at {p:?}: infeasible but drawn {rgb:?}, not a tag colour"
            );
        } else if p[0] < 1.0 - 1e-3 {
            inside += 1;
            assert_ne!(
                r.class,
                Class::Feasibility,
                "sample {q} at {p:?}: inside s < 1, tagged feasibility"
            );
            if r.class == Class::Untagged {
                assert!(
                    r.ratio <= 1.0,
                    "sample {q} at {p:?}: untagged {} ε_phys, past ε_phys",
                    r.ratio
                );
            }
            assert_eq!(rgb[0], 0.0, "sample {q} at {p:?}: a red pixel {rgb:?}");
        }
    }
    assert!(
        outside > 0 && inside > 0,
        "the scene straddles no edge ({inside} inside, {outside} outside)"
    );
    println!("qa_m226_le: {inside} inside, {outside} outside");
}

#[test]
fn qa_m226_le_feasibility_edge_is_tagged() {
    check_le(None);
}

negative_control!(
    qa_m226_le_feasibility_edge_is_tagged,
    "with the class's feasibility test cut the infeasible side is not tagged",
    expected = "not tagged feasibility",
    check_le(Some(("if (tag != 0u || tag_free != 0u) {", "if (false) {")))
);
