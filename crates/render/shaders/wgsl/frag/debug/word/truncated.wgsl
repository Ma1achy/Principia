// The word inspector's truncated flag (render contract Part 5, "Live-state & array inspection"; debug_tooling_plan
// §B, §C; payload §3): `fgw_truncated`, the length sentinel `length_raw == 127` (no flag bit), as a flag (`dbg_flag`:
// true green, false red). It fires iff a push arrived at the 76-symbol cap.
fn colour(ctx: Ctx) -> vec3<f32> {
    return dbg_flag(fgw_truncated(ctx.sample.word));
}
