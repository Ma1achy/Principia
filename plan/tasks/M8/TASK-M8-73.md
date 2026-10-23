# TASK-M8-73 — The print stage's budget: excluded from the Auto rung solve, the post level the arbiter's motion-regime lever under Auto only, the budget's definition and the print share (R-557)

- **Milestone:** M8
- **Closes:** REQ-PERF-101, REQ-PERF-102, REQ-PERF-103, REQ-PERF-104, REQ-PERF-105
- **Depends on:** TASK-M8-72, TASK-M6-17, TASK-M6-18
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~450 lines, the tests included

## Goal
The rung is solved without the print stage, against a budget that does not depend on which Print graph is active, so a Print graph never moves N, E, depth, render_scale or FTLE gating. The whole-graph fallback (reduced, then Plain) is the arbiter's post level, a motion-regime lever beside render_scale, solved from the measured cost classes, never a per-frame reading, under Auto only; under a named tier or Custom it does not run. The task defines whether the rung's budget is the whole frame budget or less a fixed print share, and proposes the share and the per-class costs.

## References
- `docs/design/principia_print_graph.md` § "8.1 Degrade while moving"
- `docs/design/principia_print_graph.md` § "8.2 The budget"
- `docs/design/principia_print_graph.md` § "8.3 Cost classes"
- `docs/design/principia_print_graph.md` § "3. What never changes because of it"
- `docs/design/principia_quality_device_note.md` § "3. The quality ladder"
- `docs/design/principia_quality_device_note.md` § "5. Timescale cascade (each layer absorbs what's too fast for the one above)"
- `docs/design/principia_quality_device_note.md` § "10. Two sanctities: the user, and observability"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "2. What to record — and the rule that makes it useful"
- `decisions.md` § "R-488 — Degrade while moving: each Print graph node runs by its motion policy (`full`, `reduced`, `bypass`), the whole graph falls back a level while over budget, and full quality returns on settle; the playhead advancing is not motion"
- `decisions.md` § "R-489 — The instrument stays honest: measurement never reads the print, goldens and scientific checks run under Plain, a styled figure is badged, and figure export defaults to Plain"
- `decisions.md` § "R-557 — A Print graph field input takes the stain's window when the stain colours by the same field, else the field's own `range_norm` default; the print stage's time is recorded but never moves the Auto rung, and its post level is the arbiter's motion-regime lever, under Auto only; the human approves PR #268 with its follow-ups *(closes RQ-478, RQ-479 and RQ-495; amends R-488)*"

## Deliverables
- `crates/engine/src/quality/arbiter/` — the post level lever and the rung solve's exclusion.
- `docs/design/principia_print_graph.md` §8.2 — the budget definition (REQ-PERF-103), with a "Removed lines" note.
- Tests `print_rung_excluded`, `print_post_level_lever`, `print_cost_model_device`; `docs/design/principia_print_graph.md` §8.3 — how the per-class cost carries to the running device (REQ-PERF-105), with a "Removed lines" note.

## Acceptance tests
- `cargo test -p engine print_rung_excluded` — REQ-PERF-101's verify (under Auto, at the same scene and device model, the resting rung (N, E, depth, render_scale, FTLE gating) is identical with Plain and with a heavy Print graph; on the same scripted pan, scene and device model, frame by frame, the motion regime's render_scale, E and the refinement floor (when locked to native) are identical with Plain and with the heavy graph; controls: a solve that reserves the print share only while a graph is active fails (R-557 B2's arm), and a motion solve that lowers render_scale, E or the refinement floor to pay for the graph fails).
- `cargo test -p engine print_post_level_lever` — REQ-PERF-102's verify (under Auto a heavy graph whose modelled cost exceeds the share drops to reduced and then Plain while moving and returns on settle; under a named tier and under Custom the same pan never changes the whole-graph level; only the arbiter module writes it; controls: injected per-frame noise on `stage_ms.post` with the model unchanged leaves the level unchanged (a per-frame reader fails), and a fallback that runs under a named tier fails, and an arbiter that drops every graph on motion fails both of: a graph whose modelled cost fits the share stays at the full whole-graph level through the same pan, and a graph whose reduced cost fits stops at reduced and never reaches Plain).
- Definition: §8.2 states the rule; the perf reviewer checks that under it the resting rung does not depend on which Print graph is active (REQ-PERF-101's arm) (REQ-PERF-103).
- Calibration: the PR proposes the share and the per-class costs from REQ-PERF-096's table and scripted pans on the reference machine; a reviewer checks them; the human confirms them at the M8 gate (REQ-PERF-104).
- Definition, and `cargo test -p engine print_cost_model_device` — REQ-PERF-105's verify (§8.3 states the rule; under it two device models with different throughput give the post levels the rule predicts for the same heavy graph (different, or the same with §8.3's reason); control: a model that ignores the device and uses the reference machine's costs unscaled fails wherever the rule scales them; the perf reviewer checks that no per-frame reading moves the level (REQ-PERF-102)).

## Notes
- The quality device note's one writer holds: the arbiter writes the post level under Auto, and under a named tier or Custom nothing does (R-557 B1).
- Reviewers (R-485, the human's point 5): the gui and perf reviewers for the Print graph, with code and qa as always; the physics reviewer for the field inputs only.
- **Fix round 3 of PR #281 (perf review 5608473498, F1 to F3, applied per R-369):** the rung arm gains the motion regime (render_scale, E and the refinement floor frame by frame); the lever gains the fits-the-share and stops-at-reduced controls; REQ-PERF-105 (new, a definition) says how the reference machine's per-class cost carries to the running device and whether `stage_ms.post` feeds the model. *Was:* "**Closes:** REQ-PERF-101, REQ-PERF-102, REQ-PERF-103, REQ-PERF-104"; "`cargo test -p engine print_rung_excluded` — REQ-PERF-101's verify (under Auto, at the same scene and device model, the resting rung (N, E, depth, render_scale, FTLE gating) is identical with Plain and with a heavy Print graph; control: a solve that reserves the print share only while a graph is active fails (R-557 B2's arm))."; "`cargo test -p engine print_post_level_lever` — REQ-PERF-102's verify (under Auto a heavy graph whose modelled cost exceeds the share drops to reduced and then Plain while moving and returns on settle; under a named tier and under Custom the same pan never changes the whole-graph level; only the arbiter module writes it; controls: injected per-frame noise on `stage_ms.post` with the model unchanged leaves the level unchanged (a per-frame reader fails), and a fallback that runs under a named tier fails)."; "Tests `print_rung_excluded`, `print_post_level_lever`.".
