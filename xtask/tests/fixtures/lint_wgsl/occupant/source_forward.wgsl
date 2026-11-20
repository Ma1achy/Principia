// A source forwards its own sample's field, `ctx.sample.ftle`, and is not held to the guard for it; another sample's
// `ftle` it reads, an aggregate's or a copied context's, is (render contract Part 4; R-255).
fn source(ctx: Ctx) -> Field {
    var sum = 0.0;
    for (var k = 1u; k < 4u; k = k + 1u) {
        let s = sample_read(k, 0.0, false, vec3<f32>(1.0), ctx.params);
        sum = max(sum, s.ftle); // fires guard
    }
    var other = ctx;
    other.sample = sample_read(1u, 0.0, false, vec3<f32>(1.0), ctx.params);
    return Field(ctx.sample.ftle, ctx.sample.ensemble_spread, sum, other.sample.ftle); // fires guard
}
