// The word inspector's whole-word hash (render contract Part 5 and Part 6, word views; debug_tooling_plan §B, §C):
// `dbg_hash_word`, the PCG fold over the `free_group_word`'s four limbs then `dbg_hash_u32`'s byte-to-RGB step (R-72;
// REQ-TOOL-155). Distinct words take distinct colours, so the view renders topological basins: word boundaries are
// finer than outcome boundaries (the Burrau topological-boundary diagnostic). Word-derived quantities are invalid
// once the word is truncated (payload §3): the retained prefix is not the reduced word, so a truncated word draws the
// hatch, not a basin's colour (applied per R-369, as the reduced-length and symbol-at-k views do).
fn colour(ctx: Ctx) -> vec3<f32> {
    let w = ctx.sample.word;
    if (!fgw_reduced_length_valid(w)) {
        return debug_invalid(ctx.frag_xy);
    }
    return dbg_hash_word(w);
}
