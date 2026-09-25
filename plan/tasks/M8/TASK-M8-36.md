# TASK-M8-36 — Research (v2): continuation, Poincaré return map, side by side with linked views (11_research.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-123, REQ-GUI-124, REQ-GUI-125, REQ-GUI-154, REQ-GUI-159
- **Depends on:** TASK-M8-35, TASK-M8-11
- **Needs (earlier milestones):** REQ-SYS-017, REQ-SYS-011, REQ-GUI-002, REQ-CHART-032
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
The Research window adds continuation along a parameter (e.g. a mass ratio) with a step, with periodic-orbit stability read from the monodromy matrix's Floquet multipliers — a fold where a multiplier crosses +1, a period-doubling where one crosses −1 (R-127); a Poincaré return map for a kept orbit on one of three sections: syzygy crossings (w = 0), a chosen shape-sphere great circle, or a Jacobi-coordinate hyperplane (R-127); and a side-by-side view with a linked cursor, linked navigation and a difference view. The linked views are a ViewUI item distinct from the chart's link ids in SimConfig (R-106), defined in §G11 and gui_state_contract §2, none of it on the sim key.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "11 Research — first pass (v2)"
- `docs/gui/principia_render_gui_spec.md` § "G11. Research — first pass (v2) (`11_research.png`)"
- `decisions.md` § "R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*"
- `decisions.md` § "R-106 — The link ids are the chart's link functions *(closes RQ-66)*"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

- `decisions.md` § "R-127 — Periodic-orbit stability is Floquet; the Poincaré sections are three *(closes RQ-109)*"
## Deliverables
- `crates/validation/src/research/{continuation,poincare}.rs` — the monodromy matrix and its Floquet multipliers in f64; the three sections.
- Doc change: `docs/gui/principia_render_gui_spec.md` §G11 — the monodromy/Floquet stability measure and the three section definitions transcribed with citations (R-127), physics-reviewed, confirmed at the M8 gate, with the "Removed lines" note.
- `crates/gui/src/windows/research/{continuation,poincare,side_by_side}.rs`; linked-view state in ViewUI.
- Doc changes: `docs/gui/principia_render_gui_spec.md` §G11 and `docs/contracts/principia_gui_state_contract.md` §2 (what a linked view holds; how cursor and navigation are shared).
- Screenshot cases `11_research/continuation`, `11_research/poincare`, `11_research/side_by_side`; test `linked_pan`.

## Acceptance tests
- `cargo xtask screenshot 11_research` (continuation) — screenshot against 11_research.png; `cargo test -p validation continuation_floquet` — on a continuation fixture the fold and period-doubling markers sit where a Floquet multiplier crosses +1 / −1 (REQ-GUI-123).
- `cargo xtask screenshot 11_research` (Poincaré map) — screenshot against 11_research.png; the section picker offers exactly the three sections (REQ-GUI-124).
- Review (physics): §G11's stability measure and section definitions carry their citations; confirmed by the human at the M8 gate (R-127).
- `cargo xtask screenshot 11_research` (side by side) and `cargo test -p gui linked_pan` — screenshot against 11_research.png; a pan in one side pans the other (REQ-GUI-125).
- Doc review of `docs/gui/principia_render_gui_spec.md` § "G11. Research — first pass (v2) (`11_research.png`)" and `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface" — §G11 and gui_state §2 define the linked views as ViewUI state, none of it on the sim key; the chart's link ids are unchanged; the physics reviewer approves the doc change before merge (REQ-GUI-154).
- Definition: the difference view written into render_gui_spec §G11 and approved by the physics reviewer (REQ-GUI-159).

## Notes
- What the difference view differences is not given (REQ-GUI-159 defines it).
- Definitions (R-72) written here: REQ-GUI-154. Each doc change carries the porting rule's "Removed lines" note and the physics reviewer's approval.
- RQ-109 ruled: R-127 — stability by the monodromy matrix's Floquet multipliers; the three Poincaré sections; transcribed with citations, physics-reviewed, confirmed at the gate (render_gui_spec §G11 conformed in step 7; the citations are this task's).
- Closes, for gaps the corpus leaves open: REQ-GUI-159 (R-72 definition) (classification accepted by R-132).
