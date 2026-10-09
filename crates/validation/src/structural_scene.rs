//! The golden harness's structural scenes (`m1-structural`, and the structural views' cases in `debug-views`; RQ-229,
//! RQ-237; TASK-M1-13): a structural synthetic set (`engine::synthetic::Synthetic::structural`), its context uniforms,
//! and a stain graph from the structural presets (`engine::structural`): a `ctx.quad` debug view, or the flat mid-grey,
//! with the post occupants after it. `golden_harness` renders one as it renders a `golden_scene` scene, through the
//! render harness (`render::bind::preset_module` and `upload`), and [`StructuralScene::expected`] is its CPU twin, from
//! the set's CPU `RenderQuad` words through `render::structural`, the occupants' CPU mirror.
//!
//! **The scenes.**
//! - The views, `s_depth`, `s_state`, `s_coherence`, `s_impurity`, `s_spread`, `s_suspect`, `s_priority`, `s_cache_age`
//!   and `s_ancestor_gap`: each over the structural set at depths 3 and 20, quad by quad, 5 × 2 quads of 2 × 2 tiles of
//!   8 px; a scalar view's `u_range` the measured range of its value over the quads. `s_spread` at E = 0 draws the hatch
//!   everywhere (`has_ensemble()` false); `s_spread_ensemble`, at E = 1, the ramp.
//! - `fallback_tint`, `pending_hatch`: the quad-state view with the fallback tint (`ancestor_gap > 0`) or the pending
//!   hatch (`quad_state == 1`) after it; `overlays`: the quad-state view under both boundary levels, the tint and the
//!   hatch.
//! - `quad_boundaries`, `tile_boundaries`: the boundary overlay at each level over the flat mid-grey, on the structural
//!   set at depths 3 and 20, 5 × 2 quads of 4 × 4 tiles of 4 px.
//! - The width cases (REQ-RENDER-024): `width_d<ℓ>_q<s>`, the quad boundaries over the flat mid-grey on the depth-ℓ set
//!   (ℓ 3 or 20), 2 × 2 quads of 4 × 4 tiles, each quad `s` px square (16 or 64), shown as the 16 × 16 window centred on
//!   the grid's inner corner. A line of one width in pixels draws one window at every quad size and depth, so the four
//!   cases share one reference image; `threshold_q16` and `threshold_q64` draw the thresholded-uv control (`u < 0.01`,
//!   render_gui_spec §12.1) in the same windows, and differ ([`line_width`] measures each).

use engine::stain::{GraphError, Occupant, StainGraph};
use engine::structural::{self, Level};
use engine::synthetic::Synthetic;
use render::assemble::Stain;
use render::bind::Context;
use render::colour::combine::MID_GREY_L;
use render::present::{self, Rgb};
use render::raster::Grid;
use render::structural::{self as mirror, QuadMeta};

use crate::golden_scene::render_stain;

/// The scenes, by name.
pub const NAMES: [&str; 21] = [
    "s_depth",
    "s_state",
    "s_coherence",
    "s_impurity",
    "s_spread",
    "s_spread_ensemble",
    "s_suspect",
    "s_priority",
    "s_cache_age",
    "s_ancestor_gap",
    "fallback_tint",
    "pending_hatch",
    "overlays",
    "quad_boundaries",
    "tile_boundaries",
    "width_d3_q16",
    "width_d3_q64",
    "width_d20_q16",
    "width_d20_q64",
    "threshold_q16",
    "threshold_q64",
];

/// The thresholded-uv control (render_gui_spec §12.1: thresholding UV directly, `u < 0.01`, "would instead give fat
/// borders on coarse quads and hairlines on deep ones"): a custom post drawing white where the quad-local `uv` lies
/// within 0.01 of an edge. Not an occupant: the width check's control only.
pub const THRESHOLD_POST: &str = "// The thresholded-uv control (render_gui_spec §12.1): white within 0.01 of a quad edge, in uv.\n\
     fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {\n    \
     let uv = ctx.quad.uv;\n    \
     let d = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));\n    \
     return select(rgb, vec3<f32>(1.0), d < 0.01);\n}\n";

