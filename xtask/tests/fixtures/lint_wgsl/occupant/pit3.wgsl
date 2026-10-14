// PIT-3 (TASK-M1-15): one colour occupant breaking four rules, each shown able to fire: an `isnan`, a `raw != raw`, a
// raw payload read and an `r_sh` read.
fn isnan(x: f32) -> bool { return (bitcast<u32>(x) & 0x7fffffffu) > 0x7f800000u; }

fn colour(ctx: Ctx) -> vec3<f32> {
    let raw = ctx.sample.d_min;
    let unset = isnan(raw); // fires isinf-isnan
    let self_ne = raw != raw; // fires self-compare
    let packed = simstate_buffer[0u].packed_a; // fires payload-buffer
    let shadow = ctx.sample.r_sh[0].x; // fires shadow-read
    return vec3<f32>(select(0.0, 1.0, unset || self_ne), f32(packed), shadow);
}
