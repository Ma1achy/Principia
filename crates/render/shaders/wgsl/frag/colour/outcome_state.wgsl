// The outcome state's canonical default palette (colour_composition §1.4; R-77, R-96; REQ-COL-002, REQ-COL-062), a
// built-in occupant of the `colour` slot. Each sample's class is read from `state` plus the `detail` union (payload §2)
// and `t_end_step`: a collision's colour is its pair's, an escape's its body's (0-based, R-22), `detail = 3` the triple
// outcome's; "degenerate" is `decode_failed`, and "collision @ t=0" is a collision with `t_end_step == 0`, whatever its
// `detail` (R-96). `running` shows the neutral "not yet" grey, `DBG_NOT_YET` (REQ-COL-053), and `sim_failed` the
// invalid pattern (colour_composition §3), as do the reserved codes 6–7, which read as finished and untrusted
// (payload §2, §6). A RUNNING sample is coloured like any class (render contract Part 4; REQ-RENDER-017).
//
// Each class's swatch is a node param, so editing one changes only its class and recompiles nothing (REQ-COL-002;
// lowering Part 3): a linear RGB colour, the colour slot's space (render contract Part 2), as a field ramp's
// `INVALID_COLOUR` is, its default each channel the f32 nearest the sRGB decode of the 8-bit code given beside it. The
// fragment returns it unchanged, so a swatch renders the same bits on every backend. The CPU twin is
// `crates/render/src/colour/outcome.rs`.
//
// collision 0–1 (pair 2), red: #DE2D2D.
// @uniform collision_pair_2: vec3<f32> = (0.73046076, 0.026241222, 0.026241222)
// collision 0–2 (pair 1), green: #2EBC4E.
// @uniform collision_pair_1: vec3<f32> = (0.027320892, 0.5028865, 0.07618538)
// collision 1–2 (pair 0), blue: #3462E0.
// @uniform collision_pair_0: vec3<f32> = (0.034339808, 0.122138776, 0.7454042)
// bounded, black: #141418.
// @uniform bounded: vec3<f32> = (0.00699541, 0.00699541, 0.009134059)
// degenerate (decode_failed), white: #ECECF0.
// @uniform degenerate: vec3<f32> = (0.838799, 0.838799, 0.8713671)
// body 0 escape, yellow: #F0DE32.
// @uniform escape_body_0: vec3<f32> = (0.8713671, 0.73046076, 0.031896032)
// body 1 escape, magenta: #E034C6.
// @uniform escape_body_1: vec3<f32> = (0.7454042, 0.034339808, 0.5647115)
// body 2 escape, cyan: #30C8DC.
// @uniform escape_body_2: vec3<f32> = (0.029556835, 0.57758045, 0.7156935)
// collision at t = 0 (t_end_step == 0), orange: #F29620.
// @uniform collision_at_start: vec3<f32> = (0.8879231, 0.3049873, 0.014443844)
// triple collision (a collision, detail = 3), lavender: #D6A1FF. Proposed, R-71 (REQ-COL-062, RQ-234).
// @uniform triple_collision: vec3<f32> = (0.67244315, 0.35640013, 1.0)
// triple ejection (an escape, detail = 3), navy: #000097. Proposed, R-71 (REQ-COL-062, RQ-234).
// @uniform triple_ejection: vec3<f32> = (0.0, 0.0, 0.30946892)

fn colour(ctx: Ctx) -> vec3<f32> {
    let s = ctx.sample.state;
    let d = ctx.sample.detail;
    if (s == STATE_RUNNING) {
        return DBG_NOT_YET;
    }
    if (s == STATE_BOUNDED) {
        return uniforms.bounded;
    }
    if (s == STATE_DECODE_FAILED) {
        return uniforms.degenerate;
    }
    if (s == STATE_COLLISION) {
        if (ctx.sample.t_end_step == 0u) {
            return uniforms.collision_at_start;
        }
        if (d == 0u) {
            return uniforms.collision_pair_0;
        }
        if (d == 1u) {
            return uniforms.collision_pair_1;
        }
        if (d == 2u) {
            return uniforms.collision_pair_2;
        }
        return uniforms.triple_collision;
    }
    if (s == STATE_ESCAPE) {
        if (d == 0u) {
            return uniforms.escape_body_0;
        }
        if (d == 1u) {
            return uniforms.escape_body_1;
        }
        if (d == 2u) {
            return uniforms.escape_body_2;
        }
        return uniforms.triple_ejection;
    }
    return debug_invalid(ctx.frag_xy);
}
