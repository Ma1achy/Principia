//! The coordinate presets (colour_composition §6, "Debug views as presets"; debug_tooling_plan §A's UV preset and §F's
//! UV passthrough; TASK-M1-07): in-code stain graphs, not editable and with no file format (RQ-215; the serialised
//! format and locked loading are TASK-M7-17's, REQ-GUI-028 and REQ-GUI-029).
//!
//! Each is a stain graph on the fixed backbone: a source holding a coordinate as a vector field, a colour drawing it
//! as RG with B = 0, the built-in pass-through combiner, and `OUT`. Every coordinate is the post-flip one, Y-up, read
//! from `ctx` (`render::coords`; the raster fills the lanes):
//! - [`uv_screen`]: `ctx.screen.uv → RG`, the pixel's UV in the view (fragment addressing);
//! - [`uv_quad`]: `ctx.quad.uv → RG`, the sample's quad-local coordinate `t` (structural addressing);
//! - [`coordinate_view`] in [`CoordinateMode::Uv`], the UV passthrough: each sample's UV coordinate reconstructed from
//!   `t` as deep_zoom §1 writes it, `u = c + h·(2t − 1)` and `v` likewise, `u → R`, `v → G` (REQ-TOOL-019,
//!   REQ-TOOL-027);
//! - [`coordinate_view`] in [`CoordinateMode::Delta`], its quad-local δ mode: δ = `h·(2t − 1)`, deep_zoom §2's
//!   quad-local offset `u − c`, each component normalised by its bound `h = 2^−(ℓ+1)` to [−1, 1] and mapped to
//!   `(x + 1)/2`, `u`'s in R and `v`'s in G, B = 0 (REQ-TOOL-153; debug_tooling_plan §F).
//!
//! [`uv_screen`]: crate::presets::uv_screen
//! [`uv_quad`]: crate::presets::uv_quad
//! [`coordinate_view`]: crate::presets::coordinate_view
//! [`CoordinateMode::Uv`]: crate::presets::CoordinateMode::Uv
//! [`CoordinateMode::Delta`]: crate::presets::CoordinateMode::Delta

use crate::stain::{GraphError, NodeId, NodeKind, Occupant, StainGraph};

/// The coordinate view's two modes (debug_tooling_plan §F's UV passthrough).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordinateMode {
    /// The sample's reconstructed UV coordinate, `c + h·(2t − 1)`, as RG.
    Uv,
    /// The quad-local offset δ = `h·(2t − 1)`, normalised by `h` and mapped to `(δ/h + 1)/2`, as RG.
    Delta,
}

/// The colour that draws its field input's first two components as R and G, with B = 0.
fn rg() -> String {
    "// The field's first two components as R and G, B = 0 (TASK-M1-07's coordinate presets).\n\
     fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].xy, 0.0); }\n"
        .to_owned()
}

/// A source holding `value`, a `vec2<f32>` expression over `ctx`, in the field's `.xy`.
fn source(what: &str, value: &str) -> String {
    format!(
        "// {what}, a vector field in `.xy` (TASK-M1-07's coordinate presets).\n\
         fn source(ctx: Ctx) -> Field {{ return Field({value}, 0.0, 0.0); }}\n"
    )
}

/// The graph: `source` into `colour`, `colour` into the pass-through combiner's colour port.
fn graph(source: String, colour: String) -> Result<StainGraph, GraphError> {
    let combiner = NodeId(0);
    let mut g = StainGraph::new();
    let s = g.add(NodeKind::Source, Occupant::Custom(source))?;
    let c = g.add(NodeKind::Colour, Occupant::Custom(colour))?;
    g.connect(s, c, 0)?;
    g.connect(c, combiner, 0)?;
    Ok(g)
}

/// `uv_screen`: the pixel's post-flip UV in the view, `ctx.screen.uv → RG` (colour_composition §6, fragment
/// addressing).
pub fn uv_screen() -> Result<StainGraph, GraphError> {
    graph(
        source("The pixel's post-flip UV in the view", "ctx.screen.uv"),
        rg(),
    )
}

/// `uv_quad`: the sample's quad-local coordinate `t`, `ctx.quad.uv → RG` (colour_composition §6, structural
/// addressing).
pub fn uv_quad() -> Result<StainGraph, GraphError> {
    graph(
        source("The sample's quad-local coordinate t", "ctx.quad.uv"),
        rg(),
    )
}

/// The coordinate view, the UV passthrough (debug_tooling_plan §F), in `mode`: the sample's UV coordinate
/// reconstructed fragment-side, `c + h·(2t − 1)` (deep_zoom §1; REQ-TOOL-019), as RG; or its δ mode, δ = `h·(2t − 1)`
/// normalised by `h` and mapped to `(δ/h + 1)/2` (REQ-TOOL-153).
pub fn coordinate_view(mode: CoordinateMode) -> Result<StainGraph, GraphError> {
    match mode {
        CoordinateMode::Uv => graph(
            source(
                "The sample's UV coordinate, c + h·(2t − 1) (deep_zoom §1)",
                "ctx.quad.centre + ctx.quad.half_width * (2.0 * ctx.quad.uv - 1.0)",
            ),
            rg(),
        ),
        CoordinateMode::Delta => graph(
            source(
                "The quad-local offset δ = h·(2t − 1) (deep_zoom §2)",
                "ctx.quad.half_width * (2.0 * ctx.quad.uv - 1.0)",
            ),
            "// δ normalised by its bound h = 2^−(ℓ+1) to [−1, 1] and mapped to (x + 1)/2: u's in R, v's in G, B = 0\n\
             // (REQ-TOOL-153).\n\
             fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>((ctx.inputs[0].xy / ctx.quad.half_width + 1.0) * 0.5, 0.0); }\n"
                .to_owned(),
        ),
    }
}
