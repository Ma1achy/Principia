# TASK-M8-17 — Inspector: Create & locate, degeneracy routing, and the honest CPU / GPU comparison

- **Milestone:** M8
- **Closes:** REQ-GUI-120, REQ-ENC-021, REQ-ENC-028, REQ-VAL-106, REQ-VAL-107, REQ-TOOL-097
- **Depends on:** TASK-M8-14, TASK-M8-16, TASK-M8-07, TASK-M4-13
- **Needs (earlier milestones):** REQ-ENC-008, REQ-VAL-073, REQ-VAL-017, REQ-SYS-026, REQ-TOOL-040, REQ-TOOL-043, REQ-GUI-004
- **Reviewers:** code, qa, physics, gui, perf
- **Pitfalls:** PIT-10, PIT-1.7
- **Size:** ~500 lines

## Goal
The Create & locate tab builds an IC, shows where it lands in z and its distance from the slice, and offers lock the view on it, centre a slice through it, mark it in the figure, and keep it. Past a calibrated conditioning threshold the Inspector switches from chart-encode to direct-physical-inject and flags it; for non-degenerate configs it runs both paths and reports their difference. The optional shape-divergence overlay (off by default) dispatches a one-pixel f32 GPU trace of the clicked IC through the survey kernel and draws its dense n(t) over the CPU curve; CPU and GPU outcome class, escaper and t_end are shown side by side, and disagreement in a regular region is a pipeline bug. The footnote uses trajectory_viewing §5's honest wording.

## References
- `docs/gui/principia_render_gui_spec.md` § "G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "05 Inspector — one IC, its trajectory, one timeline"
- `docs/notes/ic_inspector_scratchpad.md` § "Bridges to the main view"
- `docs/notes/ic_inspector_scratchpad.md` § "Degeneracy routing"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_trajectory_viewing.md` § "5. What is honestly comparable (click inspector)"

## Deliverables
- `crates/gui/src/windows/inspector/create_locate.rs` — the four actions as SetFields (lock fields, z₀) and ViewUI edits (mark, keep).
- `crates/engine/src/inspector/routing.rs` — conditioning threshold, direct-inject path, two-path diff.
- `crates/engine/src/inspector/gpu_trace.rs` — the on-demand one-pixel f32 trace job writing n(t) (a sanctioned pull, one in flight).
- `crates/gui/src/windows/inspector/comparison.rs` — side-by-side classification and the footnote text.
- Calibration proposal: κ over near-degenerate and random ICs, the threshold and the two-path agreement tolerance.

## Acceptance tests
- `cargo test -p gui create_locate_actions` — build an IC: 'mark' draws a crosshair at z's projection on the current slice; 'lock' sets the lock fields to that z; 'centre' sets z₀; 'keep' adds a kept orbit (REQ-GUI-120).
- `cargo test -p engine degeneracy_routing` (threshold and tolerance: REQ-ENC-028, calibrated) — a near-degenerate IC flips to direct inject with a flag; random non-degenerate ICs show agreeing paths (REQ-ENC-021).
- `cargo xtask gate inspector-routing` — the proposal measures κ and the chart-encode vs direct-inject difference over near-degenerate and random ICs and states both values; recorded in decisions.md; a reviewer checks the proposal and the human confirms the value at the M8 gate, then it is recorded in `decisions.md` (REQ-ENC-028).
- `cargo test -p engine shape_divergence_overlay` — overlay off by default; enabling it dispatches one job; for a periodic IC the two curves coincide to f32 tolerance (REQ-VAL-106).
- `cargo xtask gate inspector-classification-regular` — on a regular-region fixture of ICs, CPU and GPU classification agree for every IC (REQ-VAL-107).
- Review checklist (physics reviewer) — the shipped footnote text matches trajectory_viewing §5's wording (REQ-TOOL-097).

## Notes
- The on-demand GPU trace needs a dense n(t) output buffer outside the generation-root ledger; its shape is for the implementer to propose within trajectory_viewing §5 ("one trajectory's history is trivial memory") and the physics reviewer to check.
- Calibrations (R-71) proposed here: REQ-ENC-028. Each value is confirmed by the human at the M8 gate; an unconfirmed one blocks the gate.
