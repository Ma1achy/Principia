// The derived winding-count view (debug_tooling_plan §B, "DERIVED views"; render contract Part 6, "Winding
// consistency"): `orbit_count = ⌊|θ̃|/2π⌋`, computed in the fragment from the stored running phase `theta` by the read
// side's own `orbit_count` (payload §5), at its literal place on the viridis ramp (R-136).

// The value the view shows: the completed revolutions.
fn value(ctx: Ctx) -> f32 {
    return f32(orbit_count(ctx.sample.theta));
}

fn colour(ctx: Ctx) -> vec3<f32> {
    return dbg_sentinel(value(ctx), ctx.frag_xy);
}
