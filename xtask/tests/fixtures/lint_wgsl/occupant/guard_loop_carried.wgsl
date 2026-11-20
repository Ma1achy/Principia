// R-255: a guard flag stored from one sample does not guard the next sample's `ftle`, read again in a later run of the
// loop; one stored from the sample of the same run, or from `ctx.sample`, which no run reads again, does.
fn colour(ctx: Ctx) -> vec3<f32> {
    var sum = 0.0;
    var ok = false;
    for (var k = 0u; k < 4u; k = k + 1u) {
        let s = sample_read(k, 0.0, false, vec3<f32>(1.0), ctx.params);
        if (ok) { sum = sum + s.ftle; } // fires guard
        ok = has_ftle && s.ftle_valid;
    }
    return ramp_viridis(sum);
}

fn flag_of_this_run(ctx: Ctx) -> f32 {
    var sum = 0.0;
    for (var k = 0u; k < 4u; k = k + 1u) {
        let s = sample_read(k, 0.0, false, vec3<f32>(1.0), ctx.params);
        let ok = has_ftle && s.ftle_valid;
        if (ok) { sum = sum + s.ftle; }
    }
    return sum;
}

fn flag_of_the_context(ctx: Ctx) -> f32 {
    var sum = 0.0;
    var ok = false;
    for (var k = 0u; k < 4u; k = k + 1u) {
        if (ok) { sum = sum + ctx.sample.ftle; }
        ok = has_ftle && ctx.sample.ftle_valid;
    }
    return sum;
}

fn flag_kept_from_a_skipped_run(ctx: Ctx, c: bool) -> f32 {
    var sum = 0.0;
    var ok = false;
    for (var k = 0u; k < 4u; k = k + 1u) {
        let s = sample_read(k, 0.0, false, vec3<f32>(1.0), ctx.params);
        if (c) { ok = has_ftle && s.ftle_valid; }
        if (ok) { sum = sum + s.ftle; } // fires guard
    }
    return sum;
}
