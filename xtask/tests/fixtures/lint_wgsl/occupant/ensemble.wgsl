// R-145: `ensemble_spread` guarded by the uniform, `has_ensemble()`, and once not.
fn colour(ctx: Ctx) -> vec3<f32> {
    if (has_ensemble()) {
        return ramp_viridis(ctx.sample.ensemble_spread);
    }
    return ramp_viridis(ctx.sample.ensemble_spread); // fires guard
}
