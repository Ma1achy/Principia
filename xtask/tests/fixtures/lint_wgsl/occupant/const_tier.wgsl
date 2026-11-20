// A tier constant used in a `const` declaration is valid WGSL (lowering Part 3a: `has_<feature>` is a compile-time
// const), so the occupant lints: every rule but the guard reads it as assembled, and its reads the guard would check are
// reported, unchecked, since the guard reads `has_ftle` where a branch condition does.
const USE_FTLE: bool = has_ftle;

fn colour(ctx: Ctx) -> vec3<f32> {
    let d = ctx.sample.diffusion;
    if (d != d) { // fires self-compare
        return vec3<f32>(0.0);
    }
    if (USE_FTLE && ctx.sample.ftle_valid) {
        return ramp_viridis(ctx.sample.ftle); // fires guard
    }
    return vec3<f32>(f32(ctx.quad.depth)); // fires scheduler-state
}
