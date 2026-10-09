// The diffusion-slope accumulator view (render contract Part 5, "Live-state & array inspection"; debug_tooling_plan
// §E; render contract Part 6, "Diffusion (Welford)"): `C_ty/C_tt(n)` from the Welford co-moment `C_ty` and the
// closed-form `C_tt(n) = h²·n(n²−1)/12`, computed in the fragment by the read side's own `diffusion_slope` (payload §4,
// §5), `n` the sample's own `t_end_step`. An invalid fit, `n < 2`, reads NaN (R-245) and draws the hatch; a valid one
// its literal place on the viridis ramp (R-136).

// The value the view shows: the slope, NaN for `n < 2`.
fn value(ctx: Ctx) -> f32 {
    return diffusion_slope(ctx.sample.C_ty, ctx.sample.t_end_step, ctx.params.dt_macro);
}

fn colour(ctx: Ctx) -> vec3<f32> {
    return dbg_sentinel(value(ctx), ctx.frag_xy);
}
