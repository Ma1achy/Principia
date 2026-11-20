// Every float rule holds: the unset check tests the bit pattern (R-343), and the other comparisons are against finite
// values that are no stand-in for inf.
const PA_D_MIN_UNSET: u32 = 0x7c00u;

fn pa_d_min_is_unset(w: u32) -> bool { return extractBits(w, 16u, 16u) == PA_D_MIN_UNSET; }

fn d_min_or_zero(w: u32) -> f32 {
    if pa_d_min_is_unset(w) {
        return 0.0;
    }
    let d = unpack2x16float(w).y;
    if d < 1.0 {
        return d;
    }
    return 1.0;
}
