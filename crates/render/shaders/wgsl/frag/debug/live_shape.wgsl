// The live shape views (render contract Part 5, "Live-state & array inspection"; debug_tooling_plan §E; integrator dd
// §3.7), over the current derived `n` (the read side's, `shape(r, masses)`, payload §5) and the running unwrapped
// phase `θ̃` (`theta`), by `u_mode`:
// - 0: `dbg_dircos3(n)`, `½(n̂ + 1)` as linear RGB, the direction cosines of the current `n`;
// - 1: Twilight, the cyclic map, at `θ̃/2π` mod 1 (R-122: the published matplotlib table);
// - 2: `‖n‖ − 1`, the normalisation error, at its literal place on the viridis ramp (`dbg_sentinel`): flat zero is
//   expected, since `n` is derived fresh each step. A signed field on viridis, as the corpus stands (RQ-260 open).
// Every colouring maps NaN or a sentinel to its invalid colour (payload §1; render contract Part 4): where `n` is
// undefined (a coincident configuration, `I = 0`, gives `0/0`) or `θ̃` is not finite, each mode draws the hatch.
// @uniform u_mode: u32 = 0 [0, 2]

// Whether `x` is not finite, NaN or ±inf, by its exponent bits (lowering Part 3a; R-351: no `isnan`).
fn live_shape_nonfinite(x: f32) -> bool {
    return (bitcast<u32>(x) & 0x7f800000u) == 0x7f800000u;
}

fn colour(ctx: Ctx) -> vec3<f32> {
    let n = ctx.sample.n;
    if (uniforms.u_mode == 0u) {
        return dbg_dircos3(n, ctx.frag_xy);
    }
    if (uniforms.u_mode == 1u) {
        let theta = ctx.sample.theta;
        if (live_shape_nonfinite(theta)) {
            return debug_invalid(ctx.frag_xy);
        }
        return ramp_twilight(fract(theta / 6.2831855));
    }
    if (live_shape_nonfinite(n.x) || live_shape_nonfinite(n.y) || live_shape_nonfinite(n.z)) {
        return debug_invalid(ctx.frag_xy);
    }
    return dbg_sentinel(length(n) - 1.0, ctx.frag_xy);
}
