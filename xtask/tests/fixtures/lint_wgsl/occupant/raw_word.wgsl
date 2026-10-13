// Seam 5 (REQ-COL-005): the packed word decoded by hand, handed to a function no generated accessor is, stored and
// returned; and through a pointer.
fn word(ctx: Ctx) -> vec4<u32> {
    return ctx.sample.word; // fires raw-word
}

fn colour(ctx: Ctx) -> vec3<f32> {
    let len = extractBits(ctx.sample.word.w, 25u, 7u); // fires raw-word
    let hash = dbg_hash_word(ctx.sample.word); // fires raw-word
    var w: vec4<u32>;
    w = ctx.sample.word; // fires raw-word
    var s = ctx.sample;
    let x = s.word.x; // fires raw-word
    return hash + vec3<f32>(f32(len), f32(fgw_length_raw(w)), f32(x + word(ctx).y));
}
