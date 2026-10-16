// The marks the mock engine draws into its stand-in (R-390): the hover path and the fake quads' bounds, one-pixel
// lines whose ends and colours the mock computes in Rust. A vertex is a figure point `(s, t)`, `t` Y-up, with its
// straight RGBA; it maps to the figure's viewport as the stand-in does, `(2s − 1, 2t − 1)` placed at `chart.place`, the
// shown figure's rect in the viewport (`noise.wgsl`; R-406). A line flagged `outside` draws no pixel whose centre is on
// the shown figure, `[0, 1]²` give or take `ON_SHOWN_SLACK`, which the canvas puts ahead of this file
// (`quads::on_shown`): the shown rect's edges lie on whole pixels, so such a line runs right up to that edge and never
// into it, the layer shown or hidden.

struct ChartUniform {
    origin: vec4<f32>,
    q1: vec4<f32>,
    q2: vec4<f32>,
    place: vec4<f32>,
    invalid: vec4<f32>,
};

@group(0) @binding(1) var<uniform> chart: ChartUniform;

struct VsIn {
    @location(0) at: vec2<f32>,
    @location(1) rgba: vec4<f32>,
    @location(2) outside: f32,
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) rgba: vec4<f32>,
    @location(1) at: vec2<f32>,
    @location(2) @interpolate(flat) outside: f32,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    let xy = in.at * 2.0 - vec2<f32>(1.0, 1.0);
    out.position = vec4<f32>(chart.place.xy + xy * chart.place.zw, 0.0, 1.0);
    out.rgba = in.rgba;
    out.at = in.at;
    out.outside = in.outside;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let on_shown = all(in.at >= vec2<f32>(-ON_SHOWN_SLACK)) && all(in.at <= vec2<f32>(1.0 + ON_SHOWN_SLACK));
    if (in.outside > 0.5 && on_shown) {
        discard;
    }
    return in.rgba;
}
