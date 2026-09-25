# TASK-M7-01 — Composition-spec gaps: the resolutions the stain editor assumes, written into colour_composition

- **Milestone:** M7
- **Closes:** REQ-COL-054
- **Depends on:** TASK-M1-13
- **Needs (earlier milestones):** REQ-RENDER-075, REQ-COL-003, REQ-RENDER-024
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~150 lines (docs)

## Goal
render_gui_spec §16 lists eight "standing composition-spec gaps" that the stain editor "assumes": colour-source-as-axis, overlays-as-post-chain-over-configured-base, physics-as-overlay-op, gradient-unifies-the-ramp, per-footprint vs quad-aggregate spread, the debug taxonomy, the addressing channel convention (UV=RG / TL=RB / centre=BG / index=grey) and boundaries-as-overlay. After this PR `principia_colour_composition.md` states each resolution (R-72), so the graph model (TASK-M7-11), the preset library (TASK-M7-17) and the map presets (TASK-M7-18) build against written definitions, and §16's item is closed.

## References
- `docs/gui/principia_render_gui_spec.md` § "16. Open / next"
- `docs/design/principia_colour_composition.md` § "8. Amendments to other docs"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_colour_composition.md` § "1.3 Combinators  →  `vec3`"
- `docs/design/principia_colour_composition.md` § "3. The `ctx` contract"
- `docs/design/principia_colour_composition.md` § "4.2 Post chain"
- `docs/design/principia_colour_composition.md` § "6. Debug views as presets"
- `docs/gui/principia_render_gui_spec.md` § "5. Sources & multi-source"
- `docs/gui/principia_render_gui_spec.md` § "11. Presets = whole graphs"
- `docs/gui/principia_render_gui_spec.md` § "12.1 Structural overlays and tile debug shaders"
- `docs/notes/principia_sampling_msaa_note.md` § "Stability metrics: FTLE and diffusion are SimState fields; spread is a resolve-stage reduction"

## Deliverables
- `docs/design/principia_colour_composition.md` — a new subsection under §8 (or a new numbered section after it) giving one definition per §16 item, each written from the passages that already carry it: source nodes and ctx lanes (render_gui_spec §3.1, §5; colour_composition §3); the post chain over a configured base (§1.3, §4.2); `site_overlay` as the physics op (§1.3, §7); `FieldRamp{gradient_magnitude}` vs the map post node (§1.2, render_contract Part 4); footprint spread at the resolve vs `ctx.quad.spread` from `QuadReduction` (sampling note); render_gui_spec §11's debug groups (addressing / quad-structure / decoded-IC / validity / diagnostic / profiling); §16's channel convention; boundaries as a Tier-1 post node (§12.1).
- `docs/gui/principia_render_gui_spec.md` §16 — the "Standing composition-spec gaps" bullet marked closed with a pointer to the new colour_composition section (struck through, not deleted).
- Any item the cited passages do not settle is raised as a REVIEW_QUEUE entry, not written in.
- Commit message ends with the "Removed lines" note (porting rule).

## Acceptance tests
- Review (physics): `principia_colour_composition.md` states each of the eight resolutions render_gui_spec §16 says the GUI spec assumes, each traceable to a cited passage, and §16's item is closed (REQ-COL-054).
- `cargo xtask plan-check` — every requirement source and task reference still resolves after the doc edit (REQ-COL-054).

## Notes
- Definition requirement (R-72): the deliverable is the doc change; the physics reviewer approves before merge.
- Ports add, never remove: §16's bullet is struck through with its pointer, not deleted.
- Runs first because TASK-M7-11, -17 and -18 read these definitions.
