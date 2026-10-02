// Breaks the inf-nan-constant rule: a comparison against a NaN constant, a module constant of its bit pattern.
const NAN_BITS: u32 = 0x7fc00000u;

fn closure_is_unset(c: f32) -> bool {
    return c != bitcast<f32>(NAN_BITS); // fires
}
