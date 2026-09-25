# TASK-M5-25 — The two-regime scheduler: in-motion coarse cover, debounce and the backdrop reference

- **Milestone:** M5
- **Closes:** REQ-SCHED-020, REQ-SCHED-075, REQ-RENDER-039, REQ-SCHED-088
- **Depends on:** TASK-M5-15, TASK-M5-21
- **Needs (earlier milestones):** REQ-SCHED-001
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
While a gesture is active the scheduler dispatches only the full-canvas coarse cover of the current identity in
`PREVIEW_MODE`, complexity scoring suspended, the frame budget governing and ancestors showing; the full machinery
(including must-split above the screen floor) resumes when the debounce fires, the debounce proposed as a calibration.
The backdrop reference updates only on an at-rest, refined, non-preview baseline, with exactly two identities pinned
(current + backdrop) and swapped atomically.

## References
- `docs/contracts/principia_caching_contract.md` § "Part 5 — The stale backdrop: blur means loading"
- `docs/contracts/principia_scheduler_contract.md` § "Part 7 — The frame loop (lockstep presentation)"
- `decisions.md` § "R-108 — Must-split above the floor is the at-rest target *(closes RQ-68)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/contracts/principia_caching_contract.md` § "Part 4 — Baseline-first: an absolute tier, not a priority weight"

## Deliverables
- `crates/engine/src/sched/regime.rs`: gesture state, debounce, in-motion dispatch; backdrop-reference update.
- Property tests `crates/engine/tests/regime.rs`: synthetic drag dispatches only preview coarse-cover jobs; a long drag
  never promotes a coarse cover to backdrop.
- The debounce calibration proposal: scripted drag sequences, no premature at-rest dispatch, resume latency.

## Acceptance tests
- `cargo test -p engine in_motion_coarse_only` — during a synthetic drag, only preview coarse-cover jobs are dispatched (REQ-SCHED-020).
- Review checklist (perf) — decisions.md records the interval with its evidence: scripted drag sequences showing no premature at-rest dispatch and the at-rest resume latency; the proposed value is marked pending and the human confirms it at the M5 gate, then it is recorded in `decisions.md` (REQ-SCHED-075).
- `cargo test -p engine backdrop_reference_at_rest` — a long drag never promotes a coarse cover to backdrop (REQ-RENDER-039).
- Proposal: the in-motion coarse cover's level offset (and whether it is the baseline cover's) with evidence from scripted tilt/slice drags; the human confirms it at the M5 gate (REQ-SCHED-088).

## Notes
- Calibrations (R-71) this task proposes: REQ-SCHED-075.
- REQ-SCHED-075 is an R-71 calibration; the human confirms it at the M5 gate.
- The coarse cover's depth ("a few levels above camera depth", caching Part 5) is not stated to be the baseline cover's
  depth of REQ-SCHED-074 (see Gaps).
- Closes, for gaps the corpus leaves open: REQ-SCHED-088 (R-71 calibration) (REVIEW_QUEUE RQ-110 lists them for the human).
