// The stain's context (render contract Part 1's `RenderContext`, its lanes as M1 fills them; R-369; TASK-M1-04). It
// follows the read side at assembly, whose `SimState` and `ICDescriptor` it holds, and precedes the node functions,
// which read it: the assembler places it, and `cargo xtask lint wgsl` lints the generated debug views after it
// (TASK-M1-08). `inputs` holds `render::assemble::MAX_INPUTS` fields.

// A field on a wire (render_gui_spec Part II §3): a scalar or categorical value in `.x`, a vector in `.xyz`.
alias Field = vec4<f32>;

// What every node reads (render contract Part 1): the sample's read-side `SimState`, its `ICDescriptor` (`ic`, only
// the members the stain reads filled; RQ-227), the pixel's position for the invalid hatch (`debug_invalid`), the node's
// wired fields, `inputs[k]` its k-th field input, and the read side's arguments, `params`, the uniforms the sample was
// read with, whose `horizon_steps` places a step index on `[0, horizon_steps]` (render_gui_spec §10.1; TASK-M1-09).
struct Ctx {
    sample: SimState,
    ic: ICDescriptor,
    frag_xy: vec2<f32>,
    inputs: array<Field, 4>,
    params: ReadParams,
}
