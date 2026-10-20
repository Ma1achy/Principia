# TASK-M8-54 — The trace fade's general off-plane distance, as RQ-341 rules (R-509 part 3) ⚑

- **Milestone:** M8
- **Closes:** REQ-GUI-266
- **Depends on:** TASK-M8-51
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~250 lines, the tests included

## Goal
On a chart with no proxy, a trace fades off the plane by the general distance, the distance in `z` from the trajectory's encoded point to the slice plane, encoded per step, in the metric RQ-341 rules (R-509 Y3); TASK-M8-51 draws no fade there meanwhile.

## References
- `decisions.md` § "R-509 — `principia_lines.md` §11's six questions are ruled: contours in the stain's expression syntax; the playhead reads any ledger-declared payload field; the trace fade uses the proxy where it is defined, else no fade and says so, the real distance waiting on RQ-341; trace labels win; class edges move to the line system; Screen contour the default, smart ink opt-in *(closes RQ-377; amends R-476)*"
- `docs/design/principia_lines.md` § "8. Traces"
- `docs/design/principia_lines.md` § "11. Open questions"
- `docs/design/principia_trajectory_viewing.md` § "3. Hover trace — projection onto the current (tilted, sliced) plane"

## Deliverables
- The per-step encode and the distance in the trace renderer (`crates/render/src/lines/`), per RQ-341's ruling.
- Test `trace_fade_distance`.

## Acceptance tests
- `cargo test -p render trace_fade_distance` — REQ-GUI-266's verify.

## Notes
- **⚑ Physics review is required**: the distance and its metric; it does not merge on the code reviewer's word alone.
- **A leaf task waiting for RQ-341** (`rq: RQ-341` on REQ-GUI-266): nothing depends on it. Numbered past the links port's (unmerged) TASK-M8-52 and TASK-M8-53 (applied per R-369).
- **It supersedes REQ-GUI-249's no-fade arm** on the charts the general distance covers, and its settings line "fade: no off-plane distance on this chart (RQ-341)" there (REQ-GUI-266; R-509 Y3). **qa's file:** TASK-M8-51's qa test of REQ-GUI-249's latent-chart arm (`crates/render/tests/qa_TASK-M8-51.rs`, `trace_fade`'s "every vertex at full opacity and the settings line names RQ-341" on the latent chart) changes in this task's qa commit and only there, the PR listing its `M` line with the reason, the code reviewer confirming no other assertion is weakened (R-290; the ruling forcing the change, applied per R-369, gui review 5587650119 of PR #255, G13).
