// Every float rule holds. The near misses an over-broad finite-max rule would fail: comparisons against 65503.0
// (below f16's maximum), 3.4028233e38 and bitcast<f32>(0x7f7ffffeu) (below f32's) and 1.0, a clamp to 65504.0 (no
// comparison), and an unset value tested by its bits.
const PA_D_MIN_UNSET: u32 = 0x7c00u;

fn below_f16_max(d: f32) -> bool {
    return d > 65503.0;
}

fn below_f32_max(d: f32) -> bool {
    return d > 3.4028233e38;
}

fn below_f32_max_bits(d: f32) -> bool {
    return d > bitcast<f32>(0x7f7ffffeu);
}

fn above_one(d: f32) -> bool {
    return d > 1.0;
}

fn clamped(d: f32) -> f32 {
    return min(d, 65504.0);
}

fn pa_d_min_is_unset(w: u32) -> bool { return extractBits(w, 16u, 16u) == PA_D_MIN_UNSET; }
