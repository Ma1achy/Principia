// The margin, a built-in occupant of the `colour` slot (render_gui_spec Part II §5, §3.1, §15 item 6; R-813;
// TASK-M7-12): a multi-source mapping. It declares two field inputs, `top1` and `top2`, so its node has two `field`
// in-ports, each taking one wire, never two wires on one port (§6), and shows the margin `top1 − top2`, each input's
// value its field's `.x`. A source keeps no input (§3.1): the two fields reach the margin as two wires from two
// sources, or from one (§5: fan-out is free).
//
// **The map is R-962's** (colour_composition §1.2, "Default ramps by field role"): the margin is drawn on the
// greyscale ramp, `ramp_grey`, OKLab L equal to `top1 − top2` clamped to [0, 1], white where `top1` is far above
// `top2`, so a margin below 0 draws the ramp's black (a swapped pair of wires is all black) and one above 1 its white.
// The last line below is that map; the two inputs and the guards before it are the node's.
//
// It is a presentation view and draws no hatch (R-467), so it guards itself, as the outcome state and a field ramp
// do (render contract "Presentation layer"; render_gui_spec §G1, §13 "Validity-first"): a pixel the chart classifies
// as not a system, `ctx.validity.forbidden`, and a `decode_failed` sample draw the canvas background, blank, never a
// place on the map (R-467, R-470, R-511); a `sim_failed` sample and a reserved `state` code draw the presentation
// views' invalid colour, `PRESENT_INVALID`, #141418, whatever the inputs hold; and a margin that is not finite has
// no value: it is noted for the debug view (`note_non_finite()`) and draws #141418 too. Not finite is an input that
// is not finite or a difference that is not (two finite inputs may overflow), each by its bits: the inputs are
// tested themselves because a backend's fast-math may fold `x − x` to 0 where one source feeds both ports, and ∞ − ∞
// must not draw the map's zero on one backend and #141418 on another.
// @input top1
// @input top2
// Whether `x` is ±∞ or a NaN, by its bits: every exponent bit set.
fn margin_non_finite(x: f32) -> bool {
    return (bitcast<u32>(x) & 0x7f800000u) == 0x7f800000u;
}

fn colour(ctx: Ctx) -> vec3<f32> {
    if (ctx.validity.forbidden || ctx.sample.state == STATE_DECODE_FAILED) {
        return canvas_background();
    }
    if (ctx.sample.state == STATE_SIM_FAILED || ctx.sample.state > STATE_DECODE_FAILED) {
        return PRESENT_INVALID;
    }
    let top1 = ctx.inputs[0].x;
    let top2 = ctx.inputs[1].x;
    let margin = top1 - top2;
    if (margin_non_finite(top1) || margin_non_finite(top2) || margin_non_finite(margin)) {
        note_non_finite();
        return PRESENT_INVALID;
    }
    return ramp_grey(margin); // the map R-962 rules
}
