// The word inspector's whole-word hash (render contract Part 5 and Part 6, word views; debug_tooling_plan §B, §C):
// `dbg_hash_word`, the PCG fold over the `free_group_word`'s four limbs then `dbg_hash_u32`'s byte-to-RGB step (R-72;
// REQ-TOOL-155). Distinct words take distinct colours, so the view renders topological basins: word boundaries are
// finer than outcome boundaries (the Burrau topological-boundary diagnostic).
fn colour(ctx: Ctx) -> vec3<f32> {
    return dbg_hash_word(ctx.sample.word);
}
