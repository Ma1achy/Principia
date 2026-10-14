// The shadow rule (REQ-RENDER-018): a colour occupant reading `p_sh` colours the shadow as a sample, and a helper
// reading `r_sh` through a pointer.
fn colour(ctx: Ctx) -> vec3<f32> {
    let v = ctx.sample.p_sh; // fires shadow-read
    return vec3<f32>(v[0].x, v[1].y, 0.0);
}

// Through a pointer argument, whose type is the pointer's own handle.
fn shadow_of(s: ptr<function, SimState>) -> vec2<f32> {
    return (*s).r_sh[0]; // fires shadow-read
}
