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

// Another tier's flag, `has_ftle` false, and the gate false, by value and through a variable: none guards.
fn other_tier(ctx: Ctx) -> f32 {
    if (has_word && ctx.sample.ftle_valid) {
        return ctx.sample.ftle; // fires guard
    }
    return 0.0;
}

fn tier_false(ctx: Ctx) -> f32 {
    if (!has_ftle && ctx.sample.ftle_valid) {
        return ctx.sample.ftle; // fires guard
    }
    return 0.0;
}

fn gate_false(ctx: Ctx) -> f32 {
    if (has_ftle && !ctx.sample.ftle_valid) {
        return ctx.sample.ftle; // fires guard
    }
    return 0.0;
}

fn gate_false_through_a_variable(ctx: Ctx) -> f32 {
    var c = ctx;
    if (has_ftle && !c.sample.ftle_valid) {
        return c.sample.ftle; // fires guard
    }
    return 0.0;
}

// A local with no initialiser, stored on one path only, read false: unstored, it is false, and guards nothing.
fn unassigned_on_one_path(ctx: Ctx, k: u32) -> f32 {
    var bad: bool;
    if (k == 0u) {
        bad = !(has_ftle && ctx.sample.ftle_valid);
    }
    if (!bad) {
        return ctx.sample.ftle; // fires guard
    }
    return 0.0;
}
