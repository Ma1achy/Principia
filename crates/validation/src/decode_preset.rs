//! The DECODE and agreement presets' harness (TASK-M2-25; REQ-RENDER-027, REQ-TOOL-029, REQ-COL-006): each M2 chart
//! at fixed parameters, the f64 `decodeOnly()` it is held to, the scenes the presets draw, and their renders through
//! the render harness (`render::bind::module`, `render::bind::upload_decode`).
//!
//! **The scenes.** A scene is a flat grid of quads, one sample each (`N = 1`), each quad placed at its own cell of the
//! slice (`engine::synthetic::Synthetic::place`), so its frame `c` is the sample's chart point: a pixel of a one-pixel
//! tile decodes at `c` exactly, and a larger tile's pixels at `c + h·(2t − 1)`.
//!
//! **The reference** is the f64 decode, `engine::decode_dispatch::decode_only`, at the f32 chart point the fragment
//! decoded, widened (pitfalls §10: the inputs held fixed), and `Φ`'s `z` from the chart's own `map` at f64.

use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::{decode_only, DecodeStage};
use engine::synthetic::Synthetic;
use kernel::generated::links::Defaults;
use kernel::stage::decode::{
    BurrauThetaKDefaults, DecodeMode, DecodeOut, DecodeUniforms, DecodeVariant,
    EuclidPlaneDefaults, InvariantLEDefaults, InvariantLKDefaults, LatentDefaults,
    MassSimplexDefaults, ShapeSphereDefaults, UniformChart,
};
use kernel::stage::fragment::FragmentChart;
use render::bind::{self, Context, ViewOutput};
use render::headless::{self, Draw, Target};
use render::presets::{DecodeChart, Preset};
use render::raster::Grid;

use crate::decode_f32_eps::physics_key;

/// The parameters each chart is drawn at: the defaults the decode's cross-backend test holds REQ-DEC-043's factor on
/// (`engine/tests/decode_cross_backend.rs`), one fixed chart each.
pub fn uniforms(chart: DecodeChart) -> DecodeUniforms {
    use kernel::chart::{
        invariant::Invariant, BurrauThetaK, EuclidPlane, InvariantLE, InvariantLK, Latent,
        MassSimplex, ShapeMap, ShapeSphere,
    };
    let k_max = crate::K_MAX.number() as f32;
    let gamma_k = crate::GAMMA_K.number() as f32;
    let frozen = Invariant {
        z_cfg: [0.4, -0.3],
        z_mass: [0.2, -0.5],
        k_max,
        gamma_k,
    };
    let rows = match chart {
        DecodeChart::Latent => Latent {
            z0: [0.3, -0.2, 0.1, -0.4, 0.25, 0.05, 0.2, -0.35],
            q1: [0.5, 0.2, -0.3, 0.1, 0.0, 0.2, 0.4, -0.1],
            q2: [-0.2, 0.5, 0.2, 0.3, -0.25, 0.1, -0.3, 0.3],
        }
        .uniform(),
        DecodeChart::ShapeSphere => ShapeSphere {
            map: ShapeMap::Spherical,
            n0: [0.0, 0.0, 1.0],
            e1: [1.0, 0.0, 0.0],
            e2: [0.0, 1.0, 0.0],
            scale: 1.2,
            inertia: 1.0,
            phase: 0.0,
            z_mass: [0.3, -0.2],
            z_mom: [0.2, -0.1, 0.3, 0.15],
        }
        .uniform(),
        DecodeChart::InvariantLE => InvariantLE(frozen).uniform(),
        DecodeChart::InvariantLK => InvariantLK(frozen).uniform(),
        DecodeChart::MassSimplex => MassSimplex {
            nu0: 0.5,
            eps_m: crate::MASS_SIMPLEX_EPS_M.number() as f32,
        }
        .uniform(),
        DecodeChart::EuclidPlane => EuclidPlane {
            m_range: [
                crate::EUCLID_M_MIN.number() as f32,
                crate::EUCLID_M_MAX.number() as f32,
            ],
            nu_range: [
                crate::EUCLID_NU_MIN.number() as f32,
                crate::EUCLID_NU_MAX.number() as f32,
            ],
        }
        .uniform(),
        DecodeChart::BurrauThetaK => BurrauThetaK {
            theta_range: [
                crate::BURRAU_THETA_MIN.number() as f32,
                crate::BURRAU_THETA_MAX.number() as f32,
            ],
            k_max,
            gamma_k,
        }
        .uniform(),
    };
    DecodeUniforms {
        chart: rows,
        ..DecodeUniforms::default()
    }
}

