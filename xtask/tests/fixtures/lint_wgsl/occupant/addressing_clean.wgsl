// Seam 6's clean case: the raster's addressing, the step scale, the payload and the descriptor.
fn colour(ctx: Ctx) -> vec3<f32> {
    let q = ctx.quad.uv + ctx.tile.uv + ctx.screen.uv + ctx.frag_xy;
    let n = f32(ctx.quad.index + ctx.quad.sample_count + ctx.params.horizon_steps);
    return vec3<f32>(q, n + ctx.sample.d_min + ctx.ic.m0);
}
