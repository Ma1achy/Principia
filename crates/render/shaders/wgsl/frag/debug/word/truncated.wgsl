// The word inspector's truncated flag (render contract Part 5, "Live-state & array inspection"; debug_tooling_plan
// §B, §C; payload §3): `fgw_truncated`, the length sentinel `length_raw == 127` (no flag bit), as a flag (`dbg_flag`:
// true green, false red). It fires iff a push arrived at the 76-symbol cap.
// @name truncated
// @group Debug · word
// @about Whether the word is truncated.
// @maps word truncated [flag]
// @glyph name
fn colour(ctx: Ctx) -> vec3<f32> {
    handles_validity();
    return dbg_flag(fgw_truncated(ctx.sample.word));
}
