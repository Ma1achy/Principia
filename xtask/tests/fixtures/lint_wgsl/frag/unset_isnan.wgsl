// Breaks the isinf-isnan rule: an unset check by a call to a function named `isnan`.
fn isnan(x: f32) -> bool { return (bitcast<u32>(x) & 0x7fffffffu) > 0x7f800000u; }

fn closure_or_zero(c: f32) -> f32 {
    let unset = isnan(c); // fires
    return select(c, 0.0, unset);
}
