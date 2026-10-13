// Seam 5's clean case: the word handed whole to generated accessors, by value and through a pointer.
fn colour(ctx: Ctx) -> vec3<f32> {
    let w = ctx.sample.word;
    if (!fgw_reduced_length_valid(w)) {
        return debug_invalid(ctx.frag_xy);
    }
    var s = ctx.sample;
    let n = fgw_length_raw(s.word);
    let last = sd_last_symbol_valid(fgw_length_raw(ctx.sample.word));
    return vec3<f32>(f32(fgw_reduced_length(w)), f32(n), select(0.0, 1.0, last));
}
