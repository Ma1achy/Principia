// The real engine's field, drawn from its image: one texel per sample, its outcome swatch as 8-bit sRGB, rows from the
// top. One full-viewport triangle; each pixel reads the texel of its field point `(s, t)`, `t` Y-up, at the placement
// `field.place`, the shown figure's rect in the viewport (`crate::layout::Placement`), nearest, never filtered, so a
// sample's colour is its swatch exactly. A sample with no colour (alpha 0, a decode failure) and a pixel past `[0, 1]²`
// are not part of the field this draws: the canvas background, `field.background`. The texture is not sRGB-typed, so
// the bytes reach the target unchanged, gamma-space, as egui-wgpu's own output is on the targets it chooses.

struct FieldUniform {
    // The shown rect's centre (xy) and half-size (zw) in the viewport's normalized device coordinates, Y-up.
    place: vec4<f32>,
    // The canvas background (rgb), gamma-space.
    background: vec4<f32>,
};

@group(0) @binding(0) var<uniform> field: FieldUniform;
@group(0) @binding(1) var image: texture_2d<f32>;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VsOut {
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    var out: VsOut;
    out.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let viewport = vec2<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0);
    let st = ((viewport - field.place.xy) / field.place.zw + vec2<f32>(1.0, 1.0)) * 0.5;
    if (any(st < vec2<f32>(0.0, 0.0)) || any(st >= vec2<f32>(1.0, 1.0))) {
        return vec4<f32>(field.background.rgb, 1.0);
    }
    let size = textureDimensions(image);
    let texel = min(vec2<u32>(vec2<f32>(st.x, 1.0 - st.y) * vec2<f32>(size)), size - vec2<u32>(1u, 1u));
    let colour = textureLoad(image, texel, 0);
    // A sample with no colour, a decode failure, is not a system: the canvas background.
    if (colour.a < 0.5) {
        return vec4<f32>(field.background.rgb, 1.0);
    }
    return vec4<f32>(colour.rgb, 1.0);
}
