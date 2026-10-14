// Seam 6 (REQ-COL-005): the quad's frame in the slice plane is the chart's.
fn colour(ctx: Ctx) -> vec3<f32> {
    let tl = ctx.quad.tl; // fires chart-state
    let c = ctx.quad.centre; // fires chart-state
    let h = ctx.quad.half_width; // fires chart-state
    return vec3<f32>(tl + c, h.x);
}
