# TASK-M7-40 — The Chazy time views: when it was decided, and what just changed

- **Milestone:** M7
- **Closes:** REQ-COL-082, REQ-COL-083
- **Depends on:** TASK-M7-37, TASK-M7-12
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~250 lines (the second latch, the comparison and two views)

## Goal
The two views that summarise the label's history (R-442; the Chazy drill-down §10.8). "When it was decided": colour
the final class, lightness how early the coarse class settled for good, live from a second forward-only latch, the time
the current coarse class was first committed, reset whenever it changes. "What just changed": white where the coarse
class changed since the previous playhead, the rest dimmed.

## References
- `docs/design/principia_dd_chazy_final_motion.md` § "10.8 Time views"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.8.1 When it was decided"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.8.2 What just changed"
- `docs/contracts/principia_render_contract.md` § "Part 2 — Fixed pipeline, swappable slots"
- `decisions.md` § "R-442 — Chazy final motion is a separate system with its own stains: hue carries the class, B is black `#141418` and one class, collision is magenta `#DA85B6`, transient draws as B, the working states have their own status map, and the seven-class palette is the default"

## Deliverables
- The first-commit latch, stored as RQ-310 rules, and the "when it was decided" view.
- The previous-class comparison, placed as RQ-320 rules, and the "what just changed" view.

## Acceptance tests
- `cargo test -p kernel chazy_decided_latch` — on a fixture trajectory the latch holds the first-commit time of the current coarse class and resets when it changes; the view renders a never-settled pixel grey and an earlier-settled one lighter (REQ-COL-082).
- `cargo test -p render chazy_what_changed` — two frames across a playhead step: exactly the pixels whose coarse class changed are white, the rest dimmed by the proposed amount (REQ-COL-083).

## Notes
- This task waits on two rulings: RQ-310 (the latch's bits; for the human) for REQ-COL-082, and RQ-320 (where the
  previous frame's class lives) for REQ-COL-083. It starts when they are ruled; no other task waits on it.
- The dimming and the time-lightness mapping are part of REQ-COL-087's palette proposal (TASK-M7-37).
