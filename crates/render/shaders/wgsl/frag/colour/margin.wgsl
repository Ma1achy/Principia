// The margin, a built-in occupant of the `colour` slot (render_gui_spec Part II §5, §3.1, §15 item 6; R-813;
// TASK-M7-12): a multi-source mapping. It declares two field inputs, `top1` and `top2`, so its node has two `field`
// in-ports, each taking one wire, never two wires on one port (§6), and shows the margin `top1 − top2`, each input's
// value its field's `.x`, on the greyscale ramp, `ramp_grey`, which clamps it to [0, 1]. A source keeps no input
// (§3.1): the two fields reach the margin as two wires from two sources, or from one (§5: fan-out is free).
//
// A margin that is not finite, by its bits, has no value: it is noted for the debug view (`note_non_finite()`) and
// draws the presentation views' invalid colour, `PRESENT_INVALID`, #141418 (R-467; render_gui_spec §13,
// "Validity-first"), never a place on the ramp.
// @input top1
// @input top2
fn colour(ctx: Ctx) -> vec3<f32> {
    let margin = ctx.inputs[0].x - ctx.inputs[1].x;
    if ((bitcast<u32>(margin) & 0x7f800000u) == 0x7f800000u) {
        note_non_finite();
        return PRESENT_INVALID;
    }
    return ramp_grey(margin);
}
