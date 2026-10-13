// A debug view: it shows `ftle` literally, R-114's bitcast guard its NaN guard, so the guard rule does not bind it
// (R-79); the shadow and deny-list rules still do.
fn colour(ctx: Ctx) -> vec3<f32> {
    let raw = ctx.sample.ftle;
    if (bitcast<u32>(raw) == 0x7fc00000u) {
        return debug_invalid(ctx.frag_xy);
    }
    let d = f32(ctx.quad.depth); // fires scheduler-state
    let sh = ctx.sample.r_sh[0].x; // fires shadow-read
    return ramp_viridis(raw + d + sh);
}
