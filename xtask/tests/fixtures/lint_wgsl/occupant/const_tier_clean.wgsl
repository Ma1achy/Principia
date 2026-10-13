// A tier constant used in a `const` declaration, by an occupant that reads neither `ftle` nor `ensemble_spread`: it
// lints, and every rule holds.
const SHADE: f32 = select(0.25, 0.75, has_ftle);

fn colour(ctx: Ctx) -> vec3<f32> {
    return vec3<f32>(SHADE * ctx.sample.t_end_fraction);
}
