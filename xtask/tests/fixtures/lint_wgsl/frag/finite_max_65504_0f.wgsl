// Breaks the finite-max rule: a comparison against 65504 spelled `65504.0f`.
fn d_min_is_unset(d: f32) -> bool {
    return d > 65504.0f; // fires
}
