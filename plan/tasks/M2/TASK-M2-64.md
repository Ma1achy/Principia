# TASK-M2-64 — Consolidating the chart path: one leaf per operation through REQ-CHART-060's stop, the merged GUI routes made thin adapters (R-894, R-916, R-919 F1 (a))

- **Milestone:** M2
- **Closes:** REQ-CHART-156, REQ-CHART-168
- **Depends on:** TASK-M2-61, TASK-M2-62, TASK-M2-63, TASK-M2-38
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~700 lines, the tests included

## Goal
R-894: right after stage 1 and before stage 2, the chart-path leaves collapse to one per operation: one pan, one zoom, one tilt per R-474 map (each in its `X = Y` and `X ≠ Y` forms), one slice, one lock, each cut kind once, every navigation edit passing through REQ-CHART-060's stop before it is recorded. By R-919 (F1 (a)), the merged GUI routes (keys, panel, compass step, compass drag) become thin input adapters onto these leaves here, with no new UI; the same-frame slips D19 and D23 stay TASK-M8-129's.

## References
- `decisions.md` § "R-894 — Right after stage 1 and before stage 2, a consolidation step collapses the chart-path leaves to one per operation: bitwise-equal pairs merged with no bit change, rounding-only differences resolved to one canonical formula with a documented bit change and a conversion test, the GUI routes thin input adapters, the same-frame slips fixed; bit-for-bit reproduction is stage 1's safety net, not a permanent rule; the dispatch classes sit in the GUI's `dispatch.rs`, the path in `chart_space` *(the report's D-43, which stands; refines R-863 and R-864)*"
- `decisions.md` § "R-469 — A chart never leaves the manifold: every pixel it draws is a valid three-body system or explicitly "not a system", and pan, slice, zoom, tilt and lock are accepted only while the chart's centre passes `validate`, an edit that would cross stopping at the last valid value, by one generic rule in the edit path; undo and the contract record the clamped value, the slider stops and the readout names the check"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "The compass: look and views (R-451, R-452)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `decisions.md` § "R-916 — M2 is the whole non-visual chart system: the primitive registry, axes from primitives and warps, charts as a pair of axes with their dependence rules, refusals and labels, curved slices, CPU decode and encode through the composition, and the non-visual path maths, each an engine function with its tests, landing before any further downstream chart work, today's charts and view operations re-expressed in it bit for bit first; GPU lowering, contours and display, GUI routes, panels, the Sequence's UI, time-axis mode's UI and overlays keep their milestones as thin adapters onto it; a task mixing maths and visuals is split *(amends R-863; supersedes R-873 (A) (b) and (E) (i); refines R-864 and R-894)*"
- `decisions.md` § "R-864 — The chart-path work is one object: core types designed first as one module, a type sketch with a one-page rationale reviewed by physics and code and brought to the owner before any implementation; today's operations re-expressed as named families of it and their old routes deleted; bitwise reproduction first; special cases are named families, never if-branches; the kernel/engine split stays; restructuring `engine/nav`, `engine/lock.rs` and `engine/chart_authoring` is approved *(the architecture of PR #422's chart-path work; refines no ruling on `main`)*"
- `decisions.md` § "R-474 — RQ-340 is ruled: two tilt maps, "Tip as sheet", the nav cube's axis-angle map and the default, and "Lean each axis", today's, an option, differing only with one target and both angles set; Tip as sheet carries on over the pole, wrapped only in total tip at 360° and never folded onto its 180° partner; planes are compared up to sign and never written back; the map is recorded everywhere as `tilt_map`; the compass draws the active map, B11 revisited; Lean's Gram factor and warnings; every readout shows each axis's components *(closes RQ-340; amends R-451, R-457 and R-459)*"
- `docs/design/principia_chart_composition.md` § "15. Plan"
- `docs/design/principia_chart_space_model.md` § "3. The existing operations are paths of named families"
- `decisions.md` § "R-919 — The owner's answers to the chart plan: F1 (a), the merged GUI routes made thin adapters in TASK-M2-64; F10, R1 gates bitwise and R2 is undo; F11, G4's reading; F13 (b), `τ` provisional at M2; F14, the slider spans of `ν₀`, the held `L_z` and the held `K` answered now, proposed by physics for the owner to confirm, and tilts into them in M2; TASK-M2-57's three questions answered (RQ-754): one periodic `β` link, the presets and goldens along `z_β` redefined, an unpinned saved chart converted from `sigmoid_beta` with a notice; the R-864 type sketch approved when TASK-M2-61 brings it, from type-sketch v10; the trajectory interface's three-body fixtures the Lagrange equilateral homographic solution and the figure-eight, two-body Kepler only where the interface is generic, the constants in a lower crate; F2 to F20 stand; every calibration set by measurement with its criterion written first *(closes RQ-754; supersedes R-882; refines R-864 and R-902)*"

