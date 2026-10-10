// The mock engine's stand-in for the figure (R-390): smooth procedural noise, fractal value noise over the figure's
// rect, coloured from black through magenta and gold to cyan. Placeholder content, obvious as such; nothing from
// workbench/. It draws one full-viewport triangle and evaluates a fake field of the latent space at each pixel's IC
// through the mock's chart, z(s,t) = z₀ + (2s−1) q₁ + (2t−1) q₂ (chart_decoder_contract Part 3), `t` Y-up: so a pan
// shifts the picture and a zoom scales it about the centre, and a slice or a tilt changes it. `chart` carries z₀, q₁
// and q₂ projected on three directions: z_α, z_β and a fixed mix of the six hidden ones. Its output is gamma-space, as
// egui-wgpu's own output is on the targets it chooses.

struct ChartUniform {
    origin: vec4<f32>,
    q1: vec4<f32>,
    q2: vec4<f32>,
};

@group(0) @binding(0) var<uniform> chart: ChartUniform;

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

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let x = in.uv.x * 2.0 - 1.0;
    let y = 1.0 - in.uv.y * 2.0;
    let z = chart.origin.xyz + x * chart.q1.xyz + y * chart.q2.xyz;
    let p = (z.xy + vec2<f32>(1.0, 1.0)) * vec2<f32>(3.0, 2.5);
    let phase = vec2<f32>(1.7, -1.1) * z.z;
    let warp = vec2<f32>(fbm(p + vec2<f32>(3.1, 1.7) + phase), fbm(p + vec2<f32>(8.3, 2.8) - phase));
    let n = fbm(p + 2.5 * warp);
    let bands = fract(n * 4.0);
    let black = vec3<f32>(0.02, 0.02, 0.03);
    let magenta = vec3<f32>(0.93, 0.13, 0.55);
    let gold = vec3<f32>(0.98, 0.80, 0.12);
    let cyan = vec3<f32>(0.10, 0.75, 0.90);
    var colour = mix(black, magenta, smoothstep(0.0, 0.35, bands));
    colour = mix(colour, gold, smoothstep(0.45, 0.7, bands));
    colour = mix(colour, cyan, smoothstep(0.8, 1.0, bands) * n);
    return vec4<f32>(colour, 1.0);
}
