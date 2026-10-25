// The shape-sphere chart's golden occupant (TASK-M2-28; chart_reference §3.3; R-544; `validation::shape_scene`): each
// sample's shape `n` (`ctx.sample.n`, the read side's, from the decode's own state, the chart's signed point) drawn as
// its octant, Okabe–Ito classes 1 to 8 (`dbg_cat`): the quadrant of its azimuth, the signs of `n.x` and `n.y`, classes
// 1 to 4 above the equator and 5 to 8 below it, `n.z < 0`, so both hemispheres are drawn and told apart, neither
// marked redundant. `u_flag` 1 is the flagged control's alone (R-141's withdrawn flag, which no chart scene sets): the
// lower half striped black, diagonal stripes 2 px wide, `⌊(x + y)/2⌋` even. A sample showing no chart point, outside
// Hammer's ellipse, is no system, and draws the debug views' hatch (R-470).
// @uniform u_flag: u32 = 0 [0, 1]

fn colour(ctx: Ctx) -> vec3<f32> {
    if (ctx.validity.forbidden) {
        return debug_invalid(ctx.frag_xy);
    }
    let n = ctx.sample.n;
    let k = select(0u, 1u, n.x < 0.0) + select(0u, 2u, n.y < 0.0) + select(0u, 4u, n.z < 0.0);
    let p = vec2<u32>(ctx.frag_xy);
    let stripe = ((p.x + p.y) / 2u) % 2u == 0u;
    if (uniforms.u_flag == 1u && n.z < 0.0 && stripe) {
        return vec3<f32>(0.0);
    }
    return dbg_cat(k + 1u, 8u);
}
