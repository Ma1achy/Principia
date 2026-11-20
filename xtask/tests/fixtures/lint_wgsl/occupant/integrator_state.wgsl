// Seam 6 (REQ-COL-005): the read side's integrator parameters are the integrator's.
fn colour(ctx: Ctx) -> vec3<f32> {
    let dt = ctx.params.dt_macro; // fires integrator-state
    let d0 = ctx.params.delta_0; // fires integrator-state
    let nr = ctx.params.n_renorm; // fires integrator-state
    return vec3<f32>(dt, d0, f32(nr));
}
