// Breaks the inf-nan-constant rule: an unset check against +inf, a constant expression of its bit pattern.
fn d_min_is_unset(d: f32) -> bool {
    return d == bitcast<f32>(0x7f800000u); // fires
}