/// The boundary overlay's params, as a scene sets every one (`frag/post/edge_line.wgsl`'s uniforms).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edge {
    pub level: Level,
    pub width: f64,
    pub opacity: f64,
    pub tile_width: f64,
    pub tile_opacity: f64,
    pub colour: Rgb,
}

/// The overlay as the scenes draw it: white, the quad lines 2 px wide and opaque, the tile lines 1 px at half opacity.
pub const fn edge(level: Level) -> Edge {
    Edge {
        level,
        width: 2.0,
        opacity: 1.0,
        tile_width: 1.0,
        tile_opacity: 0.5,
        colour: [1.0, 1.0, 1.0],
    }
}

/// The overlay in the `overlays` scene: both levels, in a light grey, the quad lines 2.25 px wide, the tile lines 1 px
/// at 0.55 opacity, chosen so that every pixel of the scene lies at least 0.01 of an 8-bit step from a rounding tie
/// (R-296), which white 2 px quad lines over half-opaque tile lines do not.
pub const OVERLAYS_EDGE: Edge = Edge {
    level: Level::Both,
    width: 2.25,
    opacity: 1.0,
    tile_width: 1.0,
    tile_opacity: 0.55,
    colour: [0.95, 0.95, 0.95],
};

/// A post a scene appends, in order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Post {
    /// The boundary overlay, `edge_line`.
    Edge(Edge),
    /// The fallback tint, `fallback_tint`.
    Tint,
    /// The pending hatch, `pending_hatch`.
    Pending,
    /// The thresholded-uv control ([`THRESHOLD_POST`]).
    Threshold,
}

/// One structural scene.
#[derive(Debug)]
pub struct StructuralScene {
    pub name: &'static str,
    pub set: Synthetic,
    pub context: Context,
    /// The structural view in the colour slot, or none: the flat mid-grey.
    pub view: Option<&'static str>,
    pub posts: Vec<Post>,
    /// The window of the target the scene shows, `[x0, y0, width, height]`, rows from the top; the whole target when
    /// `None`.
    pub crop: Option<[u32; 4]>,
}

/// The context every scene reads with: `golden_scene`'s, its valid sample count every tile's.
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

/// `name`'s scene, or why not.
pub fn scene(name: &str) -> Result<StructuralScene, String> {
    let name: &'static str = NAMES.iter().find(|n| **n == name).ok_or_else(|| {
        format!(
            "no structural scene `{name}`; the scenes are {}",
            NAMES.join(", ")
        )
    })?;
    let views = |e: u32| Grid::new([5, 2], 2, e, 8);
    let width = |depth: u32, quad_px: u32| -> Result<StructuralScene, String> {
        let grid = Grid::new([2, 2], 4, 0, quad_px / 4)?;
        Ok(StructuralScene {
            name,
            set: Synthetic::structural(grid, &[depth])?,
            context: context(grid),
            view: None,
            posts: vec![if name.starts_with("threshold") {
                Post::Threshold
            } else {
                Post::Edge(edge(Level::Quad))
            }],
            crop: Some([quad_px - 8, quad_px - 8, 16, 16]),
        })
    };
    let (grid, view, posts) = match name {
        "s_spread_ensemble" => (views(1)?, Some("s_spread"), vec![]),
        "fallback_tint" => (views(0)?, Some("s_state"), vec![Post::Tint]),
        "pending_hatch" => (views(0)?, Some("s_state"), vec![Post::Pending]),
        "overlays" => (
            views(0)?,
            Some("s_state"),
            vec![Post::Edge(OVERLAYS_EDGE), Post::Tint, Post::Pending],
        ),
        "quad_boundaries" | "tile_boundaries" => {
            let level = if name == "quad_boundaries" {
                Level::Quad
            } else {
                Level::Tile
            };
            (
                Grid::new([5, 2], 4, 0, 4)?,
                None,
                vec![Post::Edge(edge(level))],
            )
        }
        "width_d3_q16" | "threshold_q16" => return width(3, 16),
        "width_d3_q64" | "threshold_q64" => return width(3, 64),
        "width_d20_q16" => return width(20, 16),
        "width_d20_q64" => return width(20, 64),
        view => (views(0)?, Some(view), vec![]),
    };
    Ok(StructuralScene {
        name,
        set: Synthetic::structural(grid, &[3, 20])?,
        context: context(grid),
        view,
        posts,
        crop: None,
    })
}

