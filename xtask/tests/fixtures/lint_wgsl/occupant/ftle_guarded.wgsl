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

// A guard local whose initial value a store has overwritten, read inside a block, a switch's case and a loop.
fn overwritten_in_block(ctx: Ctx) -> f32 {
    var ok = true;
    ok = has_ftle && ctx.sample.ftle_valid;
    {
        if (ok) {
            return ctx.sample.ftle;
        }
    }
    return 0.0;
}

fn overwritten_in_case(ctx: Ctx, k: u32) -> f32 {
    var ok = true;
    ok = has_ftle && ctx.sample.ftle_valid;
    switch (k) {
        default: {
            if (ok) {
                return ctx.sample.ftle;
            }
        }
    }
    return 0.0;
}

fn overwritten_in_loop(ctx: Ctx) -> f32 {
    var ok = true;
    ok = has_ftle && ctx.sample.ftle_valid;
    loop {
        if (ok) {
            return ctx.sample.ftle;
        }
        break;
    }
    return 0.0;
}

// ... and overwritten in the one branch of an `if` that does not leave, either branch.
fn overwritten_past_a_leave(ctx: Ctx, k: u32) -> f32 {
    var ok = true;
    if (k == 0u) {
        return 0.0;
    } else {
        ok = has_ftle && ctx.sample.ftle_valid;
    }
    if (ok) {
        return ctx.sample.ftle;
    }
    return 0.0;
}

fn overwritten_before_a_leave(ctx: Ctx, k: u32) -> f32 {
    var ok = true;
    if (k != 0u) {
        ok = has_ftle && ctx.sample.ftle_valid;
    } else {
        return 0.0;
    }
    if (ok) {
        return ctx.sample.ftle;
    }
    return 0.0;
}

// Two loads of one variable's sample are one sample.
fn loaded_twice(ctx: Ctx) -> f32 {
    var c = ctx;
    let guard = c.sample;
    let read = c.sample;
    if (has_ftle && guard.ftle_valid) {
        return read.ftle;
    }
    return 0.0;
}

// A guard local with no initialiser, stored on one path only: unstored, it is false.
fn assigned_in_one_branch(ctx: Ctx, k: u32) -> f32 {
    var ok: bool;
    if (k == 0u) {
        ok = has_ftle && ctx.sample.ftle_valid;
    }
    if (ok) {
        return ctx.sample.ftle;
    }
    return 0.0;
}
