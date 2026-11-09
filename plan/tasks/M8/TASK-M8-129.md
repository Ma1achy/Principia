# TASK-M8-129 — Consolidating the chart path: one leaf per operation, the GUI routes as input adapters, D19 and D23 fixed (R-894)

- **Milestone:** M8
- **Closes:** REQ-CHART-156
- **Depends on:** TASK-M4-37, TASK-M8-124, TASK-M8-125
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~600 lines, the tests included

## Goal
R-894: right after stage 1 and before stage 2, the chart-path leaves collapse to one per operation: one pan, one zoom,
one tilt per R-474 map (each in its `X = Y` and `X ≠ Y` forms), one slice, one lock, each cut kind once. Bit-for-bit
reproduction is stage 1's safety net, not a permanent rule to keep per-route arithmetic (R-864 as R-894 refines it).

## References
- `decisions.md` § "R-894 — Right after stage 1 and before stage 2, a consolidation step collapses the chart-path leaves to one per operation: bitwise-equal pairs merged with no bit change, rounding-only differences resolved to one canonical formula with a documented bit change and a conversion test, the GUI routes thin input adapters, the same-frame slips fixed; bit-for-bit reproduction is stage 1's safety net, not a permanent rule; the dispatch classes sit in the GUI's `dispatch.rs`, the path in `chart_space` *(the report's D-43, which stands; refines R-863 and R-864)*"
- `decisions.md` § "R-864 — The chart-path work is one object: core types designed first as one module, a type sketch with a one-page rationale reviewed by physics and code and brought to the owner before any implementation; today's operations re-expressed as named families of it and their old routes deleted; bitwise reproduction first; special cases are named families, never if-branches; the kernel/engine split stays; restructuring `engine/nav`, `engine/lock.rs` and `engine/chart_authoring` is approved *(the architecture of PR #422's chart-path work; refines no ruling on `main`)*"
- `decisions.md` § "R-863 — When the chart-composition design's tasks are released, they are built in five stages, each merging only with the previous stage's tests green: the path object with today's operations bitwise through it; locked paths; the primitive registry and composed axes; morph families, easing and Sequence segments; time-axis charts and the symmetry filter *(the build order of PR #422's tasks; refines no ruling)*"
- `decisions.md` § "R-474 — RQ-340 is ruled: two tilt maps, "Tip as sheet", the nav cube's axis-angle map and the default, and "Lean each axis", today's, an option, differing only with one target and both angles set; Tip as sheet carries on over the pole, wrapped only in total tip at 360° and never folded onto its 180° partner; planes are compared up to sign and never written back; the map is recorded everywhere as `tilt_map`; the compass draws the active map, B11 revisited; Lean's Gram factor and warnings; every readout shows each axis's components *(closes RQ-340; amends R-451, R-457 and R-459)*"
- `docs/design/principia_chart_composition.md` § "15. Plan"
- `docs/design/principia_chart_space_model.md` § "3. The existing operations are paths of named families"

## Deliverables
- One leaf per operation in `crates/engine`'s chart-path module; a pair measuring bitwise-equal under D2's criterion merged
  with no bit change.
- Routes differing only in rounding resolved to one canonical formula, the bit change deliberate and documented, with a
  conversion test; a difference that isn't rounding not merged but brought back to the owner.
- The GUI routes (keys, panel, compass step, compass drag) as thin input adapters feeding the same leaf, holding no
  formula.
- The same-frame slips D19 (a frame's keyboard tilt and wheel zoom both land) and D23 (a drag folds on the current state)
  fixed.
- The dispatch classes stay in `crates/gui/src/dispatch.rs`, the GUI's app layer, and the path in `chart_space` (R-894,
  the owner's placement as standing).

## Acceptance tests
- `cargo test -p engine --test chart_path_consolidate -- path_merge_bitwise path_canonical_conversion path_pin_kept path_control_`
  — merged pairs bitwise; each changed case listed old against new with its largest difference in ulps, within the count derived from the two formulas, and, on the cases whose `n̂` lies farther than a stated `δ` from each binary-collision landmark (`ulp/δ ≤ ε_phys`; R-544's mirror-test precision rule), decoding to the same system within REQ-ENC-024's `ε_phys`, its outcome class unchanged, `δ` and the excluded set stated; controls: a reordered operation fails (REQ-CHART-156).
- `cargo test -p gui --test chart_path_consolidate -- path_merge_bitwise path_canonical_conversion path_adapter_same_leaf path_frame_batch_running path_drag_running path_pin_kept path_control_`
  — each adapter's output equals its leaf's constructor's bitwise; a frame's keyboard tilt and wheel zoom both land (D19);
  a drag's fold on the current state (D23); controls: an adapter holding a formula fails (REQ-CHART-156).
- Review: the merged-away leaf names gone by a word-bounded `git grep`, and each documented bit change approved by the
  physics reviewer (REQ-CHART-156).

## Notes
- Added by R-894 (8 Nov 2026): between stages 1 and 2 of R-863's order; TASK-M8-117 (stage 2) depends on it.
- A difference that isn't rounding is not merged and not patched around: stop and bring it back (R-864 item 4).
- *Round 2 of PR #455 (physics review 5820482634, code review 5820497276):* P11: R-544's mirror-test precision rule carried in; a rounding difference near a landmark is not brought back as a real one. *Was (acceptance, the clause):* "decoding to the same system within REQ-ENC-024's `ε_phys` on the decoded state, its outcome class unchanged;".
