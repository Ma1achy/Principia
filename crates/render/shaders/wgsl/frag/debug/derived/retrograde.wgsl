// The derived winding-sense view (debug_tooling_plan §B, "DERIVED views"; render contract Part 6, "Winding
// consistency"): `retrograde = θ̃ < 0`, computed in the fragment from the stored running phase `theta` by the read
// side's own `retrograde` (payload §5), as a flag (`dbg_flag`: true green, false red).

// The value the view shows: 1 for retrograde, 0 otherwise.
fn value(ctx: Ctx) -> f32 {
    return select(0.0, 1.0, retrograde(ctx.sample.theta));
}

fn colour(ctx: Ctx) -> vec3<f32> {
    return dbg_flag(retrograde(ctx.sample.theta));
}
