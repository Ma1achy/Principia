//! The structural presets (colour_composition §6, "§F structural views → `ctx.quad` presets"; debug_tooling_plan §F;
//! render_gui_spec §12.1; TASK-M1-13): in-code stain graphs, as the coordinate presets are (RQ-215).
//!
//! - [`view`]: a structural debug view, `frag/debug/<id>.wgsl` (`s_depth`, `s_state`, `s_impurity`, …), a colour
//!   occupant reading `ctx.quad`, fed the zero source its input does not read, into the pass-through combiner and
//!   `OUT`, as the debug catalogue bakes a view (`render::registry::Catalogue`).
//! - [`boundaries`]: the Tier-1 boundary overlay, the built-in post `edge_line`, after the combiner, at its
//!   [`Level`]: quad boundaries, tile boundaries, or both. With no colour and no brightness the combiner gives the flat
//!   mid-grey (render_gui_spec §13), which the overlay draws over.
//! - [`overlay`]: a post appended to a graph's chain, before `OUT`: the boundary overlay over any view, or the
//!   built-in fallback tint (`fallback_tint`) and pending hatch (`pending_hatch`), the post occupants that read
//!   `ctx.quad` (RQ-239).
//!
//! [`view`]: crate::structural::view
//! [`boundaries`]: crate::structural::boundaries
//! [`overlay`]: crate::structural::overlay
//! [`Level`]: crate::structural::Level

use render::registry;

use crate::stain::{GraphError, NodeId, NodeKind, Occupant, StainGraph};

/// The structural debug views, each `frag/debug/<id>.wgsl` (debug_tooling_plan §F's table, in its order).
pub use render::structural::VIEWS;

/// The source of the zero field: the input a structural view takes and does not read.
fn zero() -> Occupant {
    Occupant::Custom(registry::ZERO_SOURCE.to_owned())
}

/// The structural view `id` (one of [`VIEWS`]), its occupant the registry's `debug/<id>`: the zero source into it,
/// it into the pass-through combiner's colour port.
pub fn view(id: &str) -> Result<StainGraph, GraphError> {
    if !VIEWS.contains(&id) {
        return Err(GraphError(format!("`{id}` is no structural view")));
    }
    let entries = registry::registry().map_err(|e| GraphError(e.to_string()))?;
    let source = entries
        .into_iter()
        .find(|e| e.id == format!("debug/{id}"))
        .map(|e| e.source)
        .ok_or_else(|| GraphError(format!("the registry has no `debug/{id}`")))?;
    let combiner = NodeId(0);
    let mut g = StainGraph::new();
    let s = g.add(NodeKind::Source, zero())?;
    let c = g.add(NodeKind::Colour, Occupant::Custom(source))?;
    g.connect(s, c, 0)?;
    g.connect(c, combiner, 0)?;
    Ok(g)
}

/// The id of the node feeding `OUT`'s colour.
fn tail(g: &StainGraph) -> Result<NodeId, GraphError> {
    let out = NodeId(1);
    g.wires()
        .into_iter()
        .find(|w| w.to == out && w.port == 0)
        .map(|w| w.from)
        .ok_or_else(|| GraphError("nothing feeds OUT".to_owned()))
}

/// Appends a post occupied by `occupant` to `g`'s chain: fed what fed `OUT`, feeding `OUT`. Its id.
pub fn overlay(g: &mut StainGraph, occupant: Occupant) -> Result<NodeId, GraphError> {
    let from = tail(g)?;
    let p = g.add(NodeKind::Post, occupant)?;
    g.connect(from, p, 0)?;
    g.connect(p, NodeId(1), 0)?;
    Ok(p)
}

/// The node feeding the combiner's colour port: a structural view's colour node ([`view`]).
pub fn colour_node(g: &StainGraph) -> Option<NodeId> {
    g.wires()
        .into_iter()
        .find(|w| w.to == NodeId(0) && w.port == 0)
        .map(|w| w.from)
}

/// What the boundary overlay draws (`edge_line`'s `level`): the quad boundaries, from `ctx.quad.uv`; the tile
/// boundaries, from `ctx.tile.uv`; or both (render_gui_spec §12.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Quad = 0,
    Tile = 1,
    Both = 2,
}

/// The boundary overlay at `level` over the flat mid-grey: the built-in post `edge_line` after the combiner, its
/// `level` set and its other params their defaults.
pub fn boundaries(level: Level) -> Result<StainGraph, GraphError> {
    let mut g = StainGraph::new();
    let p = overlay(&mut g, Occupant::Builtin("edge_line".to_owned()))?;
    g.set_param(p, "level", vec![f64::from(level as u32)])?;
    Ok(g)
}
