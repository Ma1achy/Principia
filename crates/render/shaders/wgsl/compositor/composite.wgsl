// The composite pass, a fixed compositor shader (lowering contract Part 3, Part 4; caching contract Part 5): the fresh
// layer over the backdrop, `fresh + backdrop · (1 − fresh.a)`, the colours premultiplied. The colour pass covers its
// layer opaquely, alpha 1, and a pixel it did not draw is cleared to 0, so the backdrop shows only where the fresh
// layer has nothing. All three are the same size.
@group(0) @binding(0) var cmp_fresh: texture_2d<f32>;
@group(0) @binding(1) var cmp_backdrop: texture_2d<f32>;

@fragment
fn composite(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<i32>(pos.xy);
    let fresh = textureLoad(cmp_fresh, p, 0);
    let back = textureLoad(cmp_backdrop, p, 0);
    return fresh + back * (1.0 - fresh.a);
}
