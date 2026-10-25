// The shape-sphere chart's golden occupant (TASK-M2-28; chart_reference §3.3; R-544; `validation::shape_scene`): each
// sample's shape `n` (`ctx.sample.n`, the read side's, from the decode's own state, the chart's signed point) drawn as
// its octant `k`, the signs of `n.x`, `n.y` and `n.z`, `k = [n.x < 0] + 2·[n.y < 0] + 4·[n.z < 0]`, 0 to 3 above the
// equator and 4 to 7 below it, in the prelude's hue wheel at `(8k + 7)/64` turns: eight hues 45° apart at the wheel's
// one OKLCH lightness and chroma, none black, none near the canvas background, the offset 7/64 the one of 1/2048's
// steps that keeps them farthest from an 8-bit rounding tie, a step of 0.09; so both hemispheres are drawn and told
// apart, neither marked redundant. `u_flag` 1 is the flagged control's alone (R-141's withdrawn flag, which no chart
// scene sets): the lower half striped white, a colour no octant uses, diagonal stripes 2 px wide, `⌊(x + y)/2⌋` even.
// `u_ellipse` 1 is the equal-area scene's: a display tile whose centre lies outside Hammer's ellipse,
// `(2x − 1)² + (2y − 1)² > 1` on the `u_cols` × `u_rows` display grid, shows no chart point, display margin and not a
// pixel of the chart, and draws the canvas background (chart_reference §3.3: "drawn blank (R-470)").
// @uniform u_flag: u32 = 0 [0, 1]
// @uniform u_ellipse: u32 = 0 [0, 1]
// @uniform u_cols: u32 = 64 [1, 4096]
// @uniform u_rows: u32 = 32 [1, 4096]

fn colour(ctx: Ctx) -> vec3<f32> {
    if (uniforms.u_ellipse == 1u) {
        let tile = vec2<u32>(ctx.quad.index % uniforms.u_cols, ctx.quad.index / uniforms.u_cols);
        let c = (vec2<f32>(tile) + vec2<f32>(0.5)) / vec2<f32>(f32(uniforms.u_cols), f32(uniforms.u_rows));
        let h = c + c - vec2<f32>(1.0);
        if (dot(h, h) > 1.0) {
            return canvas_background();
        }
    }
    let n = ctx.sample.n;
    let k = select(0u, 1u, n.x < 0.0) + select(0u, 2u, n.y < 0.0) + select(0u, 4u, n.z < 0.0);
    let p = vec2<u32>(ctx.frag_xy);
    let stripe = ((p.x + p.y) / 2u) % 2u == 0u;
    if (uniforms.u_flag == 1u && n.z < 0.0 && stripe) {
        return vec3<f32>(1.0);
    }
    return hue_wheel(f32(8u * k + 7u) / 64.0);
}
