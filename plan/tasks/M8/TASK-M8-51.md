# TASK-M8-51 — The line system's open questions as R-509 rules them: the stain's expression syntax and the ledger's fields, the trace fade's proxy, trace labels winning, Screen contour the default

- **Milestone:** M8
- **Closes:** REQ-RENDER-097, REQ-RENDER-098, REQ-GUI-249, REQ-GUI-250
- **Depends on:** TASK-M7-43, TASK-M8-49, TASK-M8-50
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~400 lines, the tests included

## Goal
Apply R-509's answers to RQ-377: (1) contour expressions typed in the stain's expression syntax and (2) any ledger-declared payload field at the playhead (REQ-RENDER-097); (3) the trace fade by the proxy where the chart defines it, no fade elsewhere, said so (REQ-GUI-249); (4) trace labels win (REQ-RENDER-098); (6) Screen contour the default, smart ink opt-in (REQ-GUI-250). Part (5), class edges, is TASK-M8-50's; part (3)'s general distance is TASK-M8-54's. *Was:* "Apply RQ-377's rulings: … Part (5), class edges, has no requirement (nothing waits)."

## References
- `decisions.md` § "R-476 — The line system is built as `principia_lines.md` specifies, a separate system outside the node graph drawing over the finished figure: contours of any expression over chart, latent and payload quantities, hover and pinned traces, axes, ticks and labels, grids and style presets; it absorbs the grid and `t_end` contours overlays as presets, the colour composition's bands staying in the graph; traces are the orbit viewer's too; energy-type contours follow the active mapping, per pixel; the design's six open questions are queued (RQ-377)"
- `docs/design/principia_lines.md` § "11. Open questions"
- `decisions.md` § "R-509 — `principia_lines.md` §11's six questions are ruled: contours in the stain's expression syntax; the playhead reads any ledger-declared payload field; the trace fade uses the proxy where it is defined, else no fade and says so, the real distance waiting on RQ-341; trace labels win; class edges move to the line system; Screen contour the default, smart ink opt-in *(closes RQ-377; amends R-476)*"
- `docs/design/principia_lines.md` § "4.1 The expression"
- `docs/design/principia_lines.md` § "7. Labels"
- `docs/design/principia_lines.md` § "8. Traces"
- `docs/design/principia_lines.md` § "6. Smart ink"

## Deliverables
- Per RQ-377's ruling, in `crates/render/src/lines/` and the contour-set editor.
- Tests `contour_syntax`, `label_collisions_traces`, `trace_fade`, `contour_default_style`.

## Acceptance tests
- `cargo test -p render contour_syntax` — REQ-RENDER-097's verify.
- `cargo test -p render label_collisions_traces` — REQ-RENDER-098's verify.
- `cargo test -p render trace_fade` — REQ-GUI-249's verify.
- `cargo test -p gui contour_default_style` — REQ-GUI-250's verify.

## Notes
- **Physics reviews** part (3): where the proxy is defined (R-509 Y3); the general distance, which needs the encode and a metric (RQ-341, RQ-346), is TASK-M8-54's.
- **No longer waiting (R-509 rules RQ-377).** Nothing depends on it but TASK-M8-54. *Was:* "A leaf task waiting for RQ-377".
