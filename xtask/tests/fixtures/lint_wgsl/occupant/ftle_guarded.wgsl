// REQ-RENDER-015's clean cases: `has_ftle && ftle_valid` taken, its negation leaving, De Morgan's form, further
// conjuncts, the reject branch leaving, through a pointer, a guard held in a local, the absence test either way round,
// and code that never runs.
fn taken(ctx: Ctx) -> vec3<f32> {
    if (has_ftle && ctx.sample.ftle_valid) {
        return ramp_viridis(ctx.sample.ftle);
    }
    return vec3<f32>(0.0);
}

fn leaves(ctx: Ctx) -> vec3<f32> {
    if (!(has_ftle && ctx.sample.ftle_valid)) {
        return debug_invalid(ctx.frag_xy);
    }
    return ramp_viridis(ctx.sample.ftle);
}

fn de_morgan(ctx: Ctx) -> vec3<f32> {
    if (!has_ftle || !ctx.sample.ftle_valid) {
        return debug_invalid(ctx.frag_xy);
    }
    return ramp_viridis(ctx.sample.ftle);
}

fn further(ctx: Ctx) -> vec3<f32> {
    if (ctx.sample.ftle_valid && has_ftle && ctx.sample.ftle > 0.5) {
        return vec3<f32>(1.0);
    }
    return vec3<f32>(0.0);
}

fn otherwise(ctx: Ctx) -> vec3<f32> {
    if (has_ftle && ctx.sample.ftle_valid) {
    } else {
        return debug_invalid(ctx.frag_xy);
    }
    return ramp_viridis(ctx.sample.ftle);
}

fn pointer(ctx: Ctx) -> vec3<f32> {
    var s = ctx.sample;
    if (has_ftle && s.ftle_valid) {
        return ramp_viridis(s.ftle);
    }
    return vec3<f32>(0.0);
}

fn held(ctx: Ctx) -> vec3<f32> {
    var ok = false;
    if (has_ftle) {
        ok = ctx.sample.ftle_valid;
    }
    if (ok) {
        return ramp_viridis(ctx.sample.ftle);
    }
    return vec3<f32>(0.0);
}

fn absence(ctx: Ctx) -> vec3<f32> {
    if (bitcast<u32>(ctx.sample.ftle) == 0x7fc00000u) {
        return debug_invalid(ctx.frag_xy);
    }
    if (0x7fc00000u != bitcast<u32>(ctx.sample.ftle)) {
        return vec3<f32>(1.0);
    }
    return vec3<f32>(0.0);
}

fn never(ctx: Ctx) -> vec3<f32> {
    var off: bool = false;
    if (off) {
        return ramp_viridis(ctx.sample.ftle);
    }
    return vec3<f32>(0.0);
}

fn colour(ctx: Ctx) -> vec3<f32> {
    return taken(ctx) + leaves(ctx) + de_morgan(ctx) + further(ctx) + otherwise(ctx) + pointer(ctx) + held(ctx)
        + absence(ctx) + never(ctx);
}
