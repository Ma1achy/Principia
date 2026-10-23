# TASK-M7-38 — The Chazy energy-channel stains: seven classes × energy, structure × energy, and B's hue

- **Milestone:** M7
- **Closes:** REQ-COL-074, REQ-COL-078, REQ-COL-079, REQ-COL-088, REQ-COL-089
- **Depends on:** TASK-M7-37, TASK-M7-04, TASK-M7-05
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** PIT-5
- **Size:** ~300 lines (two stains as data, the compaction and range ~80, tests ~150, goldens are fixtures)

## Goal
The stains that put `chazy_q` in the brightness slot under Replace-L's range form, so lightness carries the energy of
the separating motion (R-442; the Chazy drill-down §10.5): `L = 0.3 + 0.6·b`, `b = q/(1 + q)`, dark bound, mid
parabolic, light hyperbolic. Seven classes × energy recovers what the flat map folds into B: a transient sample renders
light grey and a bound one dark grey, because B has no hue. Structure × energy reads which bodies separate (hue) and
how hard (lightness). The lightness range and the compaction are proposed for the M7 gate (R-71).

## References
- `docs/design/principia_dd_chazy_final_motion.md` § "5.5 `chazy_structure` and `chazy_q`"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.1 The two rules that hold in every stain"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.5 With an energy channel"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.5.1 Seven classes × energy"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.5.2 Structure × energy"
- `docs/design/principia_dd_chazy_final_motion.md` § "11.2 Calibrations to confirm at a gate (R-71)"
- `docs/design/principia_dd_chazy_final_motion.md` § "12.2 New, for Principia"
- `docs/design/principia_dd_colouring.md` § "3.5 Combiners"
- `docs/design/principia_dd_colouring.md` § "3.6 Compaction (payload scalar → b ∈ [0,1]; forms per ledger `scale`)"
- `decisions.md` § "R-442 — Chazy final motion is a separate system with its own stains: hue carries the class, B is black `#141418` and one class, collision is magenta `#DA85B6`, transient draws as B, the working states have their own status map, and the seven-class palette is the default"
- `decisions.md` § "R-548 — The Chazy entries ruled: the seven-class palette stays the default; the subtitle ban covers captions over the figure only; the class freezes at escape fire and may differ from the escape event, the legend saying so; the class is a label, reported, not asserted; OS-candidate and B-stable ship marked provisional; B `#141418` and the grey `#8A8A90` are near-neutral, held to chroma ≤ 0.009; the rule keeps transient before B-stable, puts B-stable before excursion and lets H, HP, P and the two-stage H fire only at E ≥ 0; a render-side previous-class texture; the one-body views read `chazy_escaper`; the three panels are one figure *(closes RQ-311 to RQ-318 and RQ-320 to RQ-325)*"

## Deliverables
- `chazy_q`'s display compaction `q/(1 + q)`, range-free.
- The seven-class × energy and structure × energy stains and presets, Replace-L at `L_min = 0.3`, `L_max = 0.9`.
- B's hue check, and the calibration proposals for the lightness range and the compaction.

## Acceptance tests
- `cargo test -p render chazy_b_no_hue` — B's OKLab chroma is ≤ 0.009 (0.0080 measured), and B under Replace-L at L = 0.3, 0.6 and 0.9 gives a near-neutral grey within that bound (R-548, RQ-317); control: `#14BBC2`, its chroma well above the bound, fails the same check (REQ-COL-074).
- `cargo test -p render chazy_energy_recovers_transient` — the transient fixture's sample has OKLab L above L(b = ½) and the same configuration at rest below it; with `chazy_q` under REQ-PAY-104's definition, the drill-down's §5.5 overlap states do the same (a transient whose outer body is at ρ = 10, receding, with outer `K/U = 0.5`, above; a B-so-far whose outer body approaches with outer `K/U > 1`, below); control: the flat seven-class stain draws both identically (REQ-COL-078).
- `cargo xtask golden chazy-energy` — a synthetic payload with a sample per populated §10.5.2 cell renders its hue at the cell's lightness band; dispersal never renders dark (REQ-COL-079).
- Review (gui): the lightness range L 0.3 to 0.9, with renders showing dark regions keep their hue at L_min and lose it near 0, confirmed by the human at the M7 gate (REQ-COL-088).
- Review (physics): the compaction `q/(1 + q)`, range-free (dd_colouring §3.6, R-411) and parabolic at ½, confirmed by the human at the M7 gate (REQ-COL-089).

## Notes
- `#141418` has OKLab chroma ≈ 0.008, not 0; R-548 keeps it, near-neutral, held to chroma ≤ 0.009 (RQ-317, option
  (a)), so REQ-COL-074 no longer waits. *Was:* "(RQ-317); REQ-COL-074 waits on the ruling for its bound. The other
  four requirements do not wait."
