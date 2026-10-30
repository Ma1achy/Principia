//! The ROUNDTRIP preset's harness (TASK-M2-26; REQ-RENDER-025, REQ-ENC-002): the trip `z → D → E → D` the preset draws
//! (`render::presets::roundtrip`), read back as words over the DECODE preset's scenes (`crate::decode_preset::Scene`),
//! its CPU twin, and the golden suite `fixtures/golden/roundtrip-preset/` of the latent and `(L_z, E)` charts.
//!
//! **The twin** is the same trip on the CPU at f32 from the one Rust source the fragment's snippets are translated
//! from (R-116): the chart's decode stage (`kernel::stage::decode::sample_with`, the energy target off, as the scenes'
//! uniforms hold it), the encode (`kernel::encode::encode`) and the free decode (`kernel::decode::decode`), each under
//! the default links; its residual is `validation::roundtrip::phys` on the two states widened, its class
//! `render::presets::roundtrip::Class::of` on the two tag words and the encode's notices.

use kernel::chart::{InvariantLE, Latent};
use kernel::decode::Ic;
use kernel::generated::links::Defaults;
use kernel::stage::decode::{sample_with, tag, DecodeUniforms, UniformChart};
use kernel::stage::fragment::FragmentChart;
use render::presets::roundtrip::{self, Class};
use render::presets::DecodeChart;

use crate::decode_preset::{f, Scene};
use crate::roundtrip::{phys, widen};

/// The charts the golden suite draws (REQ-RENDER-025's verify: the latent and `(L_z, E)` charts).
pub const CHARTS: [DecodeChart; 2] = [DecodeChart::Latent, DecodeChart::InvariantLE];

/// One pixel's trip as the words view writes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trip {
    /// `residual / ε_phys`, 0 where invalid.
    pub ratio: f64,
    /// The class.
    pub class: Class,
    /// The encode's notices word.
    pub notices: u32,
    /// The first decode's tag word, and the free decode's shifted by 8.
    pub tags: u32,
}

impl Trip {
    /// The trip of the words view's four words, `Err` for a class code the view never writes.
    pub fn of(w: [u32; 4]) -> Result<Trip, String> {
        Ok(Trip {
            ratio: f(w[0]),
            class: Class::from_code(w[1]).ok_or(format!("class code {} is no class", w[1]))?,
            notices: w[2],
            tags: w[3],
        })
    }
}

