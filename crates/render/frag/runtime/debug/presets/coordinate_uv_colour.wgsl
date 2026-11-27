// The field's first two components as R and G, B = 0.
fn colour(ctx: Ctx) -> vec3<f32> { handles_validity(); return vec3<f32>(ctx.inputs[0].xy, 0.0); }
