// The derived current-drift view (debug_tooling_plan §B, "DERIVED views"; payload §5): `ΔE = H(r, p) − E_0`, computed
// in the fragment from the live f32 state through the read side's own `hamiltonian` (TASK-M1-01), with the sample's
// `ICDescriptor` masses (`ctx.ic`; dd_generation_root §3.8). Its literal place on the viridis ramp (`dbg_sentinel`),
// R-381's `symlog` placement; a signed field on viridis, as the corpus stands (RQ-260 open).

// The value the view shows: the current energy drift.
fn value(ctx: Ctx) -> f32 {
    let masses = vec3<f32>(ctx.ic.m0, ctx.ic.m1, ctx.ic.m2);
    return hamiltonian(ctx.sample.r, ctx.sample.p, masses) - ctx.sample.E_0;
}

fn colour(ctx: Ctx) -> vec3<f32> {
    return dbg_sentinel(value(ctx), ctx.frag_xy);
}
