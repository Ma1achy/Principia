//! The golden harness's shape-sphere scenes (`shape-sphere` and `shape-sphere-projection`; R-113; TASK-M2-28): the
//! shape-sphere chart drawn through a colour occupant of its own ([`WGSL`]), one sample per display tile, rendered by
//! `golden_harness` as every `golden_scene` scene is, through the render harness. [`ShapeScene::expected`] is its CPU
//! twin, from the read side's `n` of each sample (`Scene::read`).
//!
//! **What a sample holds.** Each display tile shows one chart sample (chart_reference §3.3): its display point through
//! the projection ([`Projection::chart_point`]) gives the chart point `(s, t)`, and the sample is the one whose chart
//! cell, of [`CHART`]'s grid over `[0,1]²`, holds it, decoded at that cell's centre, with no interpolation. The sample
//! holds `Φ`'s own configuration there (`Decoded::config`, before `C` folds the lower hemisphere onto the upper, R-141),
//! with the chart's masses, so the read side's `n` is the chart's point. A display tile showing no chart point, outside
//! Hammer's ellipse, is not a system: the chart's classification marks it (`ctx.validity.forbidden`) and the occupant
//! draws the debug views' hatch there (R-470: a debug view draws such a pixel with the hatch, never a palette colour).
//!
//! **What the occupant draws** ([`WGSL`]): the quadrant of `n`'s azimuth, the signs of `n.x` and `n.y`, as Okabe–Ito
//! classes 1 to 4 (`dbg_cat`); where the chart is n-to-1 over its φ hemispheres (`u_fold` 1, R-141), the redundant
//! hemisphere, `n.z < 0`, whose systems the `w ≥ 0` hemisphere holds, carries the redundancy flag, black diagonal
//! stripes 2 px wide. The chart's label ([`ShapeSphere::label`]) says the same in words.
//!
//! **The scenes** ([`NAMES`]):
//! - `shape_equirectangular`: the spherical map in the default projection, both hemispheres drawn and the redundancy
//!   flagged: the `shape-sphere` suite's `both_flagged` and the `shape-sphere-projection` suite's `equirectangular`.
//! - `shape_equal_area`: the same chart under Hammer's equal-area projection, a display remap, the ellipse's outside
//!   hatched: `shape-sphere-projection`'s `equal_area`.
//! - `shape_one_hemisphere`: the exponential map about a point near `L⁺` whose cap lies in `w ≥ 0`, bijective, one
//!   labelled hemisphere, no flag: `shape-sphere`'s `one_hemisphere`.

use kernel::chart::shape_sphere::{Projection, ShapeMap, ShapeSphere};
use kernel::chart::Chart;
use kernel::chart::SystemImage;
use kernel::generated::links::Defaults;
use render::present::{self, Rgb};
use render::raster::Grid;

use crate::golden_scene::{Colouring, Scene};
use engine::synthetic::Synthetic;

/// The scenes, by name.
pub const NAMES: [&str; 3] = [
    "shape_equirectangular",
    "shape_equal_area",
    "shape_one_hemisphere",
];

/// The colour occupant: the quadrant of `n`'s azimuth, the redundancy flag, and the hatch where no chart point shows.
pub const WGSL: &str = include_str!("shape_scene.wgsl");

/// The spherical map's chart grid, `[θ cells, φ cells]` over `[0,1]²`.
pub const CHART: [u32; 2] = [32, 16];

/// The spherical map's display grid, in tiles, and each tile's side in pixels: 128 × 64 px, two display tiles to a
/// chart cell along each axis under the default projection.
pub const DISPLAY: [u32; 2] = [64, 32];

/// The exponential map's grid, chart and display alike, and its tiles' side.
pub const CAP_GRID: [u32; 2] = [32, 32];

/// Every display tile's side in pixels.
pub const TILE_PX: u32 = 2;

/// A scene with the chart it draws.
pub struct ShapeScene {
    /// The rendered scene: its set, context and colouring.
    pub scene: Scene,
    /// The chart drawn.
    pub chart: ShapeSphere<f64>,
    /// Whether the redundancy flag is drawn: the chart n-to-1 over its φ hemispheres.
    pub fold: bool,
    /// Each sample's masses, as its `ICDescriptor` holds them: zero where the tile shows no chart point.
    pub masses: Vec<[f32; 3]>,
}

/// The spherical map: `I = 1`, `φ_f = 0`, equal masses at rest.
pub fn spherical() -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Spherical,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 1.0,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.0, 0.0],
        z_mom: [0.0; 4],
    }
}

/// The exponential map about the point at polar angle 0.3 from `L⁺` toward `+u`, `e₁ = ŷ`, `e₂` along the meridian
/// toward `L⁺`, extent `s = 0.7`: its cap holds `L⁺` and lies in `w ≥ 0`, its farthest corner at polar angle
/// `0.3 + 0.7√2 < π/2`.
pub fn one_hemisphere() -> ShapeSphere<f64> {
    let alpha: f64 = 0.3;
    ShapeSphere {
        map: ShapeMap::Exponential,
        n0: [alpha.sin(), 0.0, alpha.cos()],
        e1: [0.0, 1.0, 0.0],
        e2: [-alpha.cos(), 0.0, alpha.sin()],
        scale: 0.7,
        ..spherical()
    }
}