impl StructuralScene {
    /// The shown window's size.
    pub fn size(&self) -> (u32, u32) {
        match self.crop {
            Some([_, _, w, h]) => (w, h),
            None => self.context.grid.target(),
        }
    }

    /// The shown window, `[x0, y0, width, height]` of the target.
    fn window(&self) -> [u32; 4] {
        let (w, h) = self.context.grid.target();
        self.crop.unwrap_or([0, 0, w, h])
    }

    /// Quad `q`'s `RenderQuad`, from the set's CPU words.
    pub fn quad(&self, q: u32) -> QuadMeta {
        QuadMeta::from_words(&self.set.quad_words(q))
    }

    /// The view's `u_range`: the least and greatest of its value over the scene's quads, `None` for no view or the
    /// categorical `s_state`.
    pub fn u_range(&self) -> Option<[f64; 2]> {
        let id = self.view?;
        let values: Vec<f64> = (0..self.context.grid.quad_count())
            .filter_map(|q| mirror::scalar(id, &self.quad(q)).map(|(v, ..)| v))
            .collect();
        let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        (!values.is_empty()).then_some([lo, hi])
    }

    /// The scene's stain graph: the view's preset ([`structural::view`]) with its `u_range` set, or the backbone alone,
    /// then each post, the overlay's every param set.
    pub fn graph(&self) -> Result<StainGraph, GraphError> {
        let mut g = match self.view {
            Some(id) => structural::view(id)?,
            None => StainGraph::new(),
        };
        if let (Some(range), Some(node)) = (self.u_range(), structural::colour_node(&g)) {
            g.set_param(node, "u_range", range.to_vec())?;
        }
        for post in &self.posts {
            let occupant = |id: &str| Occupant::Builtin(id.to_owned());
            match post {
                Post::Edge(e) => {
                    let p = structural::overlay(&mut g, occupant("edge_line"))?;
                    g.set_param(p, "level", vec![f64::from(e.level as u32)])?;
                    g.set_param(p, "width", vec![e.width])?;
                    g.set_param(p, "opacity", vec![e.opacity])?;
                    g.set_param(p, "tile_width", vec![e.tile_width])?;
                    g.set_param(p, "tile_opacity", vec![e.tile_opacity])?;
                    g.set_param(p, "colour", e.colour.to_vec())?;
                }
                Post::Tint => {
                    structural::overlay(&mut g, occupant("fallback_tint"))?;
                }
                Post::Pending => {
                    structural::overlay(&mut g, occupant("pending_hatch"))?;
                }
                Post::Threshold => {
                    structural::overlay(&mut g, Occupant::Custom(THRESHOLD_POST.to_owned()))?;
                }
            }
        }
        Ok(g)
    }

    /// The stain the graph lowers to.
    pub fn stain(&self) -> Result<Stain, String> {
        self.graph()
            .and_then(|g| g.lower())
            .map_err(|e| e.to_string())
    }

