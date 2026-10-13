// REQ-RENDER-015: the guard holds for the sample it tested. A sample variable stored again after its guard, whole, by
// a member, through a pointer or by a function given one, is another sample; one stored only before it is not.
fn colour(ctx: Ctx) -> vec3<f32> {
    var s = ctx.sample;
    if (!(has_ftle && s.ftle_valid)) { return vec3<f32>(0.0); }
    s = sample_read(1u, 0.0, false, vec3<f32>(1.0), ctx.params);
    return ramp_viridis(s.ftle); // fires guard
}

fn by_member(ctx: Ctx) -> f32 {
    var s = ctx.sample;
    if (has_ftle && s.ftle_valid) {
        s.ftle = f32(ctx.params.horizon_steps);
        return s.ftle; // fires guard
    }
    return 0.0;
}

fn through_a_pointer(ctx: Ctx) -> f32 {
    var s = ctx.sample;
    let p = &s;
    if (!(has_ftle && (*p).ftle_valid)) { return 0.0; }
    *p = sample_read(2u, 0.0, false, vec3<f32>(1.0), ctx.params);
    return (*p).ftle; // fires guard
}

fn refill(p: ptr<function, SimState>, ctx: Ctx) {
    *p = sample_read(3u, 0.0, false, vec3<f32>(1.0), ctx.params);
}

fn by_a_function(ctx: Ctx) -> f32 {
    var s = ctx.sample;
    if (!(has_ftle && s.ftle_valid)) { return 0.0; }
    refill(&s, ctx);
    return s.ftle; // fires guard
}

fn in_a_branch(ctx: Ctx, c: bool) -> f32 {
    var s = ctx.sample;
    if (!(has_ftle && s.ftle_valid)) { return 0.0; }
    if (c) {
        s = sample_read(4u, 0.0, false, vec3<f32>(1.0), ctx.params);
    }
    return s.ftle; // fires guard
}

fn in_a_later_iteration(ctx: Ctx) -> f32 {
    var s = ctx.sample;
    var acc = 0.0;
    if (!(has_ftle && s.ftle_valid)) { return 0.0; }
    for (var k = 0u; k < 4u; k = k + 1u) {
        acc = acc + s.ftle; // fires guard
        s = sample_read(k, 0.0, false, vec3<f32>(1.0), ctx.params);
    }
    return acc;
}

fn through_a_flag(ctx: Ctx) -> f32 {
    var s = ctx.sample;
    let ok = has_ftle && s.ftle_valid;
    s = sample_read(5u, 0.0, false, vec3<f32>(1.0), ctx.params);
    if (ok) {
        return s.ftle; // fires guard
    }
    return 0.0;
}

fn stored_before(ctx: Ctx) -> f32 {
    var s = sample_read(6u, 0.0, false, vec3<f32>(1.0), ctx.params);
    s = sample_read(7u, 0.0, false, vec3<f32>(1.0), ctx.params);
    if (!(has_ftle && s.ftle_valid)) { return 0.0; }
    return s.ftle;
}
