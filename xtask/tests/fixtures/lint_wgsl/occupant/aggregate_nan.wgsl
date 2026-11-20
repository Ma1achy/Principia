// R-255: a footprint mean over `ftle` that relies on NaN propagation, and a minimum over NaN operands, each fail.
fn colour(ctx: Ctx) -> vec3<f32> {
    var sum = 0.0;
    var lo = 1.0e30;
    for (var k = 0u; k < 4u; k = k + 1u) {
        let s = sample_read(k, 0.0, false, vec3<f32>(1.0), ctx.params);
        sum = sum + s.ftle; // fires guard
        lo = min(lo, s.ftle); // fires guard
    }
    if (has_ftle) {
        return ramp_viridis(sum / 4.0 + lo);
    }
    return vec3<f32>(0.0);
}
