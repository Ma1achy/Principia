// The derived finalised-FTLE view (debug_tooling_plan §B, "DERIVED views"; payload §5): `ftle = S_final/(step_count ·
// dt)` with the partial renormalisation interval closed, `S_final = S + ln(δ/δ₀)`, computed in the fragment by the read
// side (`ctx.sample.ftle`), never plain `S/t`. It reads NaN whenever `ftle_valid` is false (R-253, R-254) and draws
// the hatch; a valid value its literal place on the viridis ramp, which needs no range (R-136).

// The value the view shows: the finalised FTLE.
fn value(ctx: Ctx) -> f32 {
    return ctx.sample.ftle;
}

fn colour(ctx: Ctx) -> vec3<f32> {
    return dbg_sentinel(value(ctx), ctx.frag_xy);
}