/// The ROUNDTRIP preset's trips over `scene`, one-pixel tiles, in sample order: its words view
/// (`render::presets::roundtrip::words`).
pub fn trips(
    scene: &Scene,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<Vec<Trip>, String> {
    trips_view(scene, device, queue, roundtrip::words().view())
}

/// The trips the words view `view` writes over `scene`, one-pixel tiles, in sample order: [`trips`] for a view that is
/// not the library's, a test's control among them.
pub fn trips_view(
    scene: &Scene,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    view: &str,
) -> Result<Vec<Trip>, String> {
    let grid = scene.context.grid;
    if grid.tile_px != 1 {
        return Err("the trips are read one pixel per sample".to_owned());
    }
    let image = scene.draw_view(device, queue, view, render::bind::ViewOutput::Words)?;
    let mut out = vec![None; grid.quad_count() as usize];
    let (width, height) = scene.size();
    for y in 0..height {
        for x in 0..width {
            let w = image.words(x, y);
            out[grid.cell(x, y).quad as usize] = Some(Trip::of([w[0], w[1], w[2], w[3]])?);
        }
    }
    out.into_iter()
        .map(|t| t.ok_or_else(|| "a sample drew no pixel".to_owned()))
        .collect()
}

/// The CPU twin's trip: the residual in physical units, the class, and the states and words it read.
#[derive(Clone, Copy, Debug)]
pub struct Twin {
    /// `‖D(z) − D(E(D(z)))‖_phys`, NaN where a state is non-finite.
    pub residual: f64,
    /// The class.
    pub class: Class,
    /// The encode's notices word.
    pub notices: u32,
    /// The first decode's tag word, and the free decode's shifted by 8.
    pub tags: u32,
}

impl Twin {
    /// `residual / ε_phys`, 0 where invalid, as the view writes it.
    pub fn ratio(&self) -> f64 {
        match self.class {
            Class::Invalid => 0.0,
            _ => roundtrip::ratio(self.residual),
        }
    }
}

/// The trip from the decoded state `x` and its tag word.
pub fn twin_of(x: &Ic<f32>, first_tag: u32) -> Twin {
    let e = kernel::encode::encode::<Defaults, f32>(x);
    let back = kernel::decode::decode::<Defaults, f32>(&e.z);
    let notices = kernel::stage::encode::notices(&e.report);
    let back_tag = tag(back.degenerate);
    let (xw, bw) = (widen(x), widen(&back.ic));
    let finite = |s: &Ic<f64>| {
        s.m.iter()
            .chain(s.r.as_flattened())
            .chain(s.p.as_flattened())
            .all(|v| v.is_finite())
    };
    let residual = phys(&xw, &bw);
    let ok = finite(&xw) && finite(&bw) && residual.is_finite();
    Twin {
        residual,
        class: Class::of(ok, first_tag, back_tag, notices),
        notices,
        tags: first_tag | (back_tag << 8),
    }
}

/// `C`'s trip at the f32 chart point `uv`, its parameters `u`.
fn twin_on<C: FragmentChart>(u: &DecodeUniforms, uv: [f32; 2]) -> Twin {
    let chart = C::from_uniform(&u.chart);
    let out = sample_with::<C, Defaults, f32>(&chart, uv[0], uv[1], None);
    twin_of(&out.ic, tag(out.degenerate))
}

/// `chart`'s CPU trip at the f32 chart point `uv`: [`CHARTS`]' charts only.
pub fn twin(chart: DecodeChart, u: &DecodeUniforms, uv: [f32; 2]) -> Result<Twin, String> {
    match chart {
        DecodeChart::Latent => Ok(twin_on::<Latent<f32>>(u, uv)),
        DecodeChart::InvariantLE => Ok(twin_on::<InvariantLE<f32>>(u, uv)),
        other => Err(format!(
            "the ROUNDTRIP suite draws the latent and (L_z, E) charts, not {other:?}"
        )),
    }
}

// ── The golden suite (`fixtures/golden/roundtrip-preset/`; REQ-RENDER-025) ──────────────────────────────────────────

/// The golden suite's name, its directory under `fixtures/golden/`, which `cargo xtask golden roundtrip-preset` runs.
pub const GOLDEN_SUITE: &str = "roundtrip-preset";

/// A golden scene's one-pixel samples, columns × rows.
pub const GOLDEN_SIDE: [u32; 2] = [48, 24];

/// A golden scene's cells' depth: 20, so a sample's chart point is its cell's centre to `2⁻²¹`.
pub const GOLDEN_DEPTH: u32 = 20;

/// The golden scene's name for `chart`: `roundtrip-preset-<section>`.
pub fn golden_name(chart: DecodeChart) -> String {
    format!("{GOLDEN_SUITE}-{}", chart.section())
}

/// The chart whose golden scene is `name`.
pub fn golden_chart(name: &str) -> Option<DecodeChart> {
    CHARTS.into_iter().find(|c| golden_name(*c) == name)
}

/// The golden scene's chart parameters and its slice's extent on `u`, in the chart's own `(u, v)`:
/// - the latent chart at the DECODE preset's parameters (`crate::decode_preset::uniforms`) over the unit square, its
///   interior, every sample untagged: the view dark everywhere;
/// - the `(L_z, E)` chart at the DECODE preset's parameters over `u ∈ [0, 1.5]`, past the square's edge `s = 1`, where
///   `|L_z| > L_max` and the construction is infeasible (`INVARIANT_INFEASIBLE`): the feasibility edge, tagged.
///
/// No golden sample lies at a clamp boundary: there the decode saturates onto the clamp itself, so whether the encode's
/// f32 test reports it clamped is decided by rounding, which a backend's fast-math may decide otherwise (R-297); the
/// tests read those samples' words instead ([`clamp_frame`]).
pub fn golden_uniforms(chart: DecodeChart) -> Result<(DecodeUniforms, f64), String> {
    match chart {
        DecodeChart::Latent => Ok((crate::decode_preset::uniforms(chart), 1.0)),
        DecodeChart::InvariantLE => Ok((crate::decode_preset::uniforms(chart), 1.5)),
        other => Err(format!(
            "the ROUNDTRIP suite draws the latent and (L_z, E) charts, not {other:?}"
        )),
    }
}

/// A latent chart framed by clamp boundaries: the DECODE preset's `z₀`, `q₁` along `z_α` and `q₂` along `z_β`, each
/// `±18`, past both ends of each control's clamp (near `±13.8` for the default links' `ε_z = 10⁻⁶`), so the slice's
/// four sides are clamped and its centre interior.
pub fn clamp_frame() -> DecodeUniforms {
    let axis = |k: usize| {
        let mut q = [0.0f32; 8];
        q[k] = 18.0;
        q
    };
    let mut u = crate::decode_preset::uniforms(DecodeChart::Latent);
    u.chart = Latent {
        z0: [0.3, -0.2, 0.1, -0.4, 0.25, 0.05, 0.2, -0.35],
        q1: axis(0),
        q2: axis(1),
    }
    .uniform();
    u
}

/// The golden scene of `chart`: [`GOLDEN_SIDE`] one-pixel samples on the lattice of depth-[`GOLDEN_DEPTH`] cells spaced
/// evenly over the slice's extent ([`golden_uniforms`]).
pub fn golden(chart: DecodeChart) -> Result<Scene, String> {
    let (uniforms, extent) = golden_uniforms(chart)?;
    let span = [extent, 1.0];
    let cells: Vec<[u64; 2]> = (0..GOLDEN_SIDE[1])
        .flat_map(|j| (0..GOLDEN_SIDE[0]).map(move |i| [i, j]))
        .map(|ij| {
            [0, 1].map(|k| {
                ((f64::from(ij[k]) + 0.5) * span[k] * f64::from(GOLDEN_DEPTH).exp2()
                    / f64::from(GOLDEN_SIDE[k])) as u64
            })
        })
        .collect();
    let mut scene = Scene::new(chart, GOLDEN_SIDE, 1, GOLDEN_DEPTH, &cells)?;
    scene.uniforms = uniforms;
    Ok(scene)
}

/// The CPU twin's trip at every sample of `scene`, in sample order.
pub fn twins(scene: &Scene) -> Result<Vec<Twin>, String> {
    (0..scene.context.grid.quad_count())
        .map(|q| twin(scene.chart, &scene.uniforms, scene.point(q)))
        .collect()
}

/// The ROUNDTRIP preset's CPU twin over `scene`: each pixel's linear RGB, rows from the top.
pub fn expected(scene: &Scene) -> Result<Vec<[f64; 3]>, String> {
    let grid = scene.context.grid;
    let twins = twins(scene)?;
    let (width, height) = scene.size();
    Ok((0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let t = &twins[grid.cell(x, y).quad as usize];
            roundtrip::colour(t.class, t.ratio(), [f64::from(x) + 0.5, f64::from(y) + 0.5])
        })
        .collect())
}

