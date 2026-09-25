# TASK-M8-36 — Research (v2): continuation, Poincaré return map, side by side with linked views (11_research.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-123, REQ-GUI-124, REQ-GUI-125, REQ-GUI-154
- **Depends on:** TASK-M8-35, TASK-M8-11
- **Needs (earlier milestones):** REQ-SYS-017, REQ-SYS-011, REQ-GUI-002, REQ-CHART-032
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
The Research window adds continuation along a parameter (e.g. a mass ratio) with a step, marking folds where stability changes; a Poincaré return map on a chosen section for a kept orbit; and a side-by-side view with a linked cursor, linked navigation and a difference view. The linked views are a ViewUI item distinct from the chart's link ids in SimConfig (R-106), defined in §G11 and gui_state_contract §2, none of it on the sim key.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "11 Research — first pass (v2)"
- `docs/gui/principia_render_gui_spec.md` § "G11. Research — first pass (v2) (`11_research.png`)"
- `decisions.md` § "R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*"
- `decisions.md` § "R-106 — The link ids are the chart's link functions *(closes RQ-66)*"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `crates/validation/src/research/{continuation,poincare}.rs`.
- `crates/gui/src/windows/research/{continuation,poincare,side_by_side}.rs`; linked-view state in ViewUI.
- Doc changes: `docs/gui/principia_render_gui_spec.md` §G11 and `docs/contracts/principia_gui_state_contract.md` §2 (what a linked view holds; how cursor and navigation are shared).
- Screenshot cases `11_research/continuation`, `11_research/poincare`, `11_research/side_by_side`; test `linked_pan`.

## Acceptance tests
- `cargo xtask screenshot 11_research` (continuation) — screenshot against 11_research.png (REQ-GUI-123).
- `cargo xtask screenshot 11_research` (Poincaré map) — screenshot against 11_research.png (REQ-GUI-124).
- `cargo xtask screenshot 11_research` (side by side) and `cargo test -p gui linked_pan` — screenshot against 11_research.png; a pan in one side pans the other (REQ-GUI-125).
- Doc review of `docs/gui/principia_render_gui_spec.md` § "G11. Research — first pass (v2) (`11_research.png`)" and `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface" — §G11 and gui_state §2 define the linked views as ViewUI state, none of it on the sim key; the chart's link ids are unchanged; the physics reviewer approves the doc change before merge (REQ-GUI-154).

## Notes
- The stability measure that marks a fold, the section choices for the return map, and what the difference view differences are not given (raised as a gap for a REVIEW_QUEUE entry).
- Definitions (R-72) written here: REQ-GUI-154. Each doc change carries the porting rule's "Removed lines" note and the physics reviewer's approval.
