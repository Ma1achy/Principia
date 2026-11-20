// Breaks the finite-max rule: a comparison against 65504 by `<`.
fn d_min_is_unset(d: f32) -> bool {
    return d < 65504.0; // fires
}
