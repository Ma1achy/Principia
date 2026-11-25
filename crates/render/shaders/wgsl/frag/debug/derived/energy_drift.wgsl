// The derived current-drift view (debug_tooling_plan §B, "DERIVED views"; payload §5): `ΔE = H(r, p) − E_0`, computed
// in the fragment from the live f32 state through the read side's own `hamiltonian` (TASK-M1-01), with the sample's
// `ICDescriptor` masses (`ctx.ic`; dd_generation_root §3.8). It is the `energy_drift` field's view, a drift view, which
// R-400 keeps on R-381's view (render contract Part 5; R-379, R-381; TASK-M3-05): the drift on its ramp by its
// compaction, `COMPACTION` 0 `symlog`, the default, at `dbg_sentinel`'s place of `sign(x)·ln(1 + |x|/eps_E)`, 1 `lin`,
// `range_norm` over the fixed, clamped `[−LIN_R, LIN_R]`, and 2 `log`, which shows `|drift|`, the magnitude, at
// `dbg_log`'s place, unsigned; `symlog` and `lin` on the `diverging(c−, c0, c+)` ramp, or viridis under the palette
// swap, `PALETTE` 1, and `log` on viridis always; a failed sample (`sim_failed`, `decode_failed`) hatched, its
// reference and state being failed-state values (payload §1), and a non-finite reference or drift hatched, tested on
// its bits first (R-436); under `symlog` a drift within the neutral band, the rounding's own scale `15·2⁻²³·(K + |V|)`, is drawn as 0
// (`dbg_drift_shown`, R-652, R-928). Over it the suspect styling of the drift suspect predicates read from the stored
// latches: rows for `SUSPECT_ENERGY`, columns for `SUSPECT_LZ`. `LIN_R`'s and `EPS_E`'s defaults are the constants
// register's `drift_lin_r_E` (REQ-TOOL-150) and `eps_e`, as the generated `energy_drift` view's are.
// @name energy_drift
// @group Debug · derived
// @about The current energy drift H(r, p) − E_0, with the suspect styling.
// @maps energy drift [diverging]
// @glyph name
// @uniform COMPACTION: u32 = 0 [0, 2]
// @control COMPACTION choice("symlog", "lin", "log") "Compaction"
// @uniform PALETTE: u32 = 0 [0, 1]
// @control PALETTE choice("diverging", "viridis") "Palette"
// @uniform LIN_R: f32 = 0.0625
// @control LIN_R fields "lin range"
// @uniform EPS_E: f32 = 1e-6
// @control EPS_E fields "ε_E"

// The value the view shows: the current energy drift.
fn value(ctx: Ctx) -> f32 {
    let masses = vec3<f32>(ctx.ic.m0, ctx.ic.m1, ctx.ic.m2);
    return hamiltonian(ctx.sample.r, ctx.sample.p, masses) - ctx.sample.E_0;
}

fn colour(ctx: Ctx) -> vec3<f32> {
    if (sd_is_failed(ctx.sample.state)) { return debug_invalid(ctx.frag_xy); }
    if (dbg_drift_non_finite(ctx.sample.E_0)) { return debug_invalid(ctx.frag_xy); }
    let x = value(ctx);
    if (dbg_drift_non_finite(x)) { return debug_invalid(ctx.frag_xy); }
    let band = dbg_drift_band_energy(ctx.sample.r, ctx.sample.p, vec3<f32>(ctx.ic.m0, ctx.ic.m1, ctx.ic.m2));
    let shown = dbg_drift_shown(x, uniforms.COMPACTION, band);
    let rgb = dbg_drift(shown, uniforms.COMPACTION, uniforms.PALETTE == 1u, uniforms.LIN_R, uniforms.EPS_E, ctx.frag_xy);
    // The energy predicate reads the regularised drift's latch (R-605), as the generated drift views do.
    let energy = suspect_energy(ctx.sample.dE_reg_max, ctx.sample.E_0, uniforms.EPS_E);
    let lz = suspect_lz(ctx.sample.dLz_max);
    return dbg_suspect(rgb, x, energy, lz, ctx.frag_xy);
}
