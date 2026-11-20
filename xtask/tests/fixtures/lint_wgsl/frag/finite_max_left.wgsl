// Breaks the finite-max rule: a comparison against 65504 with the constant on the left.
fn d_min_is_unset(d: f32) -> bool {
    return 65504.0 < d; // fires
}