/// The decode stage's uniform as the bytes the fragment binds (`render::bind::DECODE_BINDING`): the six chart rows,
/// the flags row and the energy row, 128 B.
pub fn uniform_bytes(u: &DecodeUniforms) -> Vec<u8> {
    let mut out = Vec::with_capacity(128);
    for row in u.chart {
        for x in [row.x, row.y, row.z, row.w] {
            out.extend(x.to_le_bytes());
        }
    }
    for row in [u.flags, u.energy] {
        for x in [row.x, row.y, row.z, row.w] {
            out.extend(x.to_le_bytes());
        }
    }
    out
}

/// One decode at f64: the decode stage's output, `Φ`'s `z` and whether it is the latent.
#[derive(Clone, Copy, Debug)]
pub struct Reference {
    pub out: DecodeOut<f64>,
    pub z: [f64; 8],
    pub z_latent: bool,
}

impl Reference {
    /// What the DECODE preset shows of it (`render::presets::decode::Shown`).
    pub fn shown(&self) -> render::presets::decode::Shown {
        render::presets::decode::Shown {
            z: self.z,
            z_latent: self.z_latent,
            m: self.out.ic.m,
            r: self.out.ic.r,
        }
    }
}

/// `C`'s f64 `decodeOnly()` at the f32 point `uv`, widened, and its `Φ` there.
fn reference_of<C: FragmentChart>(u: &DecodeUniforms, uv: [f32; 2]) -> Result<Reference, String> {
    let chart = C::from_uniform(&u.chart);
    let wide = [f64::from(uv[0]), f64::from(uv[1])];
    let out = decode_only::<C, Defaults>(&chart, wide, &physics_key()).map_err(|e| e.0)?;
    let z = chart.map(wide[0], wide[1]).z.z;
    Ok(Reference {
        out,
        z,
        z_latent: C::Z_IS_LATENT,
    })
}

/// `chart`'s f64 reference at `uv`, its parameters `u`.
pub fn reference(
    chart: DecodeChart,
    u: &DecodeUniforms,
    uv: [f32; 2],
) -> Result<Reference, String> {
    use kernel::chart::*;
    match chart {
        DecodeChart::Latent => reference_of::<Latent<f32>>(u, uv),
        DecodeChart::ShapeSphere => reference_of::<ShapeSphere<f32>>(u, uv),
        DecodeChart::InvariantLE => reference_of::<InvariantLE<f32>>(u, uv),
        DecodeChart::InvariantLK => reference_of::<InvariantLK<f32>>(u, uv),
        DecodeChart::MassSimplex => reference_of::<MassSimplex<f32>>(u, uv),
        DecodeChart::EuclidPlane => reference_of::<EuclidPlane<f32>>(u, uv),
        DecodeChart::BurrauThetaK => reference_of::<BurrauThetaK<f32>>(u, uv),
    }
}

/// A scene: a chart, its parameters, the samples' payload and the context.
pub struct Scene {
    pub chart: DecodeChart,
    pub uniforms: DecodeUniforms,
    pub set: Synthetic,
    pub context: Context,
}

/// The scene's context: the harness's defaults over `grid`.
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
        valid_sample_count: 1,
    }
}

impl Scene {
    /// `chart` over `quads[0]` × `quads[1]` quads of one sample, each `tile_px` square, quad `q` at the depth-`depth`
    /// cell `cells[q]`.
    pub fn new(
        chart: DecodeChart,
        quads: [u32; 2],
        tile_px: u32,
        depth: u32,
        cells: &[[u64; 2]],
    ) -> Result<Scene, String> {
        let grid = Grid::new(quads, 1, 0, tile_px)?;
        if cells.len() != grid.quad_count() as usize {
            return Err(format!(
                "{} cells for {} quads",
                cells.len(),
                grid.quad_count()
            ));
        }
        let mut set = Synthetic::flat(grid, depth);
        for (q, cell) in (0u32..).zip(cells) {
            set.place(q, depth, *cell);
        }
        Ok(Scene {
            chart,
            uniforms: uniforms(chart),
            set,
            context: context(grid),
        })
    }

