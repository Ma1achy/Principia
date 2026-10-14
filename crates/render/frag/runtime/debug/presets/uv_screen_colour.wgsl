// The field's first two components as R and G, B = 0 (TASK-M1-07's coordinate presets).
fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].xy, 0.0); }
