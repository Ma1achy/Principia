enable f16;

// Breaks the finite-max rule: a comparison against 65504 spelled `65504h`, an f16 literal, in a hand-written file,
// which may hold f16.
fn d_min_is_unset(d: f16) -> bool {
    return d > 65504h; // fires
}
