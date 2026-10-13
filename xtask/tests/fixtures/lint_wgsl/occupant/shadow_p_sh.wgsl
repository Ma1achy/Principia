// The shadow rule (REQ-RENDER-018): a colour occupant reading `p_sh` colours the shadow as a sample.
fn colour(ctx: Ctx) -> vec3<f32> {
    let v = ctx.sample.p_sh; // fires shadow-read
    return vec3<f32>(v[0].x, v[1].y, 0.0);
}