/// The half-width of the band about a clamp, in its `ε` (`crate::roundtrip::margins`' unit), where an f32 trip's rounding
/// could decide the clamp bit: a quarter of `ε`. The f32 resolution of a control's clamp-space value next to its clamp
/// is at most `2⁻²⁴`, 0.06 `ε` at `ε = 10⁻⁶`, and the encode recomputes it within a few of those.
pub const CLAMP_BAND: f64 = 0.25;

/// The samples of `scene` whose class could fall on either side of a clamp, each with its controls' margins: a control
/// of the decoded state within [`CLAMP_BAND`] of its clamp (`crate::roundtrip::margins`, 1 at the clamp), so a golden
/// holds none.
pub fn clamp_ambiguous(scene: &Scene) -> Result<Vec<(u32, [f64; 8])>, String> {
    use crate::roundtrip::margins;
    let mut out = Vec::new();
    for q in 0..scene.context.grid.quad_count() {
        let p = scene.point(q);
        let chart = scene.chart;
        let u = &scene.uniforms;
        let x = match chart {
            DecodeChart::Latent => {
                let c = Latent::<f32>::from_uniform(&u.chart);
                sample_with::<Latent<f32>, Defaults, f32>(&c, p[0], p[1], None).ic
            }
            DecodeChart::InvariantLE => {
                let c = InvariantLE::<f32>::from_uniform(&u.chart);
                sample_with::<InvariantLE<f32>, Defaults, f32>(&c, p[0], p[1], None).ic
            }
            other => return Err(format!("no ROUNDTRIP scene draws {other:?}")),
        };
        let m = margins(&widen(&x));
        if m.iter().any(|m| (m - 1.0).abs() < CLAMP_BAND) {
            out.push((q, m));
        }
    }
    Ok(out)
}

/// The ROUNDTRIP preset rendered over `scene` into an `Rgba32Float` target: each pixel's RGBA, rows from the top.
pub fn render(
    scene: &Scene,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<Vec<[f32; 4]>, String> {
    let image = scene.draw(device, queue, &roundtrip::preset())?;
    let (width, height) = scene.size();
    Ok((0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let w = image.words(x, y);
            [0, 1, 2, 3].map(|k| f32::from_bits(w[k]))
        })
        .collect())
}
