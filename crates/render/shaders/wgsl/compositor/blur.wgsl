// The blur pass, a fixed compositor shader run twice, across then down: a separable blur (caching contract Part 5;
// lowering contract Part 3). Each output texel is `w[0]·c(p) + Σ_{k=1}^{taps−1} w[k]·(c(p + k·d) + c(p − k·d))`,
// `d` the pass's direction and each read clamped to the texture's edge. The weights are the caller's, `w[k]` in
// `weights[k / 4][k % 4]`; the shader holds no blur constant.
struct CmpBlur {
    direction: vec2<i32>,
    taps: u32,
    _pad: u32,
    weights: array<vec4<f32>, 4>,
}
@group(0) @binding(0) var cmp_blur_source: texture_2d<f32>;
@group(0) @binding(1) var<uniform> cmp_blur: CmpBlur;

fn cmp_blur_weight(k: u32) -> f32 {
    return cmp_blur.weights[k / 4u][k % 4u];
}

fn cmp_blur_load(p: vec2<i32>) -> vec4<f32> {
    let last = vec2<i32>(textureDimensions(cmp_blur_source)) - vec2<i32>(1);
    return textureLoad(cmp_blur_source, clamp(p, vec2<i32>(0), last), 0);
}

@fragment
fn blur(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<i32>(pos.xy);
    var sum = cmp_blur_load(p) * cmp_blur_weight(0u);
    for (var k = 1u; k < cmp_blur.taps; k += 1u) {
        let d = cmp_blur.direction * i32(k);
        sum += (cmp_blur_load(p + d) + cmp_blur_load(p - d)) * cmp_blur_weight(k);
    }
    return sum;
}
