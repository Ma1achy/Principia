# TASK-M6-12 — The frontier margin derived from the refill rate, benchmarked against a widened margin

- **Milestone:** M6
- **Closes:** REQ-SCHED-068, REQ-SCHED-078, REQ-SCHED-082
- **Depends on:** TASK-M6-11, TASK-M5-28
- **Needs (earlier milestones):** REQ-SCHED-032, REQ-SCHED-033, REQ-TOOL-050
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-3
- **Size:** ~380 lines

## Goal
The off-screen frontier is scoped by a margin derived from the in-view descent's refill rate (R-45), written into refinement_policy §7 as a formula with its inputs (R-72), and benchmarked on scripted camera paths against a naive cull (the known failure) and a plain widened margin (the baseline it must beat), whose size the task proposes (R-71).

## References
- `docs/design/principia_dd_refinement_policy.md` § "7. Open"
- `docs/design/principia_dd_refinement_policy.md` § "0.1 In view, the camera decides depth and the criterion decides ORDER"
- `decisions.md` § "R-45 — The frontier margin is derived from the refill rate, and a widened margin is the baseline to beat *(RS-5)*"
- `docs/read_first/principia_INDEX.md` § "Known open items"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- Doc change: `docs/design/principia_dd_refinement_policy.md` § "7. Open" — the refill-rate margin formula (REQ-SCHED-082).
- `crates/engine/src/sched/frontier.rs`: the margin computed from the measured refill rate; the widened-margin and naive-cull variants behind a bench-only switch.
- `cargo xtask bench frontier-margin` over scripted camera paths in `fixtures/bench/camera_paths/`, reporting in-view quads and stall (px texel size).

## Acceptance tests
- `cargo xtask bench frontier-margin` — scripted camera paths: compare in-view quads and stall (px texel size) for naive cull, widened margin and refill-derived margin; refill-derived must beat widened margin, and naive cull (170 px, 4 in view) is the known failure (REQ-SCHED-068).
- `cargo xtask bench frontier-margin --sweep-widened` — decisions.md records the margin size with its evidence: the scripted-camera-path benchmark of in-view quads and stall for the candidate sizes; proposal with its evidence in the PR, reviewer-checked; the human confirms the value at the M6 gate and it is recorded in `decisions.md` (REQ-SCHED-078).
- Review checklist (physics) — the doc states the formula and its inputs; the frontier-margin benchmark uses it; the doc change is merged with the physics reviewer's approval (REQ-SCHED-082).

## Notes
- Calibration proposed: REQ-SCHED-078. Definition written: REQ-SCHED-082.
