# TASK-M7-43 — Contours: the expression's variables and totality, evaluation on the decode, levels, marching squares, and energy contours that follow the active mapping (R-476, principia_lines.md §4) ⚑

- **Milestone:** M7
- **Closes:** REQ-RENDER-092, REQ-RENDER-093, REQ-RENDER-094, REQ-RENDER-095, REQ-RENDER-096, REQ-PERF-123
- **Depends on:** TASK-M7-42, TASK-M2-37, TASK-M2-67, TASK-M2-90
- **Needs (earlier milestones):** REQ-CHART-067, REQ-CHART-069
- **Reviewers:** code, qa, gui, physics, perf
- **Pitfalls:** none
- **Size:** ~900 lines, the tests included

## Goal
Contours of any expression over §4.1's variables, total; starting-state contours evaluated on the decode at screen resolution (GPU pass, CPU fallback), playhead contours on the quads' payload; levels by count, step or list; marching squares, no segment over a non-finite or not-a-system cell; energy contours through the decode with the per-pixel warp, true level sets on a tilted plane (L4).

## References
- `decisions.md` § "R-476 — The line system is built as `principia_lines.md` specifies, a separate system outside the node graph drawing over the finished figure: contours of any expression over chart, latent and payload quantities, hover and pinned traces, axes, ticks and labels, grids and style presets; it absorbs the grid and `t_end` contours overlays as presets, the colour composition's bands staying in the graph; traces are the orbit viewer's too; energy-type contours follow the active mapping, per pixel; the design's six open questions are queued (RQ-377)"
- `decisions.md` § "R-471 — Energy-type axes (`E`, `L_z`, `K`, `KE`, `PE`) are warped by default, as chart_reference §2.1 warps them, the warp evaluated per pixel with that pixel's own `U`, `I` and masses, so every pixel inside `[0,1]²` is feasible under every tilt; a linear mapping is an option per axis, its infeasible pixels "not a system" and the navigation stop keeping the centre feasible, recorded in the view state and the sidecar ⚑ *(supersedes R-408's port B1 as the default)*"
- `decisions.md` § "R-473 — Tilt and slice on every chart ⚑: the tilt is a rotation in the 3-space of the two screen axes and the hidden target, applied to the chart's own parameters, `(a, b, d) = R(τ₁, τ₂)·(u, v, 0)`, `a` and `b` through the chart map exactly and `d` added to the hidden coordinate at the slice centre, untilted bitwise today's; a mass, and every linked coordinate, is tilted in its link's pre-image; the units are queued (RQ-375); the slice moves any hidden coordinate *(extends R-457)*"
- `docs/design/principia_lines.md` § "4.1 The expression"
- `docs/design/principia_lines.md` § "4.2 Evaluation"
- `docs/design/principia_lines.md` § "4.3 Levels"
- `docs/design/principia_lines.md` § "4.4 Extraction"
- `docs/design/principia_lines.md` § "10. How it fits today's other decisions"

## Deliverables
- `crates/render/src/lines/contours/` (the expression tree, evaluation, levels, extraction).
- Tests `contour_expr`, `contour_eval`, `contour_levels`, `contour_extract`, `energy_contours`.

## Acceptance tests
- `cargo test -p render contour_expr` — REQ-RENDER-092's verify.
- `cargo test -p render contour_eval` — REQ-RENDER-093's verify.
- `cargo test -p render contour_levels` — REQ-RENDER-094's verify.
- `cargo test -p render contour_extract` — REQ-RENDER-095's verify (exactness, totality).
- `cargo test -p render energy_contours` — REQ-RENDER-096's verify (L4).
- *11 Nov 2026 (R-952):* `cargo test -p render contour_blend_ends` — REQ-RENDER-092's verify as it now reads, the per-end evaluation of Chart and Latent variables on a blend ((C), applied per R-369) (REQ-RENDER-092); `contour_eval` reads REQ-RENDER-093's verify as it now reads: the CPU's f64 screen grid, the check at a depth where f64 itself rounds reported and never asserted against a fixed depth, the control (a contour read from the quads' payload moves with refinement) kept (REQ-RENDER-093).
- *11 Nov 2026 (R-952):* `cargo xtask bench contour_pass` — REQ-PERF-123's verify: R-952's 8.3 ms criterion on the half-resolution grid, 960 × 540, on the three test charts, kept as written and not adjusted after the run; the report says what the owner will see while moving on each chart. **Owed, the owner's:** R-952 gives no rule for a quarter-resolution grid that also misses 8.3 ms; if that happens it is reported with the numbers and the owner decides, nothing stepped down further meanwhile; benchmark: waits for the human's Mac (R-186) (REQ-PERF-123).
- *11 Nov 2026 (the physics review of PR #482, review 5852979675, P4):* `contour_eval` reads REQ-RENDER-093's verify as it now reads, the line of this date above giving way: the contour's decode is the route the picture's forward table takes; at depth 30 on the latent chart it matches the CPU f64 decode's level set to one grid cell; past R-90's switch it is checked against the level set of the one-cell patch the picture itself uses, the depth reached reported; control: a contour from the plain f64 map alone, which stops resolving where f64 rounds while the picture goes on (REQ-RENDER-093).

## Notes
- **⚑ Physics review is required** (the instruction's "How to work this", 4): this task does not merge on the code reviewer's word alone; its reviewer list includes physics. It covers L4, the energy contours on a tilted plane (R-476 M7).
- **RQ-377 (1), (2)** wait (REQ-RENDER-097, TASK-M8-51); meanwhile §4.1's tree and its named fields.
- **R-509 (20 Oct 2026) rules RQ-377 (1) and (2):** contours reuse the stain's expression syntax, and the playhead reads any field the ledger declares; nothing waits (REQ-RENDER-097, TASK-M8-51). The note above is kept as history (code review 5587720942 of PR #255, F7).
- Chart composition (6 Nov 2026): §4.1's variables grow by TASK-M7-72, after this task.
- *11 Nov 2026 (the re-plan's addition (`plan/charts/m2-chart-replan-addition.md`) § F.4; R-952):* starting-state contours are evaluated on the CPU's f64 screen grid for every chart, a blend included; no GPU contour pass is planned (the GPU decode entries stay test entries only, R-935 under RQ-765 (a)). Its dependence on TASK-M2-25 (the generated WGSL decode) is dropped; it depends on TASK-M2-67 (the CPU evaluator) and TASK-M2-90 (the blend chart). It gains a benchmark (REQ-PERF-123), so `perf` reviews it. Playhead contours, read from the resident quads' payload, are unchanged. Where a dated line under Acceptance tests differs from an earlier line for the same requirement, the dated line binds and the earlier one is kept as its *Was*. *Was (Depends on):* "TASK-M7-42, TASK-M2-37, TASK-M2-25"; *was (Closes):* "REQ-RENDER-092, REQ-RENDER-093, REQ-RENDER-094, REQ-RENDER-095, REQ-RENDER-096"; *was (Reviewers):* "code, qa, gui, physics".
- *11 Nov 2026 (the physics review of PR #482, review 5852979675, P4; R-952's dated correction):* contours move to the CPU; they keep the decode's precision path. Lines § 4.2's "It uses the same precision path as the decode (the linearised decoder at depth)" stands: the contour's decode is the route the picture's forward table takes, the one-cell patch included where R-90's switch fires (R-926 item 11; R-935, under RQ-766 (a)), so "exact at any zoom and any depth of refinement" stays true.
