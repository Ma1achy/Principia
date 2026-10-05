// The backdrop pass, a fixed compositor shader (lowering contract Part 3, Part 4; caching contract Part 5): the layer
// that becomes the backdrop, the stale layer or the snapshot, copied into the backdrop render target, texel for texel.
// The two textures are the same size.
@group(0) @binding(0) var cmp_backdrop_source: texture_2d<f32>;

@fragment
fn backdrop(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    return textureLoad(cmp_backdrop_source, vec2<i32>(pos.xy), 0);
}