    /// `chart` over `side[0]` × `side[1]` one-pixel samples on the lattice of depth `depth`'s cells spaced evenly
    /// across the slice: sample `(i, j)` at cell `⌊(i + ½)·2^depth / side⌋` on each axis.
    pub fn lattice(chart: DecodeChart, side: [u32; 2], depth: u32) -> Result<Scene, String> {
        let cells: Vec<[u64; 2]> = (0..side[1])
            .flat_map(|j| (0..side[0]).map(move |i| [i, j]))
            .map(|ij| {
                [0, 1].map(|k| {
                    ((f64::from(ij[k]) + 0.5) * (depth as f64).exp2() / f64::from(side[k])) as u64
                })
            })
            .collect();
        Scene::new(chart, side, 1, depth, &cells)
    }

    /// Sample `q`'s chart point, its quad's centre at f32, as the fragment reads it.
    pub fn point(&self, q: u32) -> [f32; 2] {
        let c = self.set.quad_frame(q).c;
        [c[0] as f32, c[1] as f32]
    }

    /// The target's size.
    pub fn size(&self) -> (u32, u32) {
        self.context.grid.target()
    }

    /// Draws `preset` over the scene: each pixel's four channels, rows from the top, as f32 for a colour view and as
    /// u32 words for a words view.
    pub fn draw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        preset: &Preset,
    ) -> Result<headless::Image, String> {
        let module = preset.module(self.chart)?;
        let bytes = self.set.bytes();
        let bound = bind::upload_decode(
            device,
            &bytes.payload(),
            &self.context,
            &uniform_bytes(&self.uniforms),
        );
        let (width, height) = self.size();
        let format = match preset.output() {
            ViewOutput::Colour => wgpu::TextureFormat::Rgba32Float,
            ViewOutput::Words => wgpu::TextureFormat::Rgba32Uint,
        };
        headless::render(
            device,
            queue,
            &Draw {
                module: &module,
                entry: bind::ENTRY,
                layouts: &bound.layout_refs(),
                groups: &bound.group_refs(),
            },
            Target {
                width,
                height,
                format,
            },
        )
    }

    /// The DECODE preset's decode of every sample, as words (`render::presets::decode::PROBE_WORDS`), in sample
    /// order: one render per word group, on one-pixel tiles.
    pub fn probe(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Vec<[u32; render::presets::decode::PROBE_WORDS]>, String> {
        use render::presets::decode::{probe, PROBE_GROUPS, PROBE_WORDS};
        let grid = self.context.grid;
        if grid.tile_px != 1 {
            return Err("the probe reads one pixel per sample".to_owned());
        }
        let mut out = vec![[0u32; PROBE_WORDS]; grid.quad_count() as usize];
        let (width, height) = self.size();
        for g in 0..PROBE_GROUPS {
            let image = self.draw(device, queue, &probe(g))?;
            for y in 0..height {
                for x in 0..width {
                    let cell = grid.cell(x, y);
                    let w = image.words(x, y);
                    for k in 0..4 {
                        out[cell.quad as usize][4 * g as usize + k] = w[k];
                    }
                }
            }
        }
        Ok(out)
    }
}

impl Scene {
    /// Runs the kernel decode stage (the kernel's WGSL `wgsl`) at every sample's chart point and writes each decode's
    /// `ICDescriptor` into the payload, as the decode stage writes `ctx.ic`; `shift` writes sample `q`'s at sample
    /// `(q + shift) mod count` instead, a dispatch scramble (0 is none). Returns the decodes in sample order.
    pub fn stage_into(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        wgsl: &str,
        shift: u32,
    ) -> Result<Vec<DecodeOut<f32>>, String> {
        let grid = self.context.grid;
        if grid.n != 1 {
            return Err("the scene holds one sample per quad".to_owned());
        }
        let count = grid.quad_count();
        let points: Vec<[f32; 2]> = (0..count).map(|q| self.point(q)).collect();
        let outs = stage(device, queue, wgsl, self.chart, &self.uniforms, &points)?;
        for (q, o) in (0u32..).zip(&outs) {
            let i = grid.sample_index((q + shift) % count, 0, 0);
            *self.set.ic(i) = o.descriptor;
        }
        Ok(outs)
    }

