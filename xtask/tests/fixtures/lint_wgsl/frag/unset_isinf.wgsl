// Breaks the isinf-isnan rule: an unset check by a call to a function named `isinf`.
fn isinf(x: f32) -> bool { return (bitcast<u32>(x) & 0x7fffffffu) == 0x7f800000u; }

fn d_min_or_zero(d: f32) -> f32 {
    if isinf(d) { // fires
        return 0.0;
    }
    return d;
}
