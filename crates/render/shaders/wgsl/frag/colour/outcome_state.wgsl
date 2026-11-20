// The outcome state's canonical default palette (colour_composition §1.4; R-77, R-444 to R-447; REQ-COL-002,
// REQ-COL-101, REQ-COL-103, REQ-COL-108), a built-in occupant of the `colour` slot. The outcome view shows physical
// outcomes only, in nine classes (R-444), read from `state` and the `detail` union alone (payload §2): a collision's
// colour is its pair's, an escape's its body's (0-based inside, R-22), `detail = 3` the triple collision's or the triple
// ejection's. `t_end_step` is not read: a collision at `t_end_step == 0` draws its pair's or the triple collision's
// colour (R-445). Black, bounded / nothing yet, means nothing has happened yet (R-446): `bounded`, `running`,
// `sim_failed` of every `detail` and a reserved `state` code all draw it, never the "not yet" stipple or the hatch,
// which stay the debug views' and every other field's. It is a presentation view and draws no hatch (R-467): its first
// test is the sample's chart classification, `ctx.validity.forbidden` (REQ-COL-102; colour_composition §3's validity
// lane), the chart's own and never the payload's `decode_failed` detail; a sample the chart classifies as not a system
// draws the canvas background, `canvas_background()`, the uniform its caller sets to the colour its window is cleared
// to, blank, never a palette colour (R-467, R-470); and a `decode_failed` sample of every `detail` and tag draws it
// too, as a pixel that is not a system (R-511).
//
// Each class's swatch is a node param, so editing one changes only its class and recompiles nothing (REQ-COL-002;
// lowering Part 3): a linear RGB colour, the colour slot's space (render contract Part 2), as a field ramp's
// `INVALID_COLOUR` is, its default each channel the f32 nearest the sRGB decode of the 8-bit code given beside it. The
// fragment returns it unchanged, so a swatch renders the same bits on every backend. Two named sets (R-447;
// REQ-COL-103): the sRGB palette, the default, and the OKLab-uniform variant, each class at its own OKLab hue at L =
// 0.72 and C = 0.123, bounded staying #141418; each set's swatches are params of their own, so editing one leaves the
// other unchanged, and `swatch_set` selects the set shown, 0 the sRGB palette, 1 the variant. The labels are R-449's
// names as shown, 1-based. The CPU twin is `crates/render/src/colour/outcome.rs`.
//
// The categorical filter (colour_composition §1.4; REQ-COL-019): `class_mask` shows class k, its index in the sRGB
// table's order, bounded / nothing yet 0 to triple ejection 8, where bit k is set, and mutes every other class by
// `cat_mute` (`shaders/wgsl/lib/palette.wgsl`; REQ-COL-058); every bit set, the default, shows every class. A sample
// with no class, one the chart classifies as not a system or a `decode_failed` sample, draws the canvas background
// whatever the mask.
//
// The set shown: 0 the sRGB palette (the default), 1 the OKLab-uniform variant.
// @uniform swatch_set: u32 = 0 [0, 1]
// The sRGB palette (R-444, R-445).
// bounded / nothing yet, black: #141418.
// @uniform bounded: vec3<f32> = (0.00699541, 0.00699541, 0.009134059)
// collision 1–2 (pair 2), red: #DE2D2D.
// @uniform collision_pair_2: vec3<f32> = (0.73046076, 0.026241222, 0.026241222)
// collision 1–3 (pair 1), green: #2EBC4E.
// @uniform collision_pair_1: vec3<f32> = (0.027320892, 0.5028865, 0.07618538)
// collision 2–3 (pair 0), blue: #3462E0.
// @uniform collision_pair_0: vec3<f32> = (0.034339808, 0.122138776, 0.7454042)
// triple collision (a collision, detail = 3), orange: #F29620.
// @uniform triple_collision: vec3<f32> = (0.8879231, 0.3049873, 0.014443844)
// escape body 1 (detail 0), yellow: #F0DE32.
// @uniform escape_body_0: vec3<f32> = (0.8713671, 0.73046076, 0.031896032)
// escape body 2 (detail 1), magenta: #E034C6.
// @uniform escape_body_1: vec3<f32> = (0.7454042, 0.034339808, 0.5647115)
// escape body 3 (detail 2), cyan: #30C8DC.
// @uniform escape_body_2: vec3<f32> = (0.029556835, 0.57758045, 0.7156935)
// triple ejection (an escape, detail = 3), purple: #611597.
// @uniform triple_ejection: vec3<f32> = (0.11953843, 0.007499032, 0.30946892)
// The OKLab-uniform variant (R-447), each chromatic class at OKLab L = 0.72, C = 0.123 and its hue.
// bounded / nothing yet: #141418.
// @uniform oklab_bounded: vec3<f32> = (0.00699541, 0.00699541, 0.009134059)
// collision 1–2, 27°: #E7857A.
// @uniform oklab_collision_pair_2: vec3<f32> = (0.7991027, 0.23455058, 0.19461784)
// collision 1–3, 147°: #6EB976.
// @uniform oklab_collision_pair_1: vec3<f32> = (0.15592647, 0.48514995, 0.18116425)
// collision 2–3, 265°: #7FA2F2.
// @uniform oklab_collision_pair_0: vec3<f32> = (0.21223076, 0.3613068, 0.8879231)
// triple collision, 65°: #D99349.
// @uniform oklab_triple_collision: vec3<f32> = (0.69387174, 0.29177064, 0.06662594)
// escape body 1, 103°: #B3A841.
// @uniform oklab_escape_body_0: vec3<f32> = (0.4507858, 0.39157248, 0.052860647)
// escape body 2, 335°: #D486C3.
// @uniform oklab_escape_body_1: vec3<f32> = (0.65837485, 0.23839757, 0.54572445)
// escape body 3, 208°: #09B9CD.
// @uniform oklab_escape_body_2: vec3<f32> = (0.0027317428, 0.48514995, 0.61049557)
// triple ejection, 305°: #B790E2.
// @uniform oklab_triple_ejection: vec3<f32> = (0.47353148, 0.27889428, 0.7605245)
// The categorical filter's class mask: bit k shows class k; every bit set shows every class.
// @uniform class_mask: u32 = 4294967295 [0, 4294967295]