    /// The agreement preset's words (`render::presets::agreement::words`) at every sample, in sample order:
    /// `(ratio's bits, valid, E's bits, E₀'s bits)`, on one-pixel tiles.
    pub fn agreement(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Vec<[u32; 4]>, String> {
        let grid = self.context.grid;
        if grid.tile_px != 1 {
            return Err("the agreement words are read one pixel per sample".to_owned());
        }
        let image = self.draw(device, queue, &render::presets::agreement::words())?;
        let mut out = vec![[0u32; 4]; grid.quad_count() as usize];
        let (width, height) = self.size();
        for y in 0..height {
            for x in 0..width {
                let w = image.words(x, y);
                out[grid.cell(x, y).quad as usize] = [w[0], w[1], w[2], w[3]];
            }
        }
        Ok(out)
    }
}

/// The f32 value of `bits`, widened.
pub fn f(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

/// The decode stage, on the GPU, at `points` under `chart`'s variant `V`: each point's decode, as the payload stores it.
fn stage_of<V: DecodeVariant>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    wgsl: &str,
    u: &DecodeUniforms,
    points: &[[f32; 2]],
) -> Result<Vec<DecodeOut<f32>>, String> {
    let stage = DecodeStage::<V>::new(device, wgsl, FastMath::default()).map_err(|e| e.0)?;
    let chart = V::Chart::from_uniform(&u.chart);
    stage
        .run_with(device, queue, &chart, DecodeMode::Full, None, points)
        .map_err(|e| e.0)
}

/// The kernel decode stage's decode of `points` under `chart` (the kernel's WGSL `wgsl`, `target/spirv/kernel.wgsl`).
pub fn stage(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    wgsl: &str,
    chart: DecodeChart,
    u: &DecodeUniforms,
    points: &[[f32; 2]],
) -> Result<Vec<DecodeOut<f32>>, String> {
    match chart {
        DecodeChart::Latent => stage_of::<LatentDefaults>(device, queue, wgsl, u, points),
        DecodeChart::ShapeSphere => stage_of::<ShapeSphereDefaults>(device, queue, wgsl, u, points),
        DecodeChart::InvariantLE => stage_of::<InvariantLEDefaults>(device, queue, wgsl, u, points),
        DecodeChart::InvariantLK => stage_of::<InvariantLKDefaults>(device, queue, wgsl, u, points),
        DecodeChart::MassSimplex => stage_of::<MassSimplexDefaults>(device, queue, wgsl, u, points),
        DecodeChart::EuclidPlane => stage_of::<EuclidPlaneDefaults>(device, queue, wgsl, u, points),
        DecodeChart::BurrauThetaK => {
            stage_of::<BurrauThetaKDefaults>(device, queue, wgsl, u, points)
        }
    }
}

/// The tag word the decode stage writes for `out` (`kernel::stage::decode::tag`): 0 none.
pub fn tag(out: &DecodeOut<f64>) -> u32 {
    kernel::stage::decode::tag(out.degenerate)
}

/// The tag word of `kernel::stage::decode::tag` for an f32 decode.
pub fn tag32(out: &DecodeOut<f32>) -> u32 {
    kernel::stage::decode::tag(out.degenerate)
}

/// A state `(m, r, p)` at f32, as a test gives it to the fragment's descriptor or encode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub m: [f32; 3],
    pub r: [[f32; 2]; 3],
    pub p: [[f32; 2]; 3],
}

impl State {
    fn ic(&self) -> kernel::decode::Ic<f32> {
        kernel::decode::Ic {
            m: self.m,
            r: self.r,
            p: self.p,
        }
    }
}

/// The descriptor's twelve fields of `s` on the CPU at f32 (`kernel::decode::descriptor::describe`), as bits, in the
/// generated struct's order.
pub fn describe32(s: &State) -> [u32; 12] {
    let d = kernel::decode::descriptor::describe(&s.ic());
    [
        d.m0,
        d.m1,
        d.m2,
        d.q_mass,
        d.rho_mag,
        d.lambda_mag,
        d.rho_ratio,
        d.rho_angle,
        d.K_0,
        d.V_0,
        d.virial_ratio,
        d.r_min_pair_0,
    ]
    .map(f32::to_bits)
}

/// The encode of `s` on the CPU at f32 under the default links, as `kernel::stage::encode`'s twelve words: `z`, `α`,
/// `β`, `λ` and the notices word.
pub fn encode32(s: &State) -> [u32; 12] {
    let e = kernel::encode::encode::<Defaults, f32>(&s.ic());
    let mut w = [0u32; 12];
    for (word, z) in w.iter_mut().zip(e.z.z) {
        *word = z.to_bits();
    }
    w[8] = e.blocks.alpha.to_bits();
    w[9] = e.blocks.beta.to_bits();
    w[10] = e.report.lambda.to_bits();
    w[11] = kernel::stage::encode::notices(&e.report);
    w
}

