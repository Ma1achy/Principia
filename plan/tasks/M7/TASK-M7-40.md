# TASK-M7-40 — The Chazy time views: when it was decided, and what just changed

- **Milestone:** M7
- **Closes:** REQ-COL-082, REQ-COL-083, REQ-COL-106
- **Depends on:** TASK-M7-37, TASK-M7-12
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~260 lines (the second latch, the comparison and two views; the calibration proposal in the PR)

## Goal
The two views that summarise the label's history (R-442; the Chazy drill-down §10.8). "When it was decided": colour
the final class, lightness how early the coarse class settled for good, live from a second forward-only latch, the time
the current coarse class was first committed, reset whenever it changes. "What just changed": white where the coarse
class changed since the previous playhead, the rest dimmed. The time-lightness mapping and the dimming are proposed
for the M7 gate (R-71).

## References
- `docs/design/principia_dd_chazy_final_motion.md` § "10.8 Time views"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.8.1 When it was decided"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.8.2 What just changed"
- `docs/design/principia_dd_chazy_final_motion.md` § "11.2 Calibrations to confirm at a gate (R-71)"
- `docs/contracts/principia_render_contract.md` § "Part 2 — Fixed pipeline, swappable slots"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-442 — Chazy final motion is a separate system with its own stains: hue carries the class, B is black `#141418` and one class, collision is magenta `#DA85B6`, transient draws as B, the working states have their own status map, and the seven-class palette is the default"

## Deliverables
- The "when it was decided" view, offline only in v1 (R-545, RQ-310): computed over a recorded run, the first-commit
  time of the current coarse class taken from the trajectory; no second latch is stored in the payload.
- The previous-class comparison, placed as RQ-320 rules, and the "what just changed" view.
- The calibration proposal for the "when it was decided" lightness by time and the "what just changed" dimming, with
  renders, in the PR (REQ-COL-106).

## Acceptance tests
- `cargo test -p kernel chazy_decided_latch` — on a fixture trajectory the latch holds the first-commit time of the current coarse class and resets when it changes; the view renders a never-settled sample grey and an earlier-settled one lighter (REQ-COL-082).
- `cargo test -p render chazy_what_changed` — two frames across a playhead step: exactly the samples whose coarse class changed are white, the rest dimmed by the proposed amount (REQ-COL-083).
- Review (gui): the time-lightness mapping (palest by t = 10 to darkest at t = 800, grey never) with renders on `preset_shape_h1`, and the "what just changed" dimming with renders, confirmed by the human at the M7 gate (REQ-COL-106).

## Notes
- RQ-310 is ruled (R-545): the "when it was decided" view is offline only in v1, so REQ-COL-082 no longer waits.
  *Was:* "This task waits on two rulings: RQ-310 (the latch's bits; for the human) for REQ-COL-082, and RQ-320 (where the …".
- This task waits on RQ-320 (where the
  previous frame's class lives) for REQ-COL-083; REQ-COL-106 carries both. It starts when they are ruled; no other task
  waits on it.
- The dimming and the time-lightness mapping are this task's calibration, REQ-COL-106, moved from REQ-COL-087 in the
  PR #223 review (R-71: proposed by the task that needs it).
- Physics reviews this task because its first-commit latch is a second forward-only latch in the march, persisted in
  the payload as RQ-310 would rule (now offline only, R-545, so nothing is persisted; physics still reviews the offline latch): payload physics state, as `I_max` is in TASK-M7-35 (applied per R-369, the PR #223 code
  review).
