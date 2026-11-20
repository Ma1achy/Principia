// R-255's clean case: aggregates over `ftle` that exclude samples by `ftle_valid`, by skipping them and by guarding the
// accumulation.
fn colour(ctx: Ctx) -> vec3<f32> {
    var sum = 0.0;
    var n = 0.0;
    var lo = 1.0e30;
    for (var k = 0u; k < 4u; k = k + 1u) {
        let s = sample_read(k, 0.0, false, vec3<f32>(1.0), ctx.params);
        if (!(has_ftle && s.ftle_valid)) {
            continue;
        }
        sum = sum + s.ftle;
        lo = min(lo, s.ftle);
        n = n + 1.0;
    }
    for (var k = 0u; k < 4u; k = k + 1u) {
        let s = sample_read(k, 0.0, false, vec3<f32>(1.0), ctx.params);
        if (has_ftle && s.ftle_valid) {
            sum = sum + s.ftle;
        }
    }
    return ramp_viridis(select(0.0, sum / max(n, 1.0), n > 0.0) + lo);
}