/// The 27 continuous values of `out`, widened, by REQ-DEC-043's measure's order (`decode_f32_eps::QUANTITIES`).
pub fn values(out: &DecodeOut<f64>) -> [f64; 27] {
    crate::decode_f32_eps::values(out)
}

/// The 27 continuous values of an f32 decode, widened.
pub fn values32(out: &DecodeOut<f32>) -> [f64; 27] {
    crate::decode_f32_eps::values(out)
}

// ── The golden suite (`fixtures/golden/decode-preset/`; REQ-RENDER-027) ────────────────────────────────────────────

/// The golden suite's name, its directory under `fixtures/golden/`, which `cargo xtask golden decode-preset` runs.
pub const GOLDEN_SUITE: &str = "decode-preset";

/// A golden scene's quads, columns × rows, one sample each.
pub const GOLDEN_QUADS: [u32; 2] = [4, 2];

/// A golden scene's tile, in pixels: six, so each of the preset's panels, thirds of the tile on `v` and on `u` in the
/// latent panel, halves on `u` in the positions panel, holds whole pixels, no pixel centre on a panel's edge.
pub const GOLDEN_TILE_PX: u32 = 6;

/// A golden scene's quads' depth: 19, `h = 2⁻²⁰`, so a tile's pixels decode within `2⁻²⁰` of its centre and show
/// one decode each panel, at f32 resolution of the chart point.
pub const GOLDEN_DEPTH: u32 = 19;

/// The least distance, in 8-bit steps, of any channel of a golden reference's CPU twin from a rounding tie: twice the
/// harness cases' 0.01 (R-269, R-296), since the fragment decodes in f32 where the twin decodes in f64, within
/// REQ-DEC-043's factor, about 2·10⁻⁶ on these values, 0.0005 of a step. Applied per R-369 (TASK-M2-25).
pub const GOLDEN_MARGIN: f64 = 0.02;

/// The golden scene's name for `chart`: `decode-preset-<section>`.
pub fn golden_name(chart: DecodeChart) -> String {
    format!("{GOLDEN_SUITE}-{}", chart.section())
}

/// The chart whose golden scene is `name`.
pub fn golden_chart(name: &str) -> Option<DecodeChart> {
    DecodeChart::ALL
        .into_iter()
        .find(|c| golden_name(*c) == name)
}

/// The quads' cells' nominal positions: quad `(i, j)` at the slice's `((i + ½)/4, (j + ½)/2)`, a cell of depth 19.
fn golden_cell(q: u32) -> [u64; 2] {
    let side = 1u64 << GOLDEN_DEPTH;
    let [i, j] = [q % GOLDEN_QUADS[0], q / GOLDEN_QUADS[0]];
    [
        (2 * u64::from(i) + 1) * side / (2 * u64::from(GOLDEN_QUADS[0])),
        (2 * u64::from(j) + 1) * side / (2 * u64::from(GOLDEN_QUADS[1])),
    ]
}

/// The cells tried for quad `q`, nominal first, then stepped by 389 cells (`389·2⁻¹⁹ ≈ 7.4·10⁻⁴` of the slice) on
/// each axis in turn: the first whose tile's twin keeps [`GOLDEN_MARGIN`] is the quad's.
const GOLDEN_TRIES: u64 = 256;

impl Scene {
    /// The chart point the fragment decodes pixel `(x, y)` (rows from the top) at, as the fragment forms it in f32:
    /// `ctx.chart.slice_uv = c + h·(2·ctx.quad.uv − 1)` (R-394), `ctx.quad.uv` the raster's.
    pub fn pixel_point(&self, x: u32, y: u32) -> [f32; 2] {
        let grid = self.context.grid;
        let cell = grid.cell(x, y);
        let frame = self.set.quad_frame(cell.quad);
        let (_, height) = self.size();
        let t = grid.tile_px as f32;
        let up = [x as f32 + 0.5, height as f32 - (y as f32 + 0.5)];
        let column = [
            cell.quad_xy[0] * grid.n + cell.tile_xy[0],
            cell.quad_xy[1] * grid.n + cell.tile_xy[1],
        ];
        let n = grid.n as f32;
        [0, 1].map(|k| {
            let tile_uv = (up[k] - (column[k] * grid.tile_px) as f32) / t;
            let quad_uv = (cell.tile_xy[k] as f32 + tile_uv) / n;
            frame.c[k] as f32 + frame.h[k] as f32 * (2.0 * quad_uv - 1.0)
        })
    }

