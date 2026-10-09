// The live shape views (render contract Part 5, "Live-state & array inspection"; debug_tooling_plan §E; integrator dd
// §3.7), over the current derived `n` (the read side's, `shape(r, masses)`, payload §5) and the running unwrapped
// phase `θ̃` (`theta`), by `u_mode`:
// - 0: `½(n + 1)` as linear RGB, the direction cosines of the current `n`;
// - 1: Twilight, the cyclic map, at `θ̃/2π` mod 1 (R-122: the published matplotlib table);
// - 2: `‖n‖ − 1`, the normalisation error, at its literal place on the viridis ramp (`dbg_sentinel`): flat zero is
//   expected, since `n` is derived fresh each step. A signed field on viridis, as the corpus stands (RQ-260 open).
// @uniform u_mode: u32 = 0 [0, 2]
fn colour(ctx: Ctx) -> vec3<f32> {
    let n = ctx.sample.n;
    if (uniforms.u_mode == 0u) {
        return 0.5 * (n + vec3<f32>(1.0));
    }
    if (uniforms.u_mode == 1u) {
        return ramp_twilight(fract(ctx.sample.theta / 6.2831855));
    }
    return dbg_sentinel(length(n) - 1.0, ctx.frag_xy);
}
