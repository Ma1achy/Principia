// The sample's UV coordinate, c + h·(2t − 1) (deep_zoom §1), a vector field in `.xy` (TASK-M1-07's coordinate presets).
fn source(ctx: Ctx) -> Field { return Field(ctx.quad.centre + ctx.quad.half_width * (2.0 * ctx.quad.uv - 1.0), 0.0, 0.0); }