    /// Renders the scene through the render harness ([`render_stain`]), each node's uniforms its params in the graph's
    /// canonical form, into an `Rgba32Float` target, and returns the shown window's pixels' RGBA, rows from the top.
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Vec<[f32; 4]>, String> {
        let canonical = self
            .graph()
            .and_then(|g| g.canonical())
            .map_err(|e| e.to_string())?;
        let full = render_stain(
            device,
            queue,
            canonical.stain(),
            &self.set,
            &self.context,
            &|node, name| canonical.nodes.get(node)?.params.get(name).cloned(),
        )?;
        let (width, _) = self.context.grid.target();
        let [x0, y0, w, h] = self.window();
        Ok((y0..y0 + h)
            .flat_map(|y| (x0..x0 + w).map(move |x| (x, y)))
            .map(|(x, y)| full[(y * width + x) as usize])
            .collect())
    }

    /// The render's CPU twin: the shown window's pixels' linear RGB, rows from the top, from the set's CPU
    /// `RenderQuad` words and the raster's cell coordinates, through the occupants' mirror (`render::structural`).
    pub fn expected(&self) -> Result<Vec<Rgb>, String> {
        let [x0, y0, w, h] = self.window();
        (y0..y0 + h)
            .flat_map(|y| (x0..x0 + w).map(move |x| (x, y)))
            .map(|(x, y)| self.pixel(x, y))
            .collect()
    }

    /// The twin of the target's pixel `(x, y)`, `y` from the top.
    pub fn pixel(&self, x: u32, y: u32) -> Result<Rgb, String> {
        let grid = self.context.grid;
        let (_, height) = grid.target();
        let cell = grid.cell(x, y);
        let q = self.quad(cell.quad);
        let frag = [f64::from(x) + 0.5, f64::from(y) + 0.5];
        // The pixel's centre, Y-up, and its tile's bottom-left corner, in pixels: the raster's cell coordinates.
        let up = [frag[0], f64::from(height) - frag[1]];
        let t = f64::from(grid.tile_px);
        let n = f64::from(grid.n);
        let tile_uv = [0, 1].map(|k| {
            let column = cell.quad_xy[k] * grid.n + cell.tile_xy[k];
            (up[k] - f64::from(column) * t) / t
        });
        let quad_uv = [0, 1].map(|k| (f64::from(cell.tile_xy[k]) + tile_uv[k]) / n);
        let mut rgb = match self.view {
            Some(id) => mirror::view(
                id,
                &q,
                frag,
                grid.e >= 1,
                mirror::range_auto(id),
                self.u_range().unwrap_or([0.0, 1.0]),
            )
            .ok_or_else(|| format!("`{id}` is no structural view"))?,
            None => present::ramp_grey(MID_GREY_L),
        };
        for post in &self.posts {
            rgb = match post {
                Post::Edge(e) => {
                    let quad = e.opacity * mirror::edge_line(quad_uv, n * t, e.width);
                    let tile = e.tile_opacity * mirror::edge_line(tile_uv, t, e.tile_width);
                    let a = match e.level {
                        Level::Quad => quad,
                        Level::Tile => tile,
                        Level::Both => quad.max(tile),
                    };
                    mirror::mix(rgb, e.colour, a)
                }
                Post::Tint => mirror::fallback_tint(rgb, q.ancestor_gap),
                Post::Pending => mirror::pending_hatch(rgb, q.state, frag),
                Post::Threshold => {
                    let d = quad_uv.map(|u| u.min(1.0 - u));
                    if d[0].min(d[1]) < 0.01 {
                        [1.0; 3]
                    } else {
                        rgb
                    }
                }
            };
        }
        Ok(rgb)
    }
}

/// The width in pixels of the line crossing row `row` of `image`, `width` pixels wide: the sum along the row of each
/// pixel's coverage, how far its red channel lies from `background`'s toward `line`'s. An antialiased line of full
/// width `w` at half coverage sums to `w`.
pub fn line_width(image: &[Rgb], width: u32, row: u32, background: Rgb, line: Rgb) -> f64 {
    let at = (row * width) as usize;
    image[at..at + width as usize]
        .iter()
        .map(|p| (p[0] - background[0]) / (line[0] - background[0]))
        .sum()
}
