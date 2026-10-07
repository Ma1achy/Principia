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

// colour_composition §3's quad lane as the stain reads it: the sample's quad-local coordinate `uv` (t, deep_zoom §1),
// and the quad's `centre` and `half_width`, deep_zoom §1's per-quad c and h (the synthetic harness's at M1;
// TASK-M1-07).
struct CtxQuad {
    uv: vec2<f32>,
    centre: vec2<f32>,
    half_width: vec2<f32>,
}

// What every node reads (render contract Part 1): the sample's read-side `SimState`, its `ICDescriptor` (`ic`, only
// the members the stain reads filled; RQ-227), the pixel's position for the invalid hatch (`debug_invalid`), its
// screen and quad lanes, and the node's wired fields, `inputs[k]` its k-th field input.
struct Ctx {
    sample: SimState,
    ic: ICDescriptor,
    frag_xy: vec2<f32>,
    screen: CtxScreen,
    quad: CtxQuad,
    inputs: array<Field, 4>,
}
