// Breaks the finite-max rule: a comparison against `bitcast<f32>(0x477fe000u)`, 65504's f32 bit pattern.
fn d_min_is_unset(d: f32) -> bool {
    return d > bitcast<f32>(0x477fe000u); // fires
}
