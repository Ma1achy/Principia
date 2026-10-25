// View › Debug view's arm (R-467 clause 3, D5; R-470 G4; REQ-COL-109): the one addition of the debug variant, a baked
// variant of the render's pipeline (R-41) that its caller selects, `RenderState`'s `debug_view` (TASK-M6-46,
// TASK-M8-46); the presentation variant is assembled without it, so a presentation view cannot draw a debug style
// (`render::assemble::View`). It draws over the active stain, whose colour `rgb` is: a pixel the chart classifies as not
// a system (`ctx.validity.forbidden`, REQ-COL-102) with the hatch, `debug_invalid`; a non-finite sample with the NaN /
// Inf style, `debug_nan_inf` (REQ-COL-110); any other failure with `debug_invalid`; every other pixel, a system, exactly
// as the stain draws it. The chart is read first, then non-finiteness, then the failure (R-467 D5). The CPU twin is
// `crate::debug_view`.
//
// The arm reads the context's members it needs as arguments, the lane, the sample and the pixel, not `Ctx` itself, so
// that `cargo xtask lint wgsl` reads it, a file of the stain's directory, after the read side whose `SimState` it
// takes; the generated `shade_at` passes them from its `ctx`.
//
// A non-finite sample is a `sim_failed` with a non-finite detail (0, NaN in state; 1, Inf or overflow in state; 2, a
// non-finite derived quantity; payload §2), a `decode_failed` of detail 0 (a non-finite decode output), or a sample
// whose stain read a non-finite value, which the stain notes through `note_non_finite()` (a field ramp's invalid lane,
// R-436). Any other failure is a `sim_failed` of detail 3, a `decode_failed` of detail 1, 2 or 3, or a reserved `state`
// code: every state but escape, bounded, collision and running.

// Whether the stain read a non-finite value at this pixel: private, so one invocation's, false until noted.
var<private> debug_non_finite: bool;

// The stain notes a non-finite value it read (R-436), which this variant draws in the NaN / Inf style.
fn note_non_finite() {
    debug_non_finite = true;
}

// The figure's pixels drawn in each of the "Debug" key's categories (render_gui_spec §G6; REQ-COL-109), raw pixel
// counts, not R-408's statistic: 0 "forbidden (chart)", 1 "decode or sim failure", 2 "NaN / Inf". Each fragment adds
// one to the category it draws, so after a draw each holds the number of the target's pixels drawn in it.
@group(3) @binding(0) var<storage, read_write> debug_counts: array<atomic<u32>, 3>;

// Whether the sample `s` is non-finite by its payload (payload §2): a `sim_failed` of detail 0, 1 or 2, or a
// `decode_failed` of detail 0.
fn debug_payload_non_finite(s: SimState) -> bool {
    return (s.state == STATE_SIM_FAILED && s.detail <= 2u) || (s.state == STATE_DECODE_FAILED && s.detail == 0u);
}

// Whether the sample `s` is a failure: no physical outcome and not running, so `sim_failed`, `decode_failed` or a
// reserved `state` code.
fn debug_payload_failed(s: SimState) -> bool {
    return s.state != STATE_ESCAPE && s.state != STATE_BOUNDED && s.state != STATE_COLLISION && s.state != STATE_RUNNING;
}

// The debug view over the stain's colour `rgb` at the pixel `frag_xy` of the sample `s`, whose chart classification is
// `forbidden` (`ctx.validity.forbidden`), its pixel counted.
fn debug_view(forbidden: bool, s: SimState, frag_xy: vec2<f32>, rgb: vec3<f32>) -> vec3<f32> {
    if (forbidden) {
        atomicAdd(&debug_counts[0], 1u);
        return debug_invalid(frag_xy);
    }
    if (debug_non_finite || debug_payload_non_finite(s)) {
        atomicAdd(&debug_counts[2], 1u);
        return debug_nan_inf(frag_xy);
    }
    if (debug_payload_failed(s)) {
        atomicAdd(&debug_counts[1], 1u);
        return debug_invalid(frag_xy);
    }
    return rgb;
}
