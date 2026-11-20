// The pixel's post-flip UV in the view, a vector field in `.xy` (TASK-M1-07's coordinate presets).
fn source(ctx: Ctx) -> Field { return Field(ctx.screen.uv, 0.0, 0.0); }
