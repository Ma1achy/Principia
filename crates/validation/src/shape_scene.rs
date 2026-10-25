//! The golden harness's shape-sphere scenes (`shape-sphere` and `shape-sphere-projection`; R-113, R-544; TASK-M2-28):
//! the shape-sphere chart drawn through a colour occupant of its own ([`WGSL`]), one sample per display tile, rendered
//! by `golden_harness` as every `golden_scene` scene is, through the render harness. [`ShapeScene::expected`] is its
//! CPU twin, from the read side's `n` of each sample (`Scene::read`).
//!
//! **What a sample holds.** Each display tile shows one chart sample (chart_reference §3.3): its display point through
//! the projection ([`Projection::chart_point`]) gives the chart point `(s, t)`, and the sample is the one whose chart
//! cell, of the scene's chart grid over `[0,1]²`, holds it, decoded at that cell's centre, with no interpolation. The
//! sample holds the decode's own state, `Decoded::ic`, the system whose signed `n̂` is the chart's point (R-544: `C`'s
//! mirror step not taken, nothing folded), with its masses, so the read side's `n` is the chart's point in either
//! hemisphere. A display tile showing no chart point, outside Hammer's ellipse, is not a system: the chart's
//! classification marks it (`ctx.validity.forbidden`) and the occupant draws the debug views' hatch there (R-470: a debug
//! view draws such a pixel with the hatch, never a palette colour).
//!
//! **What the occupant draws** ([`WGSL`]): the octant of `n`, the signs of `n.x`, `n.y` and `n.z`, as Okabe–Ito classes
//! 1 to 8 (`dbg_cat`): the quadrant of its azimuth, classes 1 to 4 above the equator and 5 to 8 below it, so both
//! hemispheres are drawn and told apart, and nothing marks either as redundant (R-544; REQ-CHART-002). The chart's
//! label ([`ShapeSphere::label`]) names no hemisphere.
//!
//! **The scenes** ([`NAMES`]):
//! - `shape_spherical`: the spherical map in the default projection, the whole sphere, pole to pole: the
//!   `shape-sphere` suite's `spherical` and the `shape-sphere-projection` suite's `equirectangular`.
//! - `shape_equal_area`: the same chart under Hammer's equal-area projection, a display remap, both hemispheres drawn,
//!   the ellipse's outside hatched: `shape-sphere-projection`'s `equal_area`.
//! - `shape_exponential_straddling`: the exponential map about a point of the equator whose cap straddles it, both
//!   halves drawn, each pixel its own system: `shape-sphere`'s `exponential_straddling`.
//! - The controls, each rendered against `spherical`'s reference by a case that expects to fail:
//!   `shape_control_one_hemisphere`, one hemisphere drawn across the whole square (R-412's withdrawn remap,
//!   `φ = (π/2)·(1 − t)`), and `shape_control_flagged`, the whole sphere with the lower half striped as redundant
//!   (R-141's withdrawn flag, black diagonal stripes 2 px wide where `n.z < 0`; `u_flag` 1, which no chart scene sets).

use kernel::chart::shape_sphere::{Projection, ShapeMap, ShapeSphere};
use kernel::chart::Chart;
use kernel::generated::links::Defaults;
use render::present::{self, Rgb};
use render::raster::Grid;

use crate::golden_scene::{Colouring, Scene};
use engine::synthetic::Synthetic;

/// The scenes, by name.
pub const NAMES: [&str; 5] = [
    "shape_spherical",
    "shape_equal_area",
    "shape_exponential_straddling",
    "shape_control_one_hemisphere",
    "shape_control_flagged",
];

/// The colour occupant: the octant of `n`, the control's redundancy stripes, and the hatch where no chart point shows.
pub const WGSL: &str = include_str!("shape_scene.wgsl");

/// The spherical map's chart grid, `[θ cells, φ cells]` over `[0,1]²`.
pub const CHART: [u32; 2] = [32, 16];

/// The spherical map's display grid, in tiles: 128 × 64 px, two display tiles to a chart cell along each axis under
/// the default projection.
pub const DISPLAY: [u32; 2] = [64, 32];

/// The exponential map's grid, chart and display alike.
pub const CAP_GRID: [u32; 2] = [32, 32];

/// Every display tile's side in pixels.
pub const TILE_PX: u32 = 2;

/// How a scene draws its chart: the chart's own render, or one of the two controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Draw {
    /// The chart's own render: the whole sphere, or the cap, each sample the chart's point.
    Chart,
    /// The one-hemisphere control: the chart point `(s, t)` drawn from `(s, (1 + t)/2)`, `φ`'s span remapped to end at
    /// the equator (R-412, withdrawn by R-544).
    OneHemisphere,
    /// The flagged control: the chart's own samples, the lower half, `n.z < 0`, striped as redundant (R-141, withdrawn
    /// by R-544).
    Flagged,
}

