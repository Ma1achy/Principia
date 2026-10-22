# TASK-M8-72 — Degrade while moving: per-node motion policies, settle, the playhead not motion, and the print stage in the frame record (`stage_ms.post`, `post_level`, schema v2) (R-488, R-534, R-557)

- **Milestone:** M8
- **Closes:** REQ-PERF-098, REQ-PERF-099, REQ-PERF-100, REQ-TOOL-190
- **Depends on:** TASK-M7-68, TASK-M5-28, TASK-M6-18, TASK-M8-68
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~600 lines, the tests included

## Goal
While the camera moves or a drag is in progress each node runs by its motion policy (`full`, `reduced` at its measured factor, `bypass`); on settle (`settle_ms`, with hysteresis, proposed here) the graph renders at full quality once and holds; the playhead advancing is not motion, and a playback the full graph cannot hold is recorded and changes nothing. The frame record gains the print stage, `post` between `colour` and `upload` and `post_level` per frame, under every tier (schema v2's print-stage part, R-534). §11 test 7. The task defines what `post_level` records when only the per-node policies act.

## References
- `docs/design/principia_print_graph.md` § "8.1 Degrade while moving"
- `docs/design/principia_print_graph.md` § "8.2 The budget"
- `docs/design/principia_print_graph.md` § "11. Tests (each with an arm that can fail)"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "2. What to record — and the rule that makes it useful"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender"
- `docs/gui/principia_render_gui_spec.md` § "Profiler"
- `decisions.md` § "R-488 — Degrade while moving: each Print graph node runs by its motion policy (`full`, `reduced`, `bypass`), the whole graph falls back a level while over budget, and full quality returns on settle; the playhead advancing is not motion"
- `decisions.md` § "R-534 — The post stage and the benchmark lines make profiler schema v2, `principia-profile-v2`; every reader accepts v1 *(closes RQ-399; amends R-497; extends R-56)*"
- `decisions.md` § "R-557 — A Print graph field input takes the stain's window when the stain colours by the same field, else the field's own `range_norm` default; the print stage's time is recorded but never moves the Auto rung, and its post level is the arbiter's motion-regime lever, under Auto only; the human approves PR #268 with its follow-ups *(closes RQ-478, RQ-479 and RQ-495; amends R-488)*"

## Deliverables
- `crates/render/src/print/motion.rs` — the policies and settle.
- `crates/engine/src/telemetry/` — `post` and `post_level` in the frame record; the v2 header for these keys; the v1 reader path.
- `docs/design/principia_print_graph.md` §8.2 — the `post_level` definition (REQ-PERF-100), with a "Removed lines" note.
- Tests `print_motion_degrade`, `frame_record_print_stage`.

## Acceptance tests
- `cargo test -p gui print_motion_degrade` — REQ-PERF-098's verify (a scripted pan under a named tier records `post_level` dropping while the camera moves and returning to `full` within `settle_ms` of the last camera change; a playing playhead in a still view stays `full`; a playback the full graph cannot hold records the shortfall and changes no setting; controls: a still view never leaves `full`, and an executor that treats the playhead as motion fails the playback arm).
- Calibration: the PR proposes both from scripted pans and stops on the reference machine (how often settle fires mid-gesture against how long the reduced look lingers); a reviewer checks them; the human confirms them at the M8 gate (REQ-PERF-099).
- Definition: §8.2 states the rule for every mix of per-node levels and the whole-graph level; the perf reviewer checks that §11 test 7's arms are decidable under it under every tier (REQ-PERF-100).
- `cargo test -p engine frame_record_print_stage` — REQ-TOOL-190's verify (a frame with a Print graph records `post` between `colour` and `upload` and its `post_level`; a headless frame records both `null`; the file's header reads `principia-profile-v2`; a v1 file reads with both keys not recorded, never 0 ms or `full`; controls: a frame record with `post` after `upload` fails the order check, and a v1 reader that rejects the v2 keys fails).

## Notes
- Schema v2 (R-534) is v1 plus the print stage, `post_level` and the benchmark's lines; this task writes the print stage's part, and the benchmark's lines are the benchmark plan's (applied per R-369).
- R-557's rulings on RQ-479 and RQ-495 landed before this task, as the human asked (R-557, PR #268's follow-up 3).
- Reviewers (R-485, the human's point 5): the gui and perf reviewers for the Print graph, with code and qa as always; the physics reviewer for the field inputs only.