    /// The pixel's within-tile coordinate, Y-up, as the raster forms it (`ctx.tile.uv`).
    fn tile_uv(&self, x: u32, y: u32) -> [f64; 2] {
        let grid = self.context.grid;
        let cell = grid.cell(x, y);
        let (_, height) = self.size();
        let t = grid.tile_px as f32;
        let up = [x as f32 + 0.5, height as f32 - (y as f32 + 0.5)];
        let column = [
            cell.quad_xy[0] * grid.n + cell.tile_xy[0],
            cell.quad_xy[1] * grid.n + cell.tile_xy[1],
        ];
        [0, 1].map(|k| f64::from((up[k] - (column[k] * grid.tile_px) as f32) / t))
    }

    /// The DECODE preset's CPU twin at pixel `(x, y)`: the f64 decode at the fragment's f32 chart point, coloured by
    /// `render::presets::decode::colour`, linear RGB.
    pub fn expected_at(&self, x: u32, y: u32) -> Result<[f64; 3], String> {
        let r = reference(self.chart, &self.uniforms, self.pixel_point(x, y))?;
        Ok(render::presets::decode::colour(
            &r.shown(),
            self.tile_uv(x, y),
            [f64::from(x) + 0.5, f64::from(y) + 0.5],
        ))
    }

    /// The DECODE preset's CPU twin: each pixel's linear RGB, rows from the top.
    pub fn expected(&self) -> Result<Vec<[f64; 3]>, String> {
        let (width, height) = self.size();
        (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| self.expected_at(x, y))
            .collect()
    }

    /// The pixels of quad `q`, `(x, y)`, rows from the top.
    fn quad_pixels(&self, q: u32) -> Vec<(u32, u32)> {
        let grid = self.context.grid;
        let (width, height) = self.size();
        (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .filter(|&(x, y)| grid.cell(x, y).quad == q)
            .collect()
    }

    /// The DECODE preset rendered over the scene into an `Rgba32Float` target: each pixel's RGBA, rows from the top.
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Vec<[f32; 4]>, String> {
        let image = self.draw(device, queue, &render::presets::decode::preset())?;
        let (width, height) = self.size();
        Ok((0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| {
                let w = image.words(x, y);
                [0, 1, 2, 3].map(|k| f32::from_bits(w[k]))
            })
            .collect())
    }
}

/// The golden scene of `chart`: [`GOLDEN_QUADS`] quads of [`GOLDEN_TILE_PX`]-pixel tiles at depth [`GOLDEN_DEPTH`],
/// each quad at the first cell of its tries whose tile's CPU twin keeps [`GOLDEN_MARGIN`] from every rounding tie.
pub fn golden(chart: DecodeChart) -> Result<Scene, String> {
    let count = GOLDEN_QUADS[0] * GOLDEN_QUADS[1];
    let cells: Vec<[u64; 2]> = (0..count).map(golden_cell).collect();
    let mut scene = Scene::new(chart, GOLDEN_QUADS, GOLDEN_TILE_PX, GOLDEN_DEPTH, &cells)?;
    for q in 0..count {
        let base = golden_cell(q);
        let pixels = scene.quad_pixels(q);
        let mut placed = false;
        for k in 0..GOLDEN_TRIES {
            let step = 389 * (k / 2 + k % 2);
            let cell = if k % 2 == 0 {
                [base[0] + step, base[1]]
            } else {
                [base[0], base[1] + step]
            };
            scene.set.place(q, GOLDEN_DEPTH, cell);
            let colours = pixels
                .iter()
                .map(|&(x, y)| scene.expected_at(x, y))
                .collect::<Result<Vec<_>, _>>()?;
            if crate::golden_scene::tie_margin(&colours) >= GOLDEN_MARGIN {
                placed = true;
                break;
            }
        }
        if !placed {
            return Err(format!(
                "{chart:?}: no cell of quad {q}'s {GOLDEN_TRIES} tries keeps {GOLDEN_MARGIN} of an 8-bit step from a \
                 rounding tie"
            ));
        }
    }
    Ok(scene)
}

/// `colours`, linear RGB, quantised as the golden runner's quantise pass does: each channel clamped to `[0, 1]`,
/// scaled to 0..255 and rounded half to even (`xtask::golden`; REQ-VAL-138).
pub fn quantise(colours: &[[f64; 3]]) -> Vec<[u8; 3]> {
    colours
        .iter()
        .map(|c| c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round_ties_even() as u8))
        .collect()
}
