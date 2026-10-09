// The stain's context (render contract Part 1's `RenderContext`, its lanes as M1 fills them; R-369; TASK-M1-04). It
// follows the read side at assembly, whose `SimState` and `ICDescriptor` it holds, and precedes the node functions,
// which read it: the assembler places it, and `cargo xtask lint wgsl` lints the generated debug views after it
// (TASK-M1-08). `inputs` holds `render::assemble::MAX_INPUTS` fields.

// A field on a wire (render_gui_spec Part II §3): a scalar or categorical value in `.x`, a vector in `.xyz`.
alias Field = vec4<f32>;

// colour_composition §3's screen lane as the stain reads it: the pixel's post-flip UV in the target (TASK-M1-07).
struct CtxScreen {
    uv: vec2<f32>,
}

// colour_composition §3's quad lane as the stain reads it, its members in the order of the harness's quad lane
// (`render::bind::lanes`): the quad's `index`; its frame, deep_zoom §1's per-quad c and h (`centre`, `half_width`)
// and its top-left corner `tl`, in slice coords (the synthetic harness's at M1; TASK-M1-07); the sample's quad-local
// coordinate `uv` (t, deep_zoom §1); the quad's `sample_count`; then its `RenderQuad` members (dd_generation_root
// §3.7a) by `render::bind::QUAD_LANE`'s mapping, read from `quad_read` (TASK-M1-13; RQ-238).
struct CtxQuad {
    index: u32,
    tl: vec2<f32>,
    centre: vec2<f32>,
    half_width: vec2<f32>,
    uv: vec2<f32>,
    sample_count: u32,
    depth: u32,
    state: u32,
    impurity: f32,
    spread: f32,
    suspect_frac: f32,
    priority: f32,
    cache_age: u32,
    coherence: f32,
    ancestor_gap: u32,
    dominant_outcome: u32,
}

// colour_composition §3's tile lane as the stain reads it: the within-tile coordinate `uv`, from the tile's
// bottom-left corner, Y-up (R-72; REQ-COL-056), which render_gui_spec §12.1 draws tile boundaries from (TASK-M1-13).
struct CtxTile {
    uv: vec2<f32>,
}

// The absence NaN's bits, the canonical quiet NaN (lowering Part 3a), which an absent lane's u32 member holds: its
// `bitcast<f32>` is the absence NaN, which `is_absent_nan` tests (TASK-M1-13; applied per R-369).
const CTX_ABSENT_BITS: u32 = 0x7fc00000u;

// The quad lane where no raster places the sample in a quad (`shade_sample`): every f32 member the absence NaN, every
// u32 member its bits (TASK-M1-13).
fn ctx_quad_absent() -> CtxQuad {
    let a = bitcast<f32>(CTX_ABSENT_BITS);
    let v = vec2<f32>(a);
    let b = CTX_ABSENT_BITS;
    return CtxQuad(b, v, v, v, v, b, b, b, a, a, a, a, b, a, b, b);
}

// The tile lane where no raster places the sample in a tile (`shade_sample`): the absence NaN.
fn ctx_tile_absent() -> CtxTile {
    return CtxTile(vec2<f32>(bitcast<f32>(CTX_ABSENT_BITS)));
}

// What every node reads (render contract Part 1): the sample's read-side `SimState`, its `ICDescriptor` (`ic`, only
// the members the stain reads filled; RQ-227), the pixel's position for the invalid hatch (`debug_invalid`), its
// screen, quad and tile lanes, the node's wired fields, `inputs[k]` its k-th field input, and the read side's
// arguments, `params`, the uniforms the sample was read with, whose `horizon_steps` places a step index on
// `[0, horizon_steps]` (render_gui_spec §10.1; TASK-M1-09).
struct Ctx {
    sample: SimState,
    ic: ICDescriptor,
    frag_xy: vec2<f32>,
    screen: CtxScreen,
    quad: CtxQuad,
    tile: CtxTile,
    inputs: array<Field, 4>,
    params: ReadParams,
}
