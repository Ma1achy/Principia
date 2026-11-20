// The colour-vision simulation pass, a fixed display-stage shader (dd_colouring §3.8; colour_composition §4.3; R-67,
// R-78, R-123, R-383): the last stage before the screen, on linear sRGB, after the gamut clamp. Not an occupant, not
// assembled, not scanned. Its CPU mirror is `crates/render/src/display/cvd.rs` (`simulate`), which derives every
// constant here in f64 from dd_colouring §3.8's data; these are those values rounded to f32.
//
// The mode codes are the Display window's order: 0 off, 1 deuteranopia, 2 protanopia, 3 tritanopia, 4 achromatopsia.
// Protan and deutan are Viénot, Brettel & Mollon 1999; tritan is Brettel, Viénot & Mollon 1997; both as the reference
// implementation R-383 names computes them, DaltonLens-Python at commit 3cba5e6a7c8f0e8199c8f83f1afb58eb6dab7a3d, on
// its LMSModel_sRGB_SmithPokorny75, at full dichromacy. Each matrix below is written row by row, each row a vec3, so
// the matrix is its rows' transpose and `c * M` is the row-major matrix applied to the column vector c.

// Viénot's protan transform on linear RGB: linearRGB_from_LMS · P_L · LMS_from_linearRGB.
const CVD_VIENOT_PROTAN = mat3x3<f32>(
    vec3<f32>(0.11238276, 0.88761723, 1.689865e-16),
    vec3<f32>(0.11238276, 0.88761723, -2.021786e-17),
    vec3<f32>(0.0040057683, -0.0040057683, 1.0),
);

// Viénot's deutan transform on linear RGB: linearRGB_from_LMS · P_M · LMS_from_linearRGB.
const CVD_VIENOT_DEUTAN = mat3x3<f32>(
    vec3<f32>(0.29275012, 0.7072499, 1.5555883e-16),
    vec3<f32>(0.29275012, 0.7072499, -1.6377951e-17),
    vec3<f32>(-0.022336587, 0.022336587, 1.0),
);

// Brettel's tritan transform on linear RGB on the separating plane's positive side: the half-plane through the anchor
// on that side, 660 nm (the reference's H1, its anchors swapped to put it first).
const CVD_BRETTEL_TRITAN_1 = mat3x3<f32>(
    vec3<f32>(1.0127727, 0.13548459, -0.14825726),
    vec3<f32>(-0.012432794, 0.86812055, 0.14431222),
    vec3<f32>(0.075890765, 0.8050025, 0.119106755),
);

// Brettel's tritan transform on linear RGB on its negative side: the half-plane through 485 nm (the reference's H2).
const CVD_BRETTEL_TRITAN_2 = mat3x3<f32>(
    vec3<f32>(0.9367812, 0.18978985, -0.1265711),
    vec3<f32>(0.061536532, 0.81526035, 0.12320311),
    vec3<f32>(-0.37562388, 1.1276655, 0.24795839),
);

// The separating plane's normal on linear RGB: a colour takes the second transform where its dot product with it is
// negative, else the first.
const CVD_BRETTEL_TRITAN_SEP = vec3<f32>(0.039014608, -0.027880758, -0.01113385);

// dd_colouring §3.8's M_achrom, whose every row is this.
const CVD_ACHROM_ROW = vec3<f32>(0.299, 0.587, 0.114);

// The simulation of the linear sRGB colour c under `mode`. Achromatopsia computes its one row's dot product once and
// writes it to every channel, so R = G = B exactly. An unknown code is off. The result is not clamped.
fn cvd_sim(mode: u32, c: vec3<f32>) -> vec3<f32> {
    switch mode {
        case 1u: {
            return c * CVD_VIENOT_DEUTAN;
        }
        case 2u: {
            return c * CVD_VIENOT_PROTAN;
        }
        case 3u: {
            if dot(c, CVD_BRETTEL_TRITAN_SEP) < 0.0 {
                return c * CVD_BRETTEL_TRITAN_2;
            }
            return c * CVD_BRETTEL_TRITAN_1;
        }
        case 4u: {
            return vec3<f32>(dot(c, CVD_ACHROM_ROW));
        }
        default: {
            return c;
        }
    }
}

struct CvdParams {
    mode: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var cvd_source: texture_2d<f32>;
@group(0) @binding(1) var<uniform> cvd_params: CvdParams;

// The pass: the finished layer's texel, read as linear colour, simulated, clamped to [0, 1] as the reference clips
// before it encodes, and written; the target's sRGB encoding, if it has one, is the store's.
@fragment
fn cvd(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let c = textureLoad(cvd_source, vec2<i32>(pos.xy), 0);
    return vec4<f32>(clamp(cvd_sim(cvd_params.mode, c.rgb), vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
}
