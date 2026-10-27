# TASK-M7-69 — `device_px` and the screen-anchored sheet in a second pass at screen resolution, at every render_scale and at export (R-490, R-618)

- **Milestone:** M7
- **Closes:** REQ-RENDER-110, REQ-RENDER-120, REQ-PERF-113
- **Depends on:** TASK-M7-55, TASK-M7-60
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~250 lines, the tests included

## Goal
A Print graph's `device_px` nodes and the screen-anchored sheet run in a second print pass at screen resolution, after the display scale (R-618, RQ-538 (b)), so a scanline is exactly one screen pixel at every render_scale, the anchoring coordinate is the pixel's screen position, and at export a display pixel is an export pixel; the rest of the graph runs before the display scale at render resolution. This task defines the split into the two passes in print_graph §6.2 (REQ-RENDER-120, R-72) and builds it: the CRT, LCD and LED matrix nodes and a sheet-only graph keep their pitch and anchoring at render_scale 0.5, 1 and 2, and at export, and a graph with neither stays bitwise, with no screen pass dispatched and no target allocated; the pass's frame cost is recorded on the Mac (REQ-PERF-113), and TASK-M7-33 counts k_d after it. *Was (perf review 5661999637 of PR #293, P2):* "and a graph with neither stays bitwise. A leaf." *Was:* "Whichever pixels RQ-538 rules `device_px` lengths and the screen anchoring are counted in, while the Print graph runs before the display scale at render resolution: … Waits on RQ-538; a leaf, so nothing else waits."

## References
- `docs/design/principia_print_graph.md` § "6.2 Units: a medium must look the same at every export size"
- `docs/design/principia_print_graph.md` § "6.3 Anchoring and stability under motion and playback"
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `docs/design/principia_quality_device_note.md` § "5. Timescale cascade (each layer absorbs what's too fast for the one above)"
- `decisions.md` § "R-490 — The Print graph is deterministic, seeded only; every length is in `ref_px`, `device_px` or `fraction`; the sheet is anchored to the screen; `boil` is off by default"
- `decisions.md` § "R-618 — A Print graph's `device_px` effects and the screen-anchored sheet run in a second pass at screen resolution, after the display scale; at export a display pixel is an export pixel *(closes RQ-538; amends R-487's and R-531's composite order)*"
- `docs/gui/principia_render_gui_spec.md` § "12.1 Structural overlays and tile debug shaders"
- `docs/design/principia_benchmark.md` § "3. The suite"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "1.1 The fixed suite — comparability across devices"
- `decisions.md` § "R-186 — GitHub-hosted runners first; no self-hosted runner *(amends R-110, R-169, R-174)*"
- `decisions.md` § "R-557 — A Print graph field input takes the stain's window when the stain colours by the same field, else the field's own `range_norm` default; the print stage's time is recorded but never moves the Auto rung, and its post level is the arbiter's motion-regime lever, under Auto only; the human approves PR #268 with its follow-ups *(closes RQ-478, RQ-479 and RQ-495; amends R-488)*"

## Deliverables
- `crates/render/src/print/units.rs` and the display stage — the screen pass after the display scale, running the `device_px` nodes and the sheet at screen resolution (R-618). *Was:* "the `device_px` and anchoring rule RQ-538 rules."
- Doc change: print_graph §6.2 — how a graph splits into the two passes (REQ-RENDER-120, R-72).
- Test `print_device_px_scale`.
- `xtask` bench `print-screen-pass` — the screen pass's frame cost, a record (REQ-PERF-113; perf review 5661999637 of PR #293, P2 (b)).

## Acceptance tests
- `cargo test -p render print_device_px_scale` — REQ-RENDER-110's verify (the CRT, LCD and LED matrix nodes and a sheet-only graph rendered at render_scale 0.5, 1 and 2, displayed, agree in pitch, one scanline per screen pixel, and in anchoring at each pixel's screen position; at export a display pixel is an export pixel; Plain renders bitwise as before, its counted passes and allocated targets equal to the no-Print baseline's, the screen pass neither dispatched nor allocated; controls: a node counting `device_px` in render pixels, run before the display scale, fails the pitch arm at render_scale 0.5, a sheet anchored in render pixels fails the anchoring arm, and a pass-through copy at display resolution under Plain, bitwise equal, fails the count arm). *Was (perf review 5661999637 of PR #293, P2 (a)):* "Plain renders bitwise as before; controls: … and a sheet anchored in render pixels fails the anchoring arm)". *Was:* "agree in pitch and anchoring as RQ-538's ruling says; control: a node counting `device_px` in the wrong pixels fails the pitch arm at render_scale 0.5" (R-618).
- `cargo xtask bench print-screen-pass` — REQ-PERF-113's verify: the fixed suite's scenarios (principia_benchmark.md §3) on a 3840 × 2160 display at render_scale 0.5 and 1, each under a CRT graph, a sheet-only graph and Plain; frame time p50/p95/p99/max and the screen pass's own GPU time per frame recorded against Plain's (REQ-SYS-060's form; a record, no new threshold); fails if the screen pass was not dispatched under the CRT and sheet-only graphs, or was under Plain; run on the human's Mac (R-186), its own process, its `prin profile` output in the PR, and the PR waits for that run (REQ-PERF-113; perf review 5661999637 of PR #293, P2 (b)).
- Review checklist (the reviewers) — print_graph §6.2 states the split into the two passes, a node between two `device_px` nodes and where the sheet runs, and that with neither the screen pass is the identity, not dispatched and allocating no target (REQ-RENDER-120). *Was:* "and that with neither the screen pass is the identity (REQ-RENDER-120)" (perf review 5661999637 of PR #293, P2 (a)).

## Notes
- R-618 (25 Oct 2026) rules RQ-538: option (b), the sheet with the `device_px` nodes as the option's words have it (flagged in R-618). *Was:* "Waits on RQ-538 (REQ-RENDER-110 carries it)."
- Definition written here (R-72; the reviewers approve before merge): REQ-RENDER-120.
- perf review 5661999637 of PR #293, P2, applied per R-369: the screen pass's "identity" is no pass and no target, asserted by count (a); its frame cost is a Mac record, REQ-PERF-113 (b); TASK-M7-33 counts k_d after this task, so its count includes the screen pass's targets (c). No longer a leaf: TASK-M7-33 depends on it.
- Reviewers (R-485, the human's point 5): the gui and perf reviewers for the Print graph, with code and qa as always; the physics reviewer for the field inputs only.
- *Was (title, before R-618/R-596):* "`device_px` and the screen-anchored sheet at render_scale ≠ 1 and at export, as RQ-538 rules (R-490)".
