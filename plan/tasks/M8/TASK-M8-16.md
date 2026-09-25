# TASK-M8-16 — Inspector pane 3, the one timeline, and the readouts

- **Milestone:** M8
- **Closes:** REQ-GUI-055, REQ-GUI-066, REQ-GUI-067, REQ-GUI-115, REQ-GUI-116, REQ-GUI-117, REQ-GUI-119, REQ-GUI-145, REQ-VAL-108
- **Depends on:** TASK-M8-14, TASK-M8-15, TASK-M4-15
- **Needs (earlier milestones):** REQ-GUI-008, REQ-TOOL-040, REQ-TOOL-041, REQ-SYS-017, REQ-SYS-016, REQ-PAY-074
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-9
- **Size:** ~500 lines

## Goal
Pane 3 shows the trajectory in real space (CoM frame) as CPU-f64 visualisation with no comparison claimed. One ViewUI `t_cursor`, independent of the survey's playhead, scrubs all three panes, and hovering a sphere point jumps real space to it. Editing the IC re-integrates live through computeIC only. The readout row shows latent z, α, β, masses, E · L_z, outcome, F₂ word, round-trip error and gauge error with plots of energy error and pair separations; the scalar readout shows t_end_step, state, escaper, d_min and the drifts at the cursor; the measurement panel shows the ic_inspector list, and a live conditioning number flags ρ → 0 and R → 0. The frame each readout is shown in is defined in the docs so no readout moves under scale-all, and every gauge-invariant readout holds under the four handles.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "05 Inspector — one IC, its trajectory, one timeline"
- `docs/design/principia_trajectory_viewing.md` § "4. Click inspector — the three-panel absolute surface"
- `docs/notes/ic_inspector_scratchpad.md` § "Measurement readouts (gauge-invariant → hold under all four handles)"
- `docs/notes/ic_inspector_scratchpad.md` § "Degeneracy routing"
- `docs/notes/ic_inspector_scratchpad.md` § "Status — as built (supersedes stale details below)"
- `docs/gui/principia_render_gui_spec.md` § "G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)"
- `docs/design/principia_trajectory_viewing.md` § "1. The single mechanism"
- `docs/design/principia_trajectory_viewing.md` § "6. Placement"
- `docs/notes/ic_inspector_scratchpad.md` § "Whole-system handles — the invariance audit"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `crates/gui/src/windows/inspector/{pane_real,timeline,readouts,measurements}.rs`.
- `crates/engine/src/inspector/readouts.rs` — the readout values and the conditioning number.
- Doc changes: `docs/notes/ic_inspector_scratchpad.md` § "Measurement readouts …" and `docs/gui/principia_render_gui_spec.md` §G8 (the frame per readout).
- Tests: `t_cursor_shared`, `edit_calls_compute_ic_once`, `conditioning_blowup`, `readout_invariance` (proptest); screenshot cases `05_inspectors/pane3`, `05_inspectors/readouts`, `05_inspectors/measurements`, `05_inspectors/scalar_readout`.

## Acceptance tests
- `cargo xtask screenshot 05_inspectors` (scalar readout) — screenshot against 05_inspectors.png's readouts (REQ-GUI-055).
- `cargo xtask screenshot 05_inspectors` (measurement panel) — screenshot of the readout panel against 05_inspectors.png (REQ-GUI-066).
- `cargo test -p engine conditioning_blowup` — the readout rises without bound as ρ → 0 and as R → 0 and is finite elsewhere; pole configs stay finite and SAT-flagged (REQ-GUI-067).
- `cargo xtask screenshot 05_inspectors` (pane 3) — screenshot against 05_inspectors.png (REQ-GUI-115).
- `cargo test -p gui t_cursor_shared` — setting t_cursor updates all panes; the global playhead is unchanged; t_cursor is a ViewUI field (REQ-GUI-116).
- `cargo test -p gui edit_calls_compute_ic_once` — a body drag triggers one computeIC call and the trajectory panes update; no other engine entry point is called (REQ-GUI-117).
- `cargo xtask screenshot 05_inspectors` (readout row) — screenshot against 05_inspectors.png's readout row (REQ-GUI-119).
- Doc review of `docs/notes/ic_inspector_scratchpad.md` § "Measurement readouts (gauge-invariant → hold under all four handles)" and §G8 — the docs name the frame per readout; the reviewer checks every listed readout is invariant under all four handles in that frame; the physics reviewer approves the doc change before merge (REQ-GUI-145).
- `cargo test -p engine readout_invariance` — random ICs: translate-, boost- and rotate-all leave every readout unchanged to round-off; scale-all leaves angles, α, β and z unchanged (REQ-VAL-108).

## Notes
- The conditioning number's formula is not given by ic_inspector_scratchpad § "Degeneracy routing" (raised as a gap for a REVIEW_QUEUE entry); REQ-GUI-067 and TASK-M8-17's routing both need it.
- Definitions (R-72) written here: REQ-GUI-145. Each doc change carries the porting rule's "Removed lines" note and the physics reviewer's approval.
