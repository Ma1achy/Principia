# TASK-M8-129 — The same-frame slips D19 and D23, fixed in the GUI's adapters onto the chart-path leaves (R-894, R-916)

- **Milestone:** M8
- **Closes:** REQ-GUI-431
- **Depends on:** TASK-M2-64
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~600 lines, the tests included

## Goal
The same-frame slips D19 and D23, fixed in the GUI's adapters onto TASK-M2-64's leaves (R-894, R-919 F1 (a)): a frame's keyboard tilt and wheel zoom both land, and a drag folds on the current state.

## References
- `decisions.md` § "R-894 — Right after stage 1 and before stage 2, a consolidation step collapses the chart-path leaves to one per operation: bitwise-equal pairs merged with no bit change, rounding-only differences resolved to one canonical formula with a documented bit change and a conversion test, the GUI routes thin input adapters, the same-frame slips fixed; bit-for-bit reproduction is stage 1's safety net, not a permanent rule; the dispatch classes sit in the GUI's `dispatch.rs`, the path in `chart_space` *(the report's D-43, which stands; refines R-863 and R-864)*"
- `decisions.md` § "R-864 — The chart-path work is one object: core types designed first as one module, a type sketch with a one-page rationale reviewed by physics and code and brought to the owner before any implementation; today's operations re-expressed as named families of it and their old routes deleted; bitwise reproduction first; special cases are named families, never if-branches; the kernel/engine split stays; restructuring `engine/nav`, `engine/lock.rs` and `engine/chart_authoring` is approved *(the architecture of PR #422's chart-path work; refines no ruling on `main`)*"
- `decisions.md` § "R-863 — When the chart-composition design's tasks are released, they are built in five stages, each merging only with the previous stage's tests green: the path object with today's operations bitwise through it; locked paths; the primitive registry and composed axes; morph families, easing and Sequence segments; time-axis charts and the symmetry filter *(the build order of PR #422's tasks; refines no ruling)*"
- `decisions.md` § "R-474 — RQ-340 is ruled: two tilt maps, "Tip as sheet", the nav cube's axis-angle map and the default, and "Lean each axis", today's, an option, differing only with one target and both angles set; Tip as sheet carries on over the pole, wrapped only in total tip at 360° and never folded onto its 180° partner; planes are compared up to sign and never written back; the map is recorded everywhere as `tilt_map`; the compass draws the active map, B11 revisited; Lean's Gram factor and warnings; every readout shows each axis's components *(closes RQ-340; amends R-451, R-457 and R-459)*"
- `docs/design/principia_chart_composition.md` § "15. Plan"
- `docs/design/principia_chart_space_model.md` § "3. The existing operations are paths of named families"

## Deliverables
- D19 (a frame's keyboard tilt and wheel zoom both land) and D23 (a drag folds on the current state) fixed in `crates/gui`'s adapters, holding no formula.

## Acceptance tests
- `cargo test -p gui --test chart_path_consolidate -- path_frame_batch_running path_drag_running` — a frame's keyboard tilt and wheel zoom both land; a drag's fold on the current state; controls: the second edit of a frame dropped fails, a drag folded on the press state fails (REQ-GUI-431).

## Notes
- Added by R-894 (8 Nov 2026): between stages 1 and 2 of R-863's order; TASK-M2-65 (stage 2) depends on it.
- A difference that isn't rounding is not merged and not patched around: stop and bring it back (R-864 item 4).
- *Round 2 of PR #455 (physics review 5820482634, code review 5820497276):* P11: R-544's mirror-test precision rule carried in; a rounding difference near a landmark is not brought back as a real one. *Was (acceptance, the clause):* "decoding to the same system within REQ-ENC-024's `ε_phys` on the decoded state, its outcome class unchanged;".
- *9 Nov 2026 (R-916):* the chart core is M2's (R-916, R-917, R-919), so this header follows plan/tasks.yaml. *Was:* "Closes: REQ-CHART-156"; "Depends on: TASK-M2-58, TASK-M2-61, TASK-M8-125".
- *9 Nov 2026 (R-916):* split: the consolidation, one leaf per operation, and the GUI routes as thin adapters (REQ-CHART-156) are TASK-M2-64's (R-919, F1 (a)); this task keeps D19 and D23 (REQ-GUI-431). *Was (Goal):* the R-894 consolidation; *was (Deliverables, Acceptance):* the leaves, the canonical formula, the adapters and their tests, now at TASK-M2-64.
