// The Tier-1 boundary overlay (render_gui_spec §12.1; REQ-RENDER-024; TASK-M1-13): an ordinary post node, drawn from
// the address alone, `ctx.quad.uv` for quad boundaries and `ctx.tile.uv` for tile boundaries, with no buffer and no
// pass of its own. `edge_line` gives the line's coverage at a pixel from its distance, in pixels, to the nearest edge
// of its cell, so a line is the same width in pixels at any quad size, depth and zoom, and antialiased.
//
// `width` and `opacity` style the quad boundaries, `tile_width` and `tile_opacity` the tile boundaries, `colour`
// (linear RGB) both; `level` picks what is drawn: 0 the quad boundaries, 1 the tile boundaries, 2 both, composited by
// the larger coverage, so an edge both cells share is drawn once. A width is the line's full width across the edge, in
// pixels, at half coverage. The leaf outlines are the quad boundaries of the quads drawn (debug_tooling_plan §F).
// @uniform width: f32 = 2.0 [0.25, 16.0]
// @uniform opacity: f32 = 1.0 [0.0, 1.0]
// @uniform tile_width: f32 = 1.0 [0.25, 16.0]
// @uniform tile_opacity: f32 = 0.5 [0.0, 1.0]
// @uniform colour: vec3<f32> = (1.0, 1.0, 1.0)
// @uniform level: u32 = 0 [0, 2]

// One pixel's size in the cell coordinate `uv`, per axis: `fwidth(uv)`, each derivative taken modulo the cell, so that
// a 2 × 2 pixel block straddling a cell edge, where `uv` jumps from near 1 to near 0, still reads the step within a
// cell, `1 / (cell size in px)`, not the jump (TASK-M1-13; applied per R-369). §12.1's `fwidth(d)` of the edge distance
// reads 0 there, the distance being equal on both sides of the edge, and at a cell's corner, where the nearer axis
// changes; this reads the pixel's size wherever the cell is at least 2 px wide.
fn cell_fwidth(uv: vec2<f32>) -> vec2<f32> {
    let dx = dpdx(uv);
    let dy = dpdy(uv);
    return abs(dx - round(dx)) + abs(dy - round(dy));
}

// render_gui_spec §12.1's `edge_line`: the distance to the nearest edge of the cell, cell-local, converted to pixels by
// the pixel's size in `uv`, then `1 − smoothstep(0, width_px, d)`: constant pixel width, antialiased.
fn edge_line(uv: vec2<f32>, width_px: f32) -> f32 {
    let e = min(uv, vec2<f32>(1.0) - uv);
    let px = e / cell_fwidth(uv);
    let d = min(px.x, px.y);
    return 1.0 - smoothstep(0.0, width_px, d);
}

fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {
    // Both lines are evaluated whatever `level` is: the derivatives need uniform control flow.
    let quad = uniforms.opacity * edge_line(ctx.quad.uv, uniforms.width);
    let tile = uniforms.tile_opacity * edge_line(ctx.tile.uv, uniforms.tile_width);
    // A lane the raster did not place (`shade_sample`) holds the absence NaN: no line there.
    let q = select(quad, 0.0, uniforms.level == 1u || is_absent_nan(ctx.quad.uv.x));
    let t = select(tile, 0.0, uniforms.level == 0u || is_absent_nan(ctx.tile.uv.x));
    return mix(rgb, uniforms.colour, max(q, t));
}