// The swatch of the sample's class in the set shown, or the canvas background for a sample with no class.
fn outcome_swatch(ctx: Ctx) -> vec3<f32> {
    if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) {
        return canvas_background();
    }
    let v = uniforms.swatch_set == 1u;
    let s = ctx.sample.state;
    let d = ctx.sample.detail;
    if (s == STATE_COLLISION) {
        if (d == 0u) {
            return select(uniforms.collision_pair_0, uniforms.oklab_collision_pair_0, v);
        }
        if (d == 1u) {
            return select(uniforms.collision_pair_1, uniforms.oklab_collision_pair_1, v);
        }
        if (d == 2u) {
            return select(uniforms.collision_pair_2, uniforms.oklab_collision_pair_2, v);
        }
        return select(uniforms.triple_collision, uniforms.oklab_triple_collision, v);
    }
    if (s == STATE_ESCAPE) {
        if (d == 0u) {
            return select(uniforms.escape_body_0, uniforms.oklab_escape_body_0, v);
        }
        if (d == 1u) {
            return select(uniforms.escape_body_1, uniforms.oklab_escape_body_1, v);
        }
        if (d == 2u) {
            return select(uniforms.escape_body_2, uniforms.oklab_escape_body_2, v);
        }
        return select(uniforms.triple_ejection, uniforms.oklab_triple_ejection, v);
    }
    return select(uniforms.bounded, uniforms.oklab_bounded, v);
}

// The sample's class, its index in the sRGB table's order (R-448's rank), or -1 for a sample with no class: one the
// chart classifies as not a system, or a `decode_failed` sample (R-467, R-511).
fn outcome_class(ctx: Ctx) -> i32 {
    let classed = !ctx.validity.forbidden && ctx.sample.state != STATE_DECODE_FAILED;
    if (!classed) {
        return -1;
    }
    let detail = i32(ctx.sample.detail);
    if (ctx.sample.state == STATE_COLLISION) {
        return select(3 - detail, 4, detail == 3);
    }
    if (ctx.sample.state == STATE_ESCAPE) {
        return select(5 + detail, 8, detail == 3);
    }
    return 0;
}

fn colour(ctx: Ctx) -> vec3<f32> {
    let rgb = outcome_swatch(ctx);
    let k = outcome_class(ctx);
    if (k < 0) {
        return rgb;
    }
    return cat_filter(rgb, u32(k), uniforms.class_mask);
}
