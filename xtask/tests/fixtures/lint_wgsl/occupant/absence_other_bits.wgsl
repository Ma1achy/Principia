// The absence test compares `ftle`'s bits with the canonical quiet NaN's, the literal or `CANONICAL_QNAN_BITS`
// (lowering Part 3a); a comparison with any other bits branches on the value unguarded.
fn colour(ctx: Ctx) -> vec3<f32> {
    if (bitcast<u32>(ctx.sample.ftle) == CANONICAL_QNAN_BITS) {
        return vec3<f32>(0.0);
    }
    if (0x7fc00000u != bitcast<u32>(ctx.sample.ftle)) {
        return vec3<f32>(0.5);
    }
    if (bitcast<u32>(ctx.sample.ftle) == 0x3f800000u) { // fires guard
        return vec3<f32>(1.0, 0.0, 0.0);
    }
    if (0x3f800000u != bitcast<u32>(ctx.sample.ftle)) { // fires guard
        return vec3<f32>(0.0, 1.0, 0.0);
    }
    return vec3<f32>(1.0);
}
