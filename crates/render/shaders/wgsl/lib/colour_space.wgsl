// The colour spaces of dd_colouring §3.1, the shared library's colour-space maps (render_gui_spec §10.1): hand-written,
// following the generated prelude (`prelude.wgsl`) at assembly. The prelude holds the sRGB transfer, `srgb_to_linear`
// and `linear_to_srgb`, and OKLab → linear sRGB, `oklab_to_linear` (M₂⁻¹ closed, cubed, M₁⁻¹), which its ramps need;
// this file adds linear sRGB → OKLab (M₁, the cube root, M₂) and the maps built on the two. Each coefficient is
// Ottosson's, checked against his reference implementation (R-51) by `crates/render/tests/oklab_transcription.rs`.
// The CPU mirror is `crates/render/src/colour/space.rs`.
//
// "sRGB" is the encoded value and "linear" linear sRGB, the colour slot's space (render contract Parts 2 and 5): a
// node holding the slot's colour converts it with `linear_to_oklab`, an encoded one with `srgb_to_oklab`.

// The real cube root of each component, negative for a negative one: `pow` is undefined for a negative base, and for
// a zero one, which is mapped to 0 here.
fn oklab_cbrt(x: vec3<f32>) -> vec3<f32> {
    return select(sign(x) * pow(abs(x), vec3<f32>(1.0 / 3.0)), vec3<f32>(0.0), x == vec3<f32>(0.0));
}

// Linear sRGB to OKLab (dd_colouring §3.1): lms = M₁·rgb, the cube root of each, then Lab = M₂·lms^{1/3}.
fn linear_to_oklab(rgb: vec3<f32>) -> vec3<f32> {
    let lms = vec3<f32>(
        0.4122214708 * rgb.x + 0.5363325363 * rgb.y + 0.0514459929 * rgb.z,
        0.2119034982 * rgb.x + 0.6806995451 * rgb.y + 0.1073969566 * rgb.z,
        0.0883024619 * rgb.x + 0.2817188376 * rgb.y + 0.6299787005 * rgb.z,
    );
    let q = oklab_cbrt(lms);
    return vec3<f32>(
        0.2104542553 * q.x + 0.7936177850 * q.y - 0.0040720468 * q.z,
        1.9779984951 * q.x - 2.4285922050 * q.y + 0.4505937099 * q.z,
        0.0259040371 * q.x + 0.7827717662 * q.y - 0.8086757660 * q.z,
    );
}

// Encoded sRGB to OKLab: decoded by the prelude's `srgb_to_linear`, then `linear_to_oklab`.
fn srgb_to_oklab(c: vec3<f32>) -> vec3<f32> { return linear_to_oklab(srgb_to_linear(c)); }

// OKLab to encoded sRGB: the prelude's `oklab_to_linear`, then encoded by its `linear_to_srgb`.
fn oklab_to_srgb(lab: vec3<f32>) -> vec3<f32> { return linear_to_srgb(oklab_to_linear(lab)); }

// OKLab to OKLCH, its polar form (dd_colouring §3.1): (L, C, h), C = √(a² + b²), h = atan2(b, a) in radians, in
// [−π, π]. An achromatic colour, C = 0, has h = 0, atan2's value at (+0, +0), where WGSL leaves atan2 undefined.
fn oklab_to_oklch(lab: vec3<f32>) -> vec3<f32> {
    let c = sqrt(lab.y * lab.y + lab.z * lab.z);
    return vec3<f32>(lab.x, c, select(0.0, atan2(lab.z, lab.y), c > 0.0));
}

// OKLCH to OKLab (dd_colouring §3.1): (L, C cos h, C sin h), h in radians. WGSL bounds cos and sin on [−π, π], the
// range `oklab_to_oklch` gives.
fn oklch_to_oklab(lch: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(lch.x, lch.y * cos(lch.z), lch.y * sin(lch.z));
}