## Deliverables
- One leaf per operation in `crates/engine`'s chart-path module; a pair measuring bitwise-equal under D2's criterion merged with no bit change.
- Routes differing only in rounding resolved to one canonical formula, the bit change deliberate and documented, with a conversion test; a difference that isn't rounding not merged but brought back to the owner.
- Every navigation edit through REQ-CHART-060's stop in the leaf, the history holding the clamped value (R-469; REQ-CHART-168).
- The merged GUI routes in `crates/gui` as thin input adapters feeding the same leaf, holding no formula (R-919, F1 (a)); the dispatch classes in `crates/gui/src/dispatch.rs` and the path in `chart_space` (R-894).

## Acceptance tests
- `cargo test -p engine --test chart_path_consolidate -- path_merge_bitwise path_canonical_conversion path_pin_kept path_control_` and `cargo test -p gui --test chart_path_consolidate -- path_merge_bitwise path_canonical_conversion path_adapter_same_leaf path_pin_kept path_control_` — merged pairs bitwise; each changed case listed old against new with its largest difference in ulps, within the count derived from the two formulas, and, farther than the stated δ from the binary-collision landmarks (ulp/δ ≤ ε_phys), decoding to the same system within ε_phys; each adapter's output equals its leaf's constructor's bitwise; controls: a reordered operation fails, an adapter holding a formula fails; plus review: the merged-away leaf names gone by a word-bounded git grep (REQ-CHART-156).
- `cargo test -p engine nav_stop_edit_path` — on (L_z, E) under linear, a pan past the feasible edge through the leaf: the recorded after equals the clamped z₀ bit for bit, the stop's reason names layer 2 and the constraint; a drag past the clamp emits nothing new and back inside emits the value; controls: the requested value recorded, a stop applied after the edit is recorded, each fail (REQ-CHART-168).

## Notes
- Split from TASK-M8-129 by R-916 (9 Nov 2026), its adapters added by R-919 (F1 (a)), so gui reviews it. TASK-M8-129 keeps D19, D23 and REQ-GUI-431. The engine half of REQ-GUI-243 (the stop in the edit path) is REQ-CHART-168.
- The outcome-class arm of the conversion test needs integrated outcomes: M2 checks the decoded state within `ε_phys`, and TASK-M3-67 checks the class (R-918).
- A difference that isn't rounding is not merged and not patched around: stop and bring it back (R-864 item 4).
- **Build order (R-916):** between stages 1 and 2; TASK-M2-65 depends on it.
- *11 Nov 2026 (the re-plan's § A.1, CHANGE; entered by the re-plan's addition, `plan/charts/m2-chart-replan-addition.md` § F.4):* the end state is window edits (move `c`, scale `Q`, rotate `Q` per R-474 map, cut, rebase, lock, re-solve, set `s`); `engine::nav::directions`' "morph" is renamed a tilt (R-925 item 4; applied per R-369: naming). The lines above stand as written until this task's file is rewritten under those rulings; where they differ, the rulings bind.
