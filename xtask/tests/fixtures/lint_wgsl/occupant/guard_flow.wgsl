// REQ-RENDER-015 over control flow: a store into the guarded sample reaches a use after a switch from each case that
// does not leave (one ending in `break`, one sharing its body with another selector); from anywhere in a loop, a
// nested block, branch, case, loop or continuing step included, it reaches the loop's next run; a call reaches every
// global, each use of which is the one global. An array's element is the same sample at the same index, and another once the index changes; two loads of
// one variable are one sample, and a load taken before a store is another than one after it.
var<private> kept: SimState;

fn refill_kept(ctx: Ctx) {
    kept = sample_read(9u, 0.0, false, vec3<f32>(1.0), ctx.params);
}

fn after_a_leaving_case(ctx: Ctx, k: u32) -> f32 {
    var s = ctx.sample;
    if (!(has_ftle && s.ftle_valid)) { return 0.0; }
    switch (k) {
        case 0u: {
            s = sample_read(1u, 0.0, false, vec3<f32>(1.0), ctx.params);
            return 0.0;
        }
        default: {}
    }
    return s.ftle;
}

fn after_a_case_that_stores(ctx: Ctx, k: u32) -> f32 {
    var s = ctx.sample;
    if (!(has_ftle && s.ftle_valid)) { return 0.0; }
    switch (k) {
        case 0u: {
            s = sample_read(1u, 0.0, false, vec3<f32>(1.0), ctx.params);
        }
        default: {}
    }
    return s.ftle; // fires guard
}

fn after_a_case_that_breaks(ctx: Ctx, k: u32) -> f32 {
    var s = ctx.sample;
    if (!(has_ftle && s.ftle_valid)) { return 0.0; }
    switch (k) {
        case 0u: {
            s = sample_read(1u, 0.0, false, vec3<f32>(1.0), ctx.params);
            break;
        }
        default: {}
    }
    return s.ftle; // fires guard
}

fn after_shared_selectors(ctx: Ctx, k: u32) -> f32 {
    var s = ctx.sample;
    if (!(has_ftle && s.ftle_valid)) { return 0.0; }
    switch (k) {
        case 0u, 1u: {
            s = sample_read(1u, 0.0, false, vec3<f32>(1.0), ctx.params);
        }
        default: {}
    }
    return s.ftle; // fires guard
}

fn loop_branch(ctx: Ctx, c: bool) -> f32 {
    var s = ctx.sample;
    var acc = 0.0;
    if (!(has_ftle && s.ftle_valid)) { return 0.0; }
    for (var k = 0u; k < 4u; k = k + 1u) {
        acc = acc + s.ftle; // fires guard
        if (c) { s = sample_read(k, 0.0, false, vec3<f32>(1.0), ctx.params); }
    }
    return acc;
}

fn loop_case(ctx: Ctx) -> f32 {
    var s = ctx.sample;
    var acc = 0.0;
    if (!(has_ftle && s.ftle_valid)) { return 0.0; }
    for (var k = 0u; k < 4u; k = k + 1u) {
        acc = acc + s.ftle; // fires guard
        switch (k) {
            case 2u: { s = sample_read(k, 0.0, false, vec3<f32>(1.0), ctx.params); }
            default: {}
        }
    }
    return acc;
}

fn loop_block(ctx: Ctx) -> f32 {
    var s = ctx.sample;
    var acc = 0.0;
    if (!(has_ftle && s.ftle_valid)) { return 0.0; }
    for (var k = 0u; k < 4u; k = k + 1u) {
        acc = acc + s.ftle; // fires guard
        {
            s = sample_read(k, 0.0, false, vec3<f32>(1.0), ctx.params);
        }
    }
    return acc;
}

fn loop_inner_loop(ctx: Ctx) -> f32 {
    var s = ctx.sample;
    var acc = 0.0;
    if (!(has_ftle && s.ftle_valid)) { return 0.0; }
    for (var k = 0u; k < 4u; k = k + 1u) {
        acc = acc + s.ftle; // fires guard
        for (var j = 0u; j < 2u; j = j + 1u) {
            s = sample_read(j, 0.0, false, vec3<f32>(1.0), ctx.params);
        }
    }
    return acc;
}

fn loop_continuing(ctx: Ctx) -> f32 {
    var s = ctx.sample;
    var acc = 0.0;
    if (!(has_ftle && s.ftle_valid)) { return 0.0; }
    for (var k = 0u; k < 4u; s = sample_read(k, 0.0, false, vec3<f32>(1.0), ctx.params)) {
        acc = acc + s.ftle; // fires guard
        k = k + 1u;
    }
    return acc;
}

fn loop_chosen(ctx: Ctx) -> f32 {
    var acc = 0.0;
    var ok = false;
    let a = sample_read(0u, 0.0, false, vec3<f32>(1.0), ctx.params);
    for (var k = 0u; k < 4u; k = k + 1u) {
        let pair = array<SimState, 2>(a, sample_read(k, 0.0, false, vec3<f32>(1.0), ctx.params));
        let s = pair[k % 2u];
        if (ok) { acc = acc + s.ftle; } // fires guard
        ok = has_ftle && s.ftle_valid;
    }
    return acc;
}

fn global_after_a_call(ctx: Ctx) -> f32 {
    if (!(has_ftle && kept.ftle_valid)) { return 0.0; }
    refill_kept(ctx);
    return kept.ftle; // fires guard
}

fn global_stored(ctx: Ctx) -> f32 {
    if (!(has_ftle && kept.ftle_valid)) { return 0.0; }
    kept = sample_read(8u, 0.0, false, vec3<f32>(1.0), ctx.params);
    return kept.ftle; // fires guard
}

fn global_without_a_call(ctx: Ctx) -> f32 {
    if (!(has_ftle && kept.ftle_valid)) { return 0.0; }
    return kept.ftle;
}

fn element(ctx: Ctx, k: u32) -> f32 {
    var arr = array<SimState, 2>(ctx.sample, ctx.sample);
    var i = k;
    if (!(has_ftle && arr[i].ftle_valid)) { return 0.0; }
    let same = arr[i].ftle;
    i = i + 1u;
    return same + arr[i].ftle; // fires guard
}

fn two_loads(ctx: Ctx) -> f32 {
    var s = ctx.sample;
    let a = s;
    let b = s;
    if (has_ftle && a.ftle_valid) {
        return b.ftle;
    }
    return 0.0;
}

fn load_before_a_store(ctx: Ctx) -> f32 {
    var s = ctx.sample;
    let a = s;
    s = sample_read(3u, 0.0, false, vec3<f32>(1.0), ctx.params);
    let b = s;
    if (has_ftle && a.ftle_valid) {
        return b.ftle; // fires guard
    }
    return 0.0;
}
