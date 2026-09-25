# TASK-M5-15 — Baseline-first tier and display-only ancestor fallback

- **Milestone:** M5
- **Closes:** REQ-SCHED-074, REQ-SYS-032, REQ-PERF-017, REQ-SCHED-028, REQ-SCHED-045, REQ-PAY-063
- **Depends on:** TASK-M5-09, TASK-M5-14
- **Needs (earlier milestones):** REQ-SCHED-001, REQ-SCHED-003, REQ-TOOL-026, REQ-RENDER-004
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
deep_zoom layer 1 works: panning never blanks. The baseline cover (viewport-covering quads a few levels above
camera depth) is a hard tier dispatched before any refinement job; a current-identity picture appears within a dispatch
or two of any navigation, starting from one root-quad dispatch. A cached ancestor always shows where a child is not
ready and zoom shows blurry ancestors that sharpen as children finish; the fallback is display-only and never writes a
child's payload. The baseline cover's depth is proposed as a calibration (REQ-SCHED-074). Two sessions with different
camera paths produce byte-identical payloads for every common quad.

## References
- `docs/contracts/principia_caching_contract.md` § "Part 4 — Baseline-first: an absolute tier, not a priority weight"
- `docs/contracts/principia_caching_contract.md` § "Part 8 — The composed guarantee"
- `docs/contracts/principia_caching_contract.md` § "Principia — caching & display continuity contract"
- `docs/design/principia_dd_refinement_policy.md` § "0. What the mechanism is actually for"
- `docs/contracts/principia_scheduler_contract.md` § "Part 1 — The firewall: the scheduler is arbitrary about *what* it looks at, never about *what* it sees"
- `docs/design/principia_deep_zoom.md` § "3. Three-layer quadtree"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-133 — The seven checkpoint-B interpretations are accepted *(closes RQ-111)*"

## Deliverables
- `crates/engine/src/sched/{baseline.rs, fallback.rs}`: the baseline tier above the priority queue; ancestor lookup
  for display.
- `crates/render`: drawing an ancestor upscaled for a missing child (quad uniforms only).
- Tests `crates/engine/tests/baseline.rs`; the firewall property test `crates/engine/tests/firewall.rs`.
- `cargo xtask bench first-cover` (dispatches to first current cover; slice change to first frame).
- `crates/engine/tests/never_blank.rs`: a property test capturing frames of a scripted fast pan and zoom (R-133).
- The calibration proposal for the baseline depth: scripted deep-zoom navigations, completion latency and coverage.

## Acceptance tests
- Review checklist (perf) — decisions.md records the level count with its evidence: baseline-completion latency and viewport coverage measured on scripted deep-zoom navigations; the proposed value is marked pending and the human confirms it at the M5 gate, then it is recorded in `decisions.md` (REQ-SCHED-074).
- `cargo xtask bench first-cover` — navigation fixture: count dispatches until first current cover; assert no blank frame (REQ-SYS-032).
- `cargo xtask bench first-cover` — time from slice change to first presented frame is one root-quad dispatch; frames keep presenting while refinement is incomplete (REQ-PERF-017).
- `cargo test -p engine fallback_display_only` — with ancestor fallback active, assert the child's payload buffer is untouched until the child's own dispatch writes it (REQ-SCHED-028).
- `cargo test -p engine never_blank_pan` — property test: capture frames during a fast pan and zoom; no blank pixels (REQ-SCHED-045).
- `cargo test -p engine firewall_two_sessions` — run two sessions with different camera paths/budgets over the same region; payloads of every common quad and their compatibility signatures are byte-identical (REQ-PAY-063).

## Notes
- Calibrations (R-71) this task proposes: REQ-SCHED-074.
- REQ-SCHED-074 is an R-71 calibration: the PR carries the proposed level count and its evidence; the human confirms
  it at the M5 gate and it is recorded in `decisions.md`.
- R-133: REQ-SCHED-045 compares no layout, so no artboard is needed; its verify method is a property test.
