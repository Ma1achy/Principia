// The drift max-vs-final accumulator view (render contract Part 5, "Live-state & array inspection"; Part 6, "Drift
// shape"): the current drift's magnitude over the running maximum, `|H(r, p) − E_0| / dE_max` (`u_quantity` 0) or
// `|L_z(r, p) − Lz_0| / dLz_max` (1), on the viridis ramp over [0, 1]. Near 1 the drift is secular, still at its
// maximum; near 0 it was a transient spike that recovered: the reason both are kept (payload §5; R-246). Before any
// drift is latched, a maximum of 0, the ratio is undefined and the hatch is drawn.
// @uniform u_quantity: u32 = 0 [0, 1]

// The value the view shows: the ratio, NaN where the maximum is not positive.
fn value(ctx: Ctx) -> f32 {
    let energy = uniforms.u_quantity == 0u;
    let last = abs(select(ctx.sample.Lz_drift, ctx.sample.energy_drift, energy));
    let top = select(ctx.sample.dLz_max, ctx.sample.dE_max, energy);
    return select(canonical_nan(), last / top, top > 0.0);
}

fn colour(ctx: Ctx) -> vec3<f32> {
    let v = value(ctx);
    if (is_absent_nan(v)) {
        return debug_invalid(ctx.frag_xy);
    }
    return dbg_lin(v, 0.0, 1.0);
}
