# TASK-M8-49 — Traces on the line system: the hover, pinned and kept traces drawn by §8, shared with the orbit viewer (R-476 L3)

- **Milestone:** M8
- **Closes:** REQ-GUI-245
- **Depends on:** TASK-M7-42, TASK-M8-09, TASK-M8-10, TASK-M8-11, TASK-M7-52, TASK-M2-99, TASK-M2-100
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~600 lines, the tests included

## Goal
The traces are drawn by the line system: projection through the chart's forward map, breaks where the path leaves the chart, markers, ticks, colour, arrows, animation, length, the incomplete-run mark; the same `Trace` and renderer serve the Inspector window and the Trajectory panel.

## References
- `decisions.md` § "R-476 — The line system is built as `principia_lines.md` specifies, a separate system outside the node graph drawing over the finished figure: contours of any expression over chart, latent and payload quantities, hover and pinned traces, axes, ticks and labels, grids and style presets; it absorbs the grid and `t_end` contours overlays as presets, the colour composition's bands staying in the graph; traces are the orbit viewer's too; energy-type contours follow the active mapping, per pixel; the design's six open questions are queued (RQ-377)"
- `docs/design/principia_lines.md` § "8. Traces"
- `docs/design/principia_trajectory_viewing.md` § "3. Hover trace — projection onto the current (tilted, sliced) plane"
- `docs/contracts/principia_render_contract.md` § "Part 7 — The hover trace (per-IC trajectory overlay)"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"

## Deliverables
- The trace's projection through the chart's forward map and its breaks where the path leaves call TASK-M7-52's shared projection (`crates/kernel`), never their own (R-542 H11).
- The `Trace` drawing in `crates/render/src/lines/`, the figure's hover trace (TASK-M8-10's) moved onto it, the orbit viewer's views reusing it.
- Test `line_traces`.

## Acceptance tests
- `cargo test -p render line_traces` — REQ-GUI-245's verify (§12 trace projection).
- `cargo test -p render shared_projection_callers` (the line-trace arm) — REQ-COL-135's arm for this consumer: it calls the kernel function on the CPU, its projected points and statuses bitwise equal to REQ-COL-134's shared source's; controls: a copied projection with one constant changed fails, and a hand-written projection defined outside `crates/kernel` fails the symbol check, the snippet `cargo xtask build-kernel` emits to `crates/render/wgsl/` exempt. *Was (PR #270 code re-review, C4, applied per R-369):* "`cargo test -p render shared_projection_callers` (the line-trace arm) — REQ-COL-135's arm for this consumer: its projection is REQ-COL-134's shared source (control: a copied projection with one constant changed, and a projection defined outside `crates/kernel`, each fail)."
- *11 Nov 2026 (R-947):* `cargo test -p render line_traces_blend` — on the two blend pairs, the latent chart to the shape sphere's exponential map, and the `(L_z, E)` chart to the shape sphere's exponential map, at `s = 0.25`, `0.5` and `0.75`: a hover trace's first vertex is the hovered pixel exactly; every vertex whose state is a pixel's own state (the `t = 0` state of a pinned trace) is placed at that pixel with its residual within the lock's tolerance (REQ-CHART-158); at `s = 0.0` and `1.0` every vertex equals the end chart's own trace bitwise. Controls, each of which must fail: a trace placed by chart A's encode reading at `s = 0.5` misses the first-vertex check; a trace at `s = 0.0` read by the arithmetic blend of the ends at `s = 10⁻⁶` fails the bitwise check. This test checks only points on the surface; what a trace does off it is measured by P2 and P4, not asserted (R-951) (REQ-GUI-245).

## Notes
- **Physics reviews** the projection onto the chart's own coordinates (R-476 M4).
- **The fade** waits for RQ-377 (3) (REQ-GUI-249, TASK-M8-51); meanwhile none.
- **R-509 (20 Oct 2026) rules RQ-377 (3):** the fade uses the proxy where the chart defines it, else no fade and the settings say so (REQ-GUI-249, TASK-M8-51); the general distance waits on RQ-341 (REQ-GUI-266, TASK-M8-54). The note above is kept as history (code review 5587720942 of PR #255, F7).
- **R-538 (20 Oct 2026):** §8's markers become the icon sets, built by TASK-M8-58 (REQ-GUI-271 to REQ-GUI-276); REQ-GUI-245 no longer draws them, and this task draws the trace without markers for TASK-M8-58 to add.
- **The one projection (R-542 H11; PR #270's code re-review, C1; applied per R-369):** this task depends on TASK-M7-52 and calls its shared projection. *Was:* "- **Depends on:** TASK-M7-42, TASK-M8-09, TASK-M8-10, TASK-M8-11".
- *11 Nov 2026 (the re-plan's addition (`plan/charts/m2-chart-replan-addition.md`) § F.4; R-947, R-951):* every trace is placed through `place` (TASK-M2-99) on every chart. On a blend that is its nearest-point route, seeded by the previous sample's place, or by the backwards table where there is no previous one or the solve fails from it; the first vertex of a hover trace is the hovered pixel itself; a sample leaves where the solve reports failure, with the rings; where the frame folds the trace takes the point nearest its previous sample. **Not built until the owner has seen P2 and P4** (TASK-M2-100's report, a new dependency): no threshold and no fallback is picked meanwhile. *Was (Depends on):* "TASK-M7-42, TASK-M8-09, TASK-M8-10, TASK-M8-11, TASK-M7-52".
- *11 Nov 2026 (the physics review of PR #482, review 5852979675, P5):* **a blend's trace is drawn unfaded.** The note of the same day above had "the fade is the route's residual, in place of the hidden coordinates' distance"; that clause is withdrawn. Fading a trace by its residual is the fallback the owner names as likely and keeps for themselves (R-951), so it is not built: a blend has no hidden coordinates, the off-slice distance has no value there, and lines § 8's rule applies, no fade, and the GUI says so (R-509), until the owner has seen P2 and P4 and decided. No opacity scale for a residual is proposed or chosen. "A failed solve" means the solve did not converge: off the chart's image a large residual is the projection distance, never a failure (REQ-CHART-199).