/// A scene with the chart it draws.
pub struct ShapeScene {
    /// The rendered scene: its set, context and colouring.
    pub scene: Scene,
    /// The chart drawn.
    pub chart: ShapeSphere<f64>,
    /// The projection it is drawn through.
    pub projection: Projection,
    /// How it is drawn.
    pub draw: Draw,
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

/// The exponential map about `n₀ = x̂`, a point of the equator, `e₁ = ŷ`, `e₂ = ŵ`, extent `s = 1` ([`spherical`]'s):
/// its cap straddles the equator, the top half of the square at `w > 0` and the bottom half at `w < 0`, and lies in
/// `n.x > 0`, its farthest corner at `r = √2 < π/2`.
pub fn straddling() -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Exponential,
        n0: [1.0, 0.0, 0.0],
        e1: [0.0, 1.0, 0.0],
        e2: [0.0, 0.0, 1.0],
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

/// The chart point display tile `(col, row)` of a `display` grid shows through `projection` onto a `cells` chart grid,
/// drawn as `draw` draws it: its display point's chart point, the chart cell holding it, that cell's centre; `None`
/// where it shows none.
pub fn shown(
    projection: Projection,
    draw: Draw,
    display: [u32; 2],
    cells: [u32; 2],
    col: u32,
    row: u32,
) -> Option<[f64; 2]> {
    let [s, t] = projection.chart_point(centre(col, display[0]), centre(row, display[1]))?;
    let t = match draw {
        Draw::OneHemisphere => (1.0 + t) / 2.0,
        Draw::Chart | Draw::Flagged => t,
    };
    Some([cell_centre(s, cells[0]), cell_centre(t, cells[1])])
}

/// `chart` over a `display` grid of tiles through `projection` onto a `cells` chart grid, drawn as `draw` draws it:
/// each tile's sample the decode's state at the chart point it shows ([`shown`]), or marked as no system where none
/// shows.
fn fill(
    chart: &ShapeSphere<f64>,
    display: [u32; 2],
    cells: [u32; 2],
    projection: Projection,
    draw: Draw,
) -> Result<(Grid, Synthetic, Vec<[f32; 3]>), String> {
    let grid = Grid::new(display, 1, 0, TILE_PX)?;
    let mut set = Synthetic::flat(grid, 0);
    let mut masses = vec![[0.0; 3]; grid.sample_count() as usize];
    for row in 0..display[1] {
        for col in 0..display[0] {
            let i = row * display[0] + col;
            let Some([s, t]) = shown(projection, draw, display, cells, col, row) else {
                set.set_forbidden(i, true);
                continue;
            };
            let d = chart.decode::<Defaults, f64>(s, t);
            let r = d.ic.r.map(|p| p.map(|c| c as f32));
            let m = d.ic.m.map(|m| m as f32);
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
    let (chart, projection, draw) = match name {
        "shape_spherical" => (spherical(), Projection::Equirectangular, Draw::Chart),
        "shape_equal_area" => (spherical(), Projection::EqualArea, Draw::Chart),
        "shape_exponential_straddling" => (straddling(), Projection::Equirectangular, Draw::Chart),
        "shape_control_one_hemisphere" => (
            spherical(),
            Projection::Equirectangular,
            Draw::OneHemisphere,
        ),
        _ => (spherical(), Projection::Equirectangular, Draw::Flagged),
    };
    let (display, cells) = match chart.map {
        ShapeMap::Spherical => (DISPLAY, CHART),
        ShapeMap::Exponential => (CAP_GRID, CAP_GRID),
    };
    let (grid, set, masses) = fill(&chart, display, cells, projection, draw)?;
    let flag = draw == Draw::Flagged;
    let params = vec![("u_flag".to_owned(), vec![f64::from(u8::from(flag))])];
    Ok(ShapeScene {
        scene: Scene {
            name,
            set,
            context: crate::golden_scene::context(grid),
            colouring: Colouring::Probe(WGSL.to_owned(), params),
        },
        chart,
        projection,
        draw,
        masses,
    })
}

impl ShapeScene {
    /// The control's redundancy stripe at the pixel `(x, y)`, from the top left: stripe `⌊(x + y)/2⌋` even.
    pub fn stripe(x: u32, y: u32) -> bool {
        ((x + y) / 2).is_multiple_of(2)
    }

    /// The chart grid the scene samples, and its display grid.
    pub fn grids(&self) -> ([u32; 2], [u32; 2]) {
        match self.chart.map {
            ShapeMap::Spherical => (CHART, DISPLAY),
            ShapeMap::Exponential => (CAP_GRID, CAP_GRID),
        }
    }

    /// Each sample's shape `n` as the read side derives it, with the sample's own masses ([`ShapeScene::masses`]):
    /// `Scene::read_with`, the fragment's unpack on the CPU.
    pub fn shapes(&self) -> Vec<[f32; 3]> {
        (0..self.scene.context.grid.sample_count())
            .map(|i| self.scene.read_with(i, self.masses[i as usize]).n)
            .collect()
    }

    /// The occupant's class of the shape `n`: the quadrant of its azimuth, `[n.x < 0] + 2·[n.y < 0]`, plus 4 below the
    /// equator, `n.z < 0`; 0 to 7.
    pub fn class(n: [f32; 3]) -> u32 {
        u32::from(n[0] < 0.0) + 2 * u32::from(n[1] < 0.0) + 4 * u32::from(n[2] < 0.0)
    }

    /// The render's CPU twin: each pixel's linear RGB, rows from the top (the module's notes).
    pub fn expected(&self) -> Vec<Rgb> {
        let (width, height) = self.scene.size();
        let grid = self.scene.context.grid;
        let shapes = self.shapes();
        let flag = self.draw == Draw::Flagged;
        let mut out = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                let i = grid.cell(x, y).sample;
                let frag = [f64::from(x) + 0.5, f64::from(y) + 0.5];
                out.push(if self.scene.set.forbidden(i) {
                    present::debug_invalid(frag)
                } else {
                    let n = shapes[i as usize];
                    if flag && n[2] < 0.0 && Self::stripe(x, y) {
                        [0.0; 3]
                    } else {
                        present::dbg_cat(Self::class(n) + 1, 8)
                    }
                });
            }
        }
        out
    }
}