/// The centre of tile `i` of `n`: `(i + ½)/n`.
fn centre(i: u32, n: u32) -> f64 {
    (f64::from(i) + 0.5) / f64::from(n)
}

/// The centre of the chart cell of `cells` holding the coordinate `c ∈ [0, 1]`: cell `⌊c·cells⌋`, the square's far
/// edge, `c = 1`, in the last cell.
pub fn cell_centre(c: f64, cells: u32) -> f64 {
    let k = ((c * f64::from(cells)).floor() as u32).min(cells - 1);
    centre(k, cells)
}

/// `chart` over a `display` grid of tiles through `projection` onto a `cells` chart grid: each tile's sample the chart
/// cell's holding its chart point, decoded at the cell's centre, or marked as no system where none shows.
fn fill(
    chart: &ShapeSphere<f64>,
    display: [u32; 2],
    cells: [u32; 2],
    projection: Projection,
) -> Result<(Grid, Synthetic, Vec<[f32; 3]>), String> {
    let grid = Grid::new(display, 1, 0, TILE_PX)?;
    let mut set = Synthetic::flat(grid, 0);
    let mut masses = vec![[0.0; 3]; grid.sample_count() as usize];
    for row in 0..display[1] {
        for col in 0..display[0] {
            let i = row * display[0] + col;
            let Some([s, t]) =
                projection.chart_point(centre(col, display[0]), centre(row, display[1]))
            else {
                set.set_forbidden(i, true);
                continue;
            };
            let (s, t) = (cell_centre(s, cells[0]), cell_centre(t, cells[1]));
            let d = chart.decode::<Defaults, f64>(s, t);
            let r = d.config.r.map(|p| p.map(|c| c as f32));
            let m = d.masses.m.map(|m| m as f32);
            set.sample(i).r(r);
            let ic = set.ic(i);
            (ic.m0, ic.m1, ic.m2) = (m[0], m[1], m[2]);
            masses[i as usize] = m;
        }
    }
    Ok((grid, set, masses))
}

/// The scene `name`, or why not.
pub fn scene(name: &str) -> Result<ShapeScene, String> {
    let name: &'static str = NAMES.iter().find(|n| **n == name).ok_or_else(|| {
        format!(
            "no shape-sphere scene `{name}`; the scenes are {}",
            NAMES.join(", ")
        )
    })?;
    let (chart, (grid, set, masses)) = match name {
        "shape_equirectangular" => {
            let c = spherical();
            (c, fill(&c, DISPLAY, CHART, Projection::Equirectangular)?)
        }
        "shape_equal_area" => {
            let c = spherical();
            (c, fill(&c, DISPLAY, CHART, Projection::EqualArea)?)
        }
        _ => {
            let c = one_hemisphere();
            (
                c,
                fill(&c, CAP_GRID, CAP_GRID, Projection::Equirectangular)?,
            )
        }
    };
    let fold = chart.system_image() == SystemImage::NToOne(2);
    let params = vec![("u_fold".to_owned(), vec![f64::from(u8::from(fold))])];
    Ok(ShapeScene {
        scene: Scene {
            name,
            set,
            context: crate::golden_scene::context(grid),
            colouring: Colouring::Probe(WGSL.to_owned(), params),
        },
        chart,
        fold,
        masses,
    })
}

impl ShapeScene {
    /// The redundancy flag's stripe at the pixel `(x, y)`, from the top left: stripe `⌊(x + y)/2⌋` even.
    pub fn stripe(x: u32, y: u32) -> bool {
        ((x + y) / 2).is_multiple_of(2)
    }

    /// Each sample's shape `n` as the read side derives it, with the sample's own masses ([`ShapeScene::masses`]):
    /// `Scene::read_with`, the fragment's unpack on the CPU.
    pub fn shapes(&self) -> Vec<[f32; 3]> {
        (0..self.scene.context.grid.sample_count())
            .map(|i| self.scene.read_with(i, self.masses[i as usize]).n)
            .collect()
    }

    /// The render's CPU twin: each pixel's linear RGB, rows from the top (the module's notes).
    pub fn expected(&self) -> Vec<Rgb> {
        let (width, height) = self.scene.size();
        let grid = self.scene.context.grid;
        let shapes = self.shapes();
        let mut out = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                let i = grid.cell(x, y).sample;
                let frag = [f64::from(x) + 0.5, f64::from(y) + 0.5];
                out.push(if self.scene.set.forbidden(i) {
                    present::debug_invalid(frag)
                } else {
                    let n = shapes[i as usize];
                    let q = u32::from(n[0] < 0.0) + 2 * u32::from(n[1] < 0.0);
                    if self.fold && n[2] < 0.0 && Self::stripe(x, y) {
                        [0.0; 3]
                    } else {
                        present::dbg_cat(q + 1, 8)
                    }
                });
            }
        }
        out
    }
}
