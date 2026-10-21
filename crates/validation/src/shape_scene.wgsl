// The shape-sphere chart's golden occupant (TASK-M2-28; chart_reference §3.3; `validation::shape_scene`): each
// sample's shape `n` (`ctx.sample.n`, the read side's, from the chart's own configuration) drawn as the quadrant of
// its azimuth, the signs of `n.x` and `n.y`, Okabe–Ito classes 1 to 4 (`dbg_cat`). Where the chart is n-to-1 over its
// φ hemispheres (`u_fold` 1, R-141), the redundant hemisphere, `n.z < 0`, whose systems the `w ≥ 0` hemisphere holds,
// carries the redundancy flag: black diagonal stripes 2 px wide, `⌊(x + y)/2⌋` even. A sample showing no chart point,
// outside Hammer's ellipse, is no system, and draws the debug views' hatch (R-470).
// @uniform u_fold: u32 = 0 [0, 1]

fn colour(ctx: Ctx) -> vec3<f32> {
    if (ctx.validity.forbidden) {
        return debug_invalid(ctx.frag_xy);
    }
    let n = ctx.sample.n;
    let q = select(0u, 1u, n.x < 0.0) + select(0u, 2u, n.y < 0.0);
    let p = vec2<u32>(ctx.frag_xy);
    let stripe = ((p.x + p.y) / 2u) % 2u == 0u;
    if (uniforms.u_fold == 1u && n.z < 0.0 && stripe) {
        return vec3<f32>(0.0);
    }
    return dbg_cat(q + 1u, 8u);
}
