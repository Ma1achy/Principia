// Breaks the finite-max rule: a comparison against `bitcast<f32>(0x7f7fffffu)`, f32's largest finite bit pattern.
fn d_min_is_unset(d: f32) -> bool {
    return d > bitcast<f32>(0x7f7fffffu); // fires
}
