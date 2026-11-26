//! The named presets as stain graphs, read from the library's files (`render/presets/`, [`preset`]): the coordinate
//! presets (colour_composition §6, "Debug views as presets"; debug_tooling_plan §A's UV preset and §F's UV passthrough)
//! and the outcome stain. The graph of a debug view named by id is the catalogue's one wiring rule ([`named_view`]).
//!
//! Each coordinate preset is a stain graph on the fixed backbone: a source holding a coordinate as a vector field, a colour drawing it
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
//! And the stain the real app opens on, the outcome palette's ([`outcome`](crate::presets::outcome)), and a debug view's
//! own graph, named by id ([`named_view`](crate::presets::named_view)) or as text ([`view`](crate::presets::view)).
//!
//! [`uv_screen`]: crate::presets::uv_screen
//! [`uv_quad`]: crate::presets::uv_quad
//! [`coordinate_view`]: crate::presets::coordinate_view
//! [`CoordinateMode::Uv`]: crate::presets::CoordinateMode::Uv
//! [`CoordinateMode::Delta`]: crate::presets::CoordinateMode::Delta

use render::presets::file::Section;

use crate::contract::stain::preset;
use crate::stain::{GraphError, NodeId, NodeKind, Occupant, StainGraph};

/// The coordinate view's two modes (debug_tooling_plan §F's UV passthrough).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordinateMode {
    /// The sample's reconstructed UV coordinate, `c + h·(2t − 1)`, as RG.
    Uv,
    /// The quad-local offset δ = `h·(2t − 1)`, normalised by `h` and mapped to `(δ/h + 1)/2`, as RG.
    Delta,
}

/// The outcome stain, the one the real app opens on: the sample's `state` into the outcome palette's colour
/// (colour_composition §1.4), passed through.
pub fn outcome() -> Result<StainGraph, GraphError> {
    library_graph(Section::Production, "outcome")
}

/// The graph of the library preset `id` of `section`, from its file.
fn library_graph(section: Section, id: &str) -> Result<StainGraph, GraphError> {
    preset::graph(section, id)
}

/// The debug view whose colour is the WGSL `colour`, wired as the catalogue wires it (`render::registry::view_nodes`):
/// the zero source into the colour, passed through.
pub fn view(colour: &str) -> Result<StainGraph, GraphError> {
    chain(
        Occupant::Custom(render::registry::ZERO_SOURCE.to_owned()),
        Occupant::Custom(colour.to_owned()),
    )
}

/// The shipped debug view `name` (`render::registry::Entry::name`), wired as the catalogue wires it
/// (`render::registry::named_view_nodes`): the zero source into the view, both named by id, passed through. The same
/// code as [`view`] of the view's file.
pub fn named_view(name: &str) -> Result<StainGraph, GraphError> {
    chain(
        Occupant::Builtin(render::registry::ZERO.to_owned()),
        Occupant::Builtin(name.to_owned()),
    )
}

/// The graph of a source `source` into a colour `colour`, that into the pass-through combiner's colour port.
fn chain(source: Occupant, colour: Occupant) -> Result<StainGraph, GraphError> {
    let combiner = NodeId(0);
    let mut g = StainGraph::new();
    let s = g.add(NodeKind::Source, source)?;
    let c = g.add(NodeKind::Colour, colour)?;
    g.connect(s, c, 0)?;
    g.connect(c, combiner, 0)?;
    Ok(g)
}

/// `uv_screen`: the pixel's post-flip UV in the view, `ctx.screen.uv → RG` (colour_composition §6, fragment
/// addressing). Its file is `presets/debug/uv_screen.json`.
pub fn uv_screen() -> Result<StainGraph, GraphError> {
    library_graph(Section::Debug, "uv_screen")
}

/// `uv_quad`: the sample's quad-local coordinate `t`, `ctx.quad.uv → RG` (colour_composition §6, structural
/// addressing).
pub fn uv_quad() -> Result<StainGraph, GraphError> {
    library_graph(Section::Debug, "uv_quad")
}

/// The coordinate view, the UV passthrough (debug_tooling_plan §F), in `mode`: the sample's UV coordinate
/// reconstructed fragment-side, `c + h·(2t − 1)` (deep_zoom §1), as RG; or its δ mode, δ = `h·(2t − 1)` normalised by
/// `h` and mapped to `(δ/h + 1)/2`.
pub fn coordinate_view(mode: CoordinateMode) -> Result<StainGraph, GraphError> {
    library_graph(
        Section::Debug,
        match mode {
            CoordinateMode::Uv => "coordinate_uv",
            CoordinateMode::Delta => "coordinate_delta",
        },
    )
}
