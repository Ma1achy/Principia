// The word inspector's reduced length (render contract Part 5, "Live-state & array inspection"; Part 6, word views;
// payload §3, §5): `fgw_reduced_length`, the net reduced branch-cut crossings, on the viridis ramp over [0, 76]
// (`FGW_CAPACITY`). Invalid-styled when truncated (R-72; REQ-TOOL-156): where `fgw_reduced_length_valid` is false, a
// failed validity predicate as R-245 reads one, the view draws `debug_invalid(frag_xy)`, the hatch, never 127; the raw
// `length` view still shows 127 as its literal value (R-136). The predicate is the validity lane's for the word
// (`render::bind::validity_over`).

// The value the view shows: the reduced length, NaN where it is invalid.
fn value(ctx: Ctx) -> f32 {
    let w = ctx.sample.word;
    return select(canonical_nan(), f32(fgw_reduced_length(w)), fgw_reduced_length_valid(w));
}

fn colour(ctx: Ctx) -> vec3<f32> {
    let w = ctx.sample.word;
    if (!fgw_reduced_length_valid(w)) {
        return debug_invalid(ctx.frag_xy);
    }
    return dbg_lin(f32(fgw_reduced_length(w)), 0.0, f32(FGW_CAPACITY));
}
