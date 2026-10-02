// Breaks the finite-max rule: a comparison against 65504 spelled `6.5504e4`.
fn d_min_is_unset(d: f32) -> bool {
    return d > 6.5504e4; // fires
}
