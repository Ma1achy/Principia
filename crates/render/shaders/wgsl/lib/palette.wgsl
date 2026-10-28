// The categorical palettes and the categorical filter (colour_composition §1.4; dd_colouring §3.7; R-77): hand-written,
// following the prelude and the presentation layer at assembly, so every node may call them. Every helper returns
// linear RGB, the colour slot's space. The CPU mirror is `crates/render/src/colour/palette.rs`.
//
// A categorical field other than the outcome state maps through a generated palette (dd_colouring §3.7): an Okabe–Ito
// cycle for n ≤ 8 classes, and for n > 8 the golden-angle hue θ_i = 2π·frac(i·φ_g), φ_g = (√5 − 1)/2, of every class,
// on the prelude's `hue_wheel`. The outcome state keeps its canonical palette, the occupant `outcome_state`.
//
// The categorical filter (colour_composition §1.4, "show class ∈ {…}, mute the rest"; REQ-COL-019) is the class mask
// `mask`, a `u32` uniform of the node: class i is shown when bit min(i, 31) is set, so bit 31 stands for class 31 and
// every class past it, and the default, every bit set, shows every class. A class not shown takes the mute transform,
// `cat_mute` (colour_composition §1.4; REQ-COL-058, R-72).

// The mute's OKLab lightness (colour_composition §1.4; REQ-COL-058): proposed (R-71, RQ-607), the midpoint, to two
// places, of the presentation views' invalid colour #141418 (L 0.193) and the "not yet" grey #4E4E4E (L 0.424).
const CAT_MUTE_L: f32 = 0.31;

// The mute's chroma factor (colour_composition §1.4; REQ-COL-058): proposed (R-71, RQ-607). A muted class keeps a
// quarter of its OKLab chroma, at its own hue.
const CAT_MUTE_CHROMA: f32 = 0.25;

// The generated index palette's colour of class `i` of `n` (dd_colouring §3.7): Okabe–Ito, cycling, for n ≤ 8; the
// golden-angle hue of class i, frac(i · φ_g) turns computed exactly as i times φ_g's 32-bit fixed point, on
// `hue_wheel`, for n > 8.
fn cat_palette(i: u32, n: u32) -> vec3<f32> {
    if (n <= 8u) {
        return dbg_srgb8(DBG_OKABE_ITO[i % 8u]);
    }
    return hue_wheel(f32(i * DBG_GOLDEN_FIXED) / 4294967296.0);
}

// The mute transform (colour_composition §1.4; REQ-COL-058): the class colour `c` in OKLab, its lightness replaced by
// CAT_MUTE_L and its a, b scaled by CAT_MUTE_CHROMA, back to linear RGB, each channel clamped to [0, 1].
fn cat_mute(c: vec3<f32>) -> vec3<f32> {
    let lab = linear_to_oklab(c);
    let m = vec3<f32>(CAT_MUTE_L, CAT_MUTE_CHROMA * lab.y, CAT_MUTE_CHROMA * lab.z);
    return clamp(oklab_to_linear(m), vec3<f32>(0.0), vec3<f32>(1.0));
}

// Whether the class mask `mask` shows class `k`: bit min(k, 31) set.
fn cat_shown(k: u32, mask: u32) -> bool {
    return ((mask >> min(k, 31u)) & 1u) == 1u;
}

// The categorical filter: `c`, the colour of class `k`, where the mask shows the class, else its mute.
fn cat_filter(c: vec3<f32>, k: u32, mask: u32) -> vec3<f32> {
    return select(cat_mute(c), c, cat_shown(k, mask));
}

// The categorical ramp: class `i` of `n` on the generated palette, through the class mask `mask`.
fn cat_ramp(i: u32, n: u32, mask: u32) -> vec3<f32> {
    return cat_filter(cat_palette(i, n), i, mask);
}
