// R-255 in a debug view: it shows its own sample's value literally, `ctx.sample.ftle` (R-79), but an aggregate over
// `ftle` of other samples still excludes them by `ftle_valid`, never by NaN propagation; so does any read of a sample
// that is not the context the view is given.
fn own_value(ctx: Ctx) -> f32 {
    return ctx.sample.ftle;
}

fn given_a_copy(c: Ctx) -> f32 {
    return c.sample.ftle; // fires guard
}

fn colour(ctx: Ctx) -> vec3<f32> {
    var sum = 0.0;
    for (var k = 0u; k < 4u; k = k + 1u) {
        let s = sample_read(k, 0.0, false, vec3<f32>(1.0), ctx.params);
        sum = sum + s.ftle; // fires guard
    }
    var other = ctx;
    other.sample = sample_read(1u, 0.0, false, vec3<f32>(1.0), ctx.params);
    let shown = own_value(ctx) + ctx.sample.ftle + ctx.sample.ensemble_spread;
    return vec3<f32>(sum, shown, given_a_copy(other));
}
