// The FTLE-running accumulator view (render contract Part 5, "Live-state & array inspection"; debug_tooling_plan §E):
// the Benettin sum over the elapsed time, `S/t` with `t = step_count · dt_macro`, a live approximation; the finalised
// read, `S_final/(step_count · dt)`, is the derived `ftle` view's (payload §5). Undefined before the first step, where
// it reads the canonical quiet NaN and draws the hatch; otherwise its literal place on the viridis ramp (R-136).

// The value the view shows: `S/(step_count · dt_macro)`, NaN at `step_count = 0`.
fn value(ctx: Ctx) -> f32 {
    let n = ctx.sample.t_end_step;
    return select(canonical_nan(), ctx.sample.S / (f32(n) * ctx.params.dt_macro), n > 0u);
}

fn colour(ctx: Ctx) -> vec3<f32> {
    return dbg_sentinel(value(ctx), ctx.frag_xy);
}
