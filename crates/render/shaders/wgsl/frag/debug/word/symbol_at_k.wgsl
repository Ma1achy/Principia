// The word inspector's symbol at slot `k` (render contract Part 5, "Live-state & array inspection"; debug_tooling_plan
// §C; payload §3): `fgw_symbol(word, u_k)`, decoded on the cold path, as the symbols' categorical colour
// (`dbg_cat(s, 4)`, `a = 0, A = 1, b = 2, B = 3`). The slot slider is `u_k`. Word-derived quantities are invalid once
// the word is truncated (payload §3), and a slot past the word's length holds no symbol (`FGW_NO_SYMBOL`): either way
// the view draws the hatch (applied per R-369, as REQ-TOOL-156 styles the reduced length).
// @uniform u_k: u32 = 0 [0, 75]

// The value the view shows: the symbol's code, NaN where there is none or the word is truncated.
fn value(ctx: Ctx) -> f32 {
    let w = ctx.sample.word;
    let s = fgw_symbol(w, uniforms.u_k);
    return select(canonical_nan(), f32(s), fgw_reduced_length_valid(w) && s != FGW_NO_SYMBOL);
}

fn colour(ctx: Ctx) -> vec3<f32> {
    let w = ctx.sample.word;
    let s = fgw_symbol(w, uniforms.u_k);
    if (!fgw_reduced_length_valid(w) || s == FGW_NO_SYMBOL) {
        return debug_invalid(ctx.frag_xy);
    }
    return dbg_cat(s, 4u);
}
