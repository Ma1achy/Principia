// The derived complexity-proxy view (render contract Part 6, the `times` row: "total_substeps_log2 from the u32";
// debug_tooling_plan §B, "DERIVED views"): `total_substeps_log2 = ⌊log₂ total_substeps⌋`, 0 for a total ≤ 1 (R-86),
// computed in the fragment from the exact stored u32 `total_substeps` by the read side's own `total_substeps_log2`
// (render contract Part 5), never a descriptor field, at its literal place on the viridis ramp (R-136).

// The value the view shows: the floor of the total's base-2 logarithm.
fn value(ctx: Ctx) -> f32 {
    return f32(ctx.sample.total_substeps_log2);
}

fn colour(ctx: Ctx) -> vec3<f32> {
    return dbg_sentinel(value(ctx), ctx.frag_xy);
}
