// Breaks the finite-max rule: a comparison against f32's largest finite value spelled `3.4028235e38`.
fn d_min_is_unset(d: f32) -> bool {
    return d >= 3.4028235e38; // fires
}
