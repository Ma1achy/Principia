// Breaks the finite-max rule: a comparison against 3.40282347e38, f32's largest finite value.
fn d_min_is_unset(d: f32) -> bool {
    return d > 3.40282347e38; // fires
}
