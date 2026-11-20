// REQ-RENDER-015: the guard holds for the sample read: `ctx.sample`'s gate does not guard another sample's `ftle`.
fn colour(ctx: Ctx) -> vec3<f32> {
    let s = sample_read(1u, 0.0, false, vec3<f32>(1.0), ctx.params);
    if (has_ftle && ctx.sample.ftle_valid) {
        return ramp_viridis(s.ftle); // fires guard
    }
    return vec3<f32>(0.0);
}
