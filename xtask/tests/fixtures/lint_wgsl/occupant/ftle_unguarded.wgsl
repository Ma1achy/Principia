// REQ-RENDER-015: `ftle` coloured with no guard, with the gate alone, and with `has_ftle` alone.
fn gate_only(ctx: Ctx) -> vec3<f32> {
    if (ctx.sample.ftle_valid) {
        return ramp_viridis(ctx.sample.ftle); // fires guard
    }
    return vec3<f32>(0.0);
}

fn has_only(ctx: Ctx) -> vec3<f32> {
    if (has_ftle) {
        return ramp_viridis(ctx.sample.ftle); // fires guard
    }
    return vec3<f32>(0.0);
}

fn on(ctx: Ctx) -> vec3<f32> {
    var on: bool = true;
    if (on) {
        return ramp_viridis(ctx.sample.ftle); // fires guard
    }
    return vec3<f32>(0.0);
}

fn colour(ctx: Ctx) -> vec3<f32> {
    return ramp_viridis(ctx.sample.ftle) + gate_only(ctx) + has_only(ctx) + on(ctx); // fires guard
}
