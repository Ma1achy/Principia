// The derived current-drift view (debug_tooling_plan §B, "DERIVED views"; payload §5): `ΔE = H(r, p) − E_0`, computed
// in the fragment from the live f32 state through the read side's own `hamiltonian` (TASK-M1-01), with the sample's
// `ICDescriptor` masses (`ctx.ic`; dd_generation_root §3.8). It is the `energy_drift` field's view, a drift view, which
// R-400 keeps on R-381's view (render contract Part 5; R-379, R-381; TASK-M3-05): the drift on its ramp by its
// compaction, `COMPACTION` 0 `symlog`, the default, at `dbg_sentinel`'s place of `sign(x)·ln(1 + |x|/eps_E)`, 1 `lin`,
// `range_norm` over the fixed, clamped `[−LIN_R, LIN_R]`, and 2 `log`, which shows `|drift|`, the magnitude, at
// `dbg_log`'s place, unsigned; `symlog` and `lin` on the `diverging(c−, c0, c+)` ramp, or viridis under the palette
// swap, `PALETTE` 1, and `log` on viridis always; the absence NaN hatched; under `symlog` a drift within the neutral
// band of `E_0` is drawn as 0 (`dbg_drift_shown`, R-652). Over it the suspect styling of the drift
// suspect predicates read from the stored latches: rows for `SUSPECT_ENERGY`, columns for `SUSPECT_LZ`. `LIN_R`'s and
// `EPS_E`'s defaults are the constants register's `drift_lin_r_E` (REQ-TOOL-150) and `eps_E_default`, as the generated
// `energy_drift` view's are.
// @uniform COMPACTION: u32 = 0 [0, 2]
// @uniform PALETTE: u32 = 0 [0, 1]
// @uniform LIN_R: f32 = 0.0625
// @uniform EPS_E: f32 = 1e-6

// The value the view shows: the current energy drift.
fn value(ctx: Ctx) -> f32 {
    let masses = vec3<f32>(ctx.ic.m0, ctx.ic.m1, ctx.ic.m2);
    return hamiltonian(ctx.sample.r, ctx.sample.p, masses) - ctx.sample.E_0;
}

fn colour(ctx: Ctx) -> vec3<f32> {
    let x = value(ctx);
    let shown = dbg_drift_shown(x, uniforms.COMPACTION, ctx.sample.E_0);
    let rgb = dbg_drift(shown, uniforms.COMPACTION, uniforms.PALETTE == 1u, uniforms.LIN_R, uniforms.EPS_E, ctx.frag_xy);
    let energy = suspect_energy(ctx.sample.dE_max, ctx.sample.E_0, uniforms.EPS_E);
    let lz = suspect_lz(ctx.sample.dLz_max);
    return dbg_suspect(rgb, x, energy, lz, ctx.frag_xy);
}
