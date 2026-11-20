// δ normalised by its bound h = 2^−(ℓ+1) to [−1, 1] and mapped to (x + 1)/2: u's in R, v's in G, B = 0
// (REQ-TOOL-153).
fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>((ctx.inputs[0].xy / ctx.quad.half_width + 1.0) * 0.5, 0.0); }
