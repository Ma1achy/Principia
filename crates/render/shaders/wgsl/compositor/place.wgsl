// The stain view's placement pass (render::stain_view): a fixed pass, like the compositor's, never assembled. It draws
// the stain view's layer, one texel per sample, into the caller's viewport at the placement `view.place`, the shown
// figure's rect, its centre (xy) and half-size (zw) in the viewport's normalized device coordinates, Y-up
// (gui::layout::Placement). The arithmetic is the GUI's real field's, `crates/gui/src/real/field.wgsl`, line for line,
// so the two put a sample on the same pixels: one full-viewport triangle; each pixel's field point `(s, t)`, `t` Y-up;
// nearest, never filtered. A pixel past `[0, 1]²`, and a texel the stain did not draw (alpha 0: no pipeline has
// compiled yet), is the canvas background, `view.background`, in the space the target is written in.
//
// The layer's rows are the field's from the bottom: layer row `r` holds field row `r` up from the bottom (the colour
// pass shades sample `y · width + x` at layer row `y`, and the records run row by row from the bottom left). So where
// `field.wgsl` reads image row `k` from the top, this reads layer row `height − 1 − k`.
//
// The layer is bound as the caller's target needs it: its linear view for an sRGB-typed target, which encodes on
// store; its encoded view (stored bytes, unconverted) for any other, so the bytes reach the target unchanged,
// gamma-space, as the GUI's own field image does.

struct PlaceUniform {
    place: vec4<f32>,
    background: vec4<f32>,
};

@group(0) @binding(0) var<uniform> view: PlaceUniform;
@group(0) @binding(1) var layer: texture_2d<f32>;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_place(@builtin(vertex_index) index: u32) -> VsOut {
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    var out: VsOut;
    out.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

@fragment
fn fs_place(in: VsOut) -> @location(0) vec4<f32> {
    let viewport = vec2<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0);
    let st = ((viewport - view.place.xy) / view.place.zw + vec2<f32>(1.0, 1.0)) * 0.5;
    if (any(st < vec2<f32>(0.0, 0.0)) || any(st >= vec2<f32>(1.0, 1.0))) {
        return vec4<f32>(view.background.rgb, 1.0);
    }
    let size = textureDimensions(layer);
    let texel = min(vec2<u32>(vec2<f32>(st.x, 1.0 - st.y) * vec2<f32>(size)), size - vec2<u32>(1u, 1u));
    let colour = textureLoad(layer, vec2<u32>(texel.x, size.y - 1u - texel.y), 0);
    if (colour.a < 0.5) {
        return vec4<f32>(view.background.rgb, 1.0);
    }
    return vec4<f32>(colour.rgb, 1.0);
}
