// The coordinate convention (principia_coordinate_conventions_note.md, "The one-line rule"): one internal orientation,
// bottom-left origin, Y-up, and exactly one flip, at the framebuffer → UV boundary. The framebuffer is Y-down, its row
// 0 at the top (frag_coord, canvas events, the read-back image); everything after this flip is Y-up. The Rust twin is
// `render::coords::flip_y` (crates/render/src/coords.rs), the same flip, held equal to this one at every row of a
// target by `flip_twin_matches_wgsl`. A mirrored image is a wrong count of this flip, fixed here, never by a
// compensating flip elsewhere.

// THE CONVENTION FLIP: a framebuffer y (Y-down, measured from the top edge) of a target `height` high, as the Y-up y
// measured from the bottom edge: height − y. It maps the framebuffer's edges to the UV frame's, so v = 1 − y/H is
// flip_y(y, H)/H ([`frag_uv`]), and a pixel row's centre y + 0.5 to its Y-up centre.
fn flip_y(y: f32, height: f32) -> f32 {
    return height - y;
}

// The post-flip UV of the framebuffer point `frag` (frag_coord.xy, a pixel's centre in the fragment) in a target of
// `dims` pixels: u = x/W, v = flip_y(y, H)/H = 1 − frag_coord.y/H, both in [0, 1], bottom-left origin, Y-up.
fn frag_uv(frag: vec2<f32>, dims: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(frag.x, flip_y(frag.y, dims.y)) / dims;
}
