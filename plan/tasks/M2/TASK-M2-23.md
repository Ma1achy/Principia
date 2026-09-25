# TASK-M2-23 — The lock: the anchor, the excursion δ, re-based sliders and SimConfig lock fields

- **Milestone:** M2
- **Closes:** REQ-GUI-004, REQ-GUI-005, REQ-GUI-006, REQ-GUI-007, REQ-SYS-012, REQ-CHART-010, REQ-CHART-007
- **Depends on:** TASK-M2-06, TASK-M2-19, TASK-M2-22
- **Needs (earlier milestones):** REQ-GUI-001
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-9
- **Size:** ~480 lines

## Goal
The lock is CPU-side chart construction held in `SimConfig` (R-69): setting it computes z_locked = z₀ + (2s−1)q₁ + (2t−1)q₂ on an affine chart with no readback, and on a nonlinear chart replays Φ and D on the CPU, re-centring through the inverse-encode path; recentring is a `SetField` on z₀. Tilt, zoom and chart-mode switch preserve the anchor exactly; locked slicing and slider moves are an explicit excursion δ from the stored anchor, ghost-marked, with snap-back returning δ → 0 exactly; sliders are re-based, not frozen; unlock keeps the current (z₀, q₁, q₂). The kernel receives only (z₀, q₁, q₂, chart id + params) in either mode. A tilt that carries the neighbourhood outside an invariant chart's feasible region tags those pixels and offers re-centre; curve-axis tilt requires a lock.

## References
- `docs/contracts/principia_chart_decoder_contract.md` § "The lock (projective microscope)"
- `docs/design/principia_dd_encode.md` § "2. Consolidated contract"
- `docs/gui/principia_render_gui_spec.md` § "G4. Lock — the reticle and the pin (`08_lock.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G7. Chart builder (`03_chartbuilder.png`)"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "03 Chart builder"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "08 Lock — the reticle and the pin"
- `decisions.md` § "R-83 — The slice scale lives in q *(closes RQ-34)*"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/contracts/principia_chart_decoder_contract.md` § "What the GPU knows about all of this: nothing"
- `docs/contracts/principia_chart_decoder_contract.md` § "Design axioms (the six that must survive contact with a code agent)"
- `decisions.md` § "R-69 — What is undoable *(closes the step-6 open question)*"
- `docs/contracts/principia_inverse_encode_contract.md` § "Chart-aware validation"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "The four axis kinds — a closed set"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"
- `docs/contracts/principia_chart_decoder_contract.md` § "Directions are axis kinds — the unifying rule"

## Deliverables
- `crates/engine/src/contract/sim_config.rs`: the lock flag, z_locked anchor and δ excursion fields.
- `crates/engine/src/lock.rs` (set / excursion / snap-back / unlock / lock validation), emitting typed `SetField` edits only.
- Tests `crates/engine/tests/lock.rs`; the bitwise locked-vs-unlocked check on the GPU decode stage.

## Acceptance tests
- `cargo test -p engine lock_set` — lock a pixel on an affine and on a nonlinear chart; the centre pixel's IC equals the clicked pixel's IC (REQ-GUI-004).
- `cargo test -p engine lock_anchor_invariance` — property: random tilt / zoom / chart switches leave the centre IC bitwise constant; snap-back restores the stored anchor bitwise (REQ-GUI-005).
- `cargo test -p engine lock_sliders` — moving a slider while locked is an excursion along that control; unlock → the free state equals the last locked state (REQ-GUI-006).
- `cargo test -p engine lock_setfields` — the lock action emits SetFields on `SimConfig`'s lock fields and z₀; none are `ViewUI` (REQ-GUI-007).
- Review (physics) + `cargo test -p engine lock_kernel_uniform_identical` — the kernel uniform struct has no lock field; locked and unlocked renders of the same (z₀, q₁, q₂) through the decode stage are bitwise identical (REQ-SYS-012).
- `cargo test -p engine lock_infeasible_tilt` — tilt a locked (L_z, E) view past the parabola; infeasible pixels are tagged; lock validation reports and offers re-centre (REQ-CHART-010).
- `cargo test -p engine curve_tilt_requires_lock` — tilting a curve axis without a lock is refused; with a lock the tilt uses γ'(ν₀) (REQ-CHART-007).

## Notes
- Gap G16 applies to "re-centred to the nearest feasible point".
- Lock is undoable (R-69) through the contract's history; the GUI binding (K, right-click) lands at M8.
