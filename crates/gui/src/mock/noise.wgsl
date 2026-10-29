// The mock engine's stand-in for the figure (R-390): smooth procedural noise, fractal value noise over the figure's
// rect, coloured from black through magenta and gold to cyan. Placeholder content, obvious as such; nothing from
// workbench/. It draws one full-viewport triangle and evaluates a fake field of the latent space at each pixel's IC
// through the mock's chart, z(s,t) = z₀ + (2s−1) q₁ + (2t−1) q₂ (chart_decoder_contract Part 3), `t` Y-up: so a pan
// shifts the picture and a zoom scales it about the centre, and a slice or a tilt changes it. `chart` carries z₀, q₁
// and q₂ projected on three directions: z_α, z_β and a fixed mix of the six hidden ones. Its output is gamma-space, as
// egui-wgpu's own output is on the targets it chooses. `fs_stain` draws the Stain editor's preview of the same
// stand-in in the look the mock takes from its stain graph (`stain.rs`): colour or a grey ramp, a late fade to
// `look.y`, multiplied, its lightness replaced, or passed through, and shifted by the source field's phase.
// `fs_main` draws the figure at `chart.place`, the shown figure's rect in the viewport (`crate::layout::Placement`):
// with the egui layer hidden the viewport is the window and the field continues past the shown rect at the same
// scale, an affine chart, each point where the shown layout puts it (R-406). A pixel whose field point lies in
// `chart.invalid`, the region the mock flags invalid (a test hook), past the shown rect or inside it, is not part of the
// chart: `fs_main`, the presentation view, draws it as the canvas background, `chart.background`, the colour the window
// is cleared to, with no hatch; `fs_debug`, the debug view (View › Debug view), with the shared prelude's hatch,
// `debug_invalid` (R-467 D2, D5, D8; R-407's fallback). They are two variants, so the presentation one holds no debug
// style (R-470 G4). The canvas puts the prelude ahead of this file, so its uniform block's binding, 0, is left to it.

struct ChartUniform {
    origin: vec4<f32>,
    q1: vec4<f32>,
    q2: vec4<f32>,
    // The shown rect's centre (xy) and half-size (zw) in the viewport's normalized device coordinates, Y-up.
    place: vec4<f32>,
    // The region flagged invalid, `[s₀, t₀, s₁, t₁)` of the field.
    invalid: vec4<f32>,
    // The canvas background (rgb), gamma-space as the stand-in's output is.
    background: vec4<f32>,
};

@group(0) @binding(1) var<uniform> chart: ChartUniform;

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

fn hash(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(127.1, 311.7));
    let r = q + dot(q, q.yx + vec2<f32>(19.19, 47.13));
    return fract(r.x * r.y);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm(p: vec2<f32>) -> f32 {
    var sum = 0.0;
    var amplitude = 0.5;
    var q = p;
    for (var octave = 0; octave < 5; octave++) {
        sum += amplitude * value_noise(q);
        q = q * 2.03 + vec2<f32>(17.0, 9.0);
        amplitude *= 0.5;
    }
    return sum;
}

struct StandIn {
    n: f32,
    bands: f32,
    warp: vec2<f32>,
};

// The stand-in's field at `uv` of the whole viewport, its noise shifted by `shift`.
fn stand_in(uv: vec2<f32>, shift: vec2<f32>) -> StandIn {
    return stand_in_at(vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0), shift);
}

// The stand-in's field at the field point `xy` = (2s − 1, 2t − 1), `t` Y-up, its noise shifted by `shift`: any point
// of the plane, the chart affine past `[0, 1]²`.
fn stand_in_at(xy: vec2<f32>, shift: vec2<f32>) -> StandIn {
    let z = chart.origin.xyz + xy.x * chart.q1.xyz + xy.y * chart.q2.xyz;
    let p = (z.xy + vec2<f32>(1.0, 1.0)) * vec2<f32>(3.0, 2.5) + shift;
    let phase = vec2<f32>(1.7, -1.1) * z.z;
    let warp = vec2<f32>(fbm(p + vec2<f32>(3.1, 1.7) + phase), fbm(p + vec2<f32>(8.3, 2.8) - phase));
    let n = fbm(p + 2.5 * warp);
    return StandIn(n, fract(n * 4.0), warp);
}

// The stand-in's colours: black through magenta and gold to cyan.
fn palette(f: StandIn) -> vec3<f32> {
    let black = vec3<f32>(0.02, 0.02, 0.03);
    let magenta = vec3<f32>(0.93, 0.13, 0.55);
    let gold = vec3<f32>(0.98, 0.80, 0.12);
    let cyan = vec3<f32>(0.10, 0.75, 0.90);
    var colour = mix(black, magenta, smoothstep(0.0, 0.35, f.bands));
    colour = mix(colour, gold, smoothstep(0.45, 0.7, f.bands));
    colour = mix(colour, cyan, smoothstep(0.8, 1.0, f.bands) * f.n);
    return colour;
}

// Whether the field point `xy` is not part of the chart: flagged invalid, past the shown rect or inside it.
fn forbidden(xy: vec2<f32>) -> bool {
    let st = (xy + vec2<f32>(1.0, 1.0)) * 0.5;
    return all(st >= chart.invalid.xy) && all(st < chart.invalid.zw);
}

// The field point of the fragment `in`: `(2s − 1, 2t − 1)` at the placement.
fn field_xy(in: VsOut) -> vec2<f32> {
    let viewport = vec2<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0);
    return (viewport - chart.place.xy) / chart.place.zw;
}

// The presentation view: a pixel that is not part of the chart is the canvas background.
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let xy = field_xy(in);
    if (forbidden(xy)) {
        return vec4<f32>(chart.background.rgb, 1.0);
    }
    return vec4<f32>(palette(stand_in_at(xy, vec2<f32>(0.0, 0.0))), 1.0);
}

// The debug view: a pixel that is not part of the chart is the hatch.
@fragment
fn fs_debug(in: VsOut) -> @location(0) vec4<f32> {
    let xy = field_xy(in);
    if (forbidden(xy)) {
        // The hatch from the pixel's position, as the render contract draws it; linear, so encoded as the stand-in is.
        return vec4<f32>(linear_to_srgb(debug_invalid(in.position.xy)), 1.0);
    }
    return vec4<f32>(palette(stand_in_at(xy, vec2<f32>(0.0, 0.0))), 1.0);
}

// The preview's look: colour (1) or a grey ramp (0); the darkest the late fade goes; the combiner, 0 Multiply,
// 1 Replace-L, 2 pass-through; and the source field's phase.
@group(0) @binding(2) var<uniform> look: vec4<f32>;

@fragment
fn fs_stain(in: VsOut) -> @location(0) vec4<f32> {
    let f = stand_in(in.uv, vec2<f32>(7.0, 3.0) * look.w);
    var base = vec3<f32>(f.n);
    if (look.x > 0.5) {
        base = palette(f);
    }
    // The time of fate, a second field of the stand-in: early fates full, late ones fading to `look.y`.
    let fate = clamp(f.warp.x * 1.4 - 0.2, 0.0, 1.0);
    let b = mix(1.0, look.y, fate);
    var colour = base;
    if (look.z < 0.5) {
        colour = base * b;
    } else if (look.z < 1.5) {
        let l = max(dot(base, vec3<f32>(0.2126, 0.7152, 0.0722)), 0.02);
        colour = clamp(base * (0.5 * b / l), vec3<f32>(0.0), vec3<f32>(1.0));
    }
    return vec4<f32>(colour, 1.0);
}
