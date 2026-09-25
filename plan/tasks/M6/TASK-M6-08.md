# TASK-M6-08 — The decoder switchover and AT_F32_FLOOR: DECODE_MODE, collapse detection, the linear-path uniform

- **Milestone:** M6
- **Closes:** REQ-DEC-033, REQ-DEC-037, REQ-SCHED-062, REQ-PAY-069, REQ-SYS-038
- **Depends on:** TASK-M6-01, TASK-M6-02, TASK-M6-07, TASK-M4-07
- **Needs (earlier milestones):** REQ-PAY-064, REQ-SCHED-024, REQ-SYS-024, REQ-PERF-077
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-9
- **Size:** ~400 lines

## Goal
The scheduler switches a quad to the linearised decoder (`QuadRequest` `DECODE_MODE`) once `quad.collapsed` is set — the full decoder's adjacent samples give bitwise-identical ICs — or at depth `ℓ_switch = 20` (lowering's `SWITCH`), whichever comes first (R-90); refinement continues. The same symptom on the linear path fires `AT_F32_FLOOR`, a terminal stop cached as a quad fact. `x₀` and `J_D` travel in a separate uniform buffer bound only for linear-path quads. Decode mode is a per-quad workgroup-uniform flag, not a baked variant, unless the occupancy benchmark says otherwise.

## References
- `docs/contracts/principia_scheduler_contract.md` § "Part 4 — Terminal vs refinable, tied to the two floors"
- `docs/design/principia_deep_zoom.md` § "2. Linearised decoder — IC precision"
- `docs/design/principia_deep_zoom.md` § "4. Sliding depth bound"
- `decisions.md` § "R-90 — The decoder switchover trigger *(closes RQ-41)*"
- `docs/contracts/principia_lowering_contract.md` § "Part 5 — The resolution function (the "switch", concretely)"
- `docs/contracts/principia_scheduler_contract.md` § "`QuadRequest.flags`"
- `docs/contracts/principia_lowering_contract.md` § "Compute side"
- `decisions.md` § "R-154 — REQ-DEC-036 is verified over a depth sweep at M5 *(closes RQ-124)*"

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
## Deliverables
- `crates/engine/src/deep/switchover.rs`: bitwise IC comparison of adjacent samples (not the energy-drift diagnostic), `quad.collapsed`, the `ℓ_switch = 20` bound shared with lowering's `SWITCH` constant, the switch/stop response keyed off `DECODE_MODE`.
- `crates/engine/src/dispatch/linear_uniform.rs`: the linear-path bind group with the `x₀`/`J_D` uniform; `QuadRequest` unchanged (no `x₀`/`J_D` fields).
- `crates/kernel`: a workgroup-uniform branch on `DECODE_MODE` between full and linear decode.
- Events `DECODE_SWITCHOVER` and `AT_F32_FLOOR` emitted to the scheduler's event stream (consumed by TASK-M6-21).
- `cargo xtask bench decode-mode-occupancy` (two-path branch vs baked linear variant).

## Acceptance tests
- `cargo test -p engine collapse_triggers_switchover` — force bitwise-identical adjacent ICs on the full decoder (depth ~22, and at shallow depth with tiny q₁,q₂); assert DECODE_MODE flips to the linearised path and the quad keeps splitting (REQ-DEC-033).
- `cargo test -p engine switchover_trigger` — quads at depth 20/21 with distinct adjacent ICs carry FULL/LIN; a shallow tiny-q quad whose adjacent ICs are bitwise identical switches early; collapse is detected by bitwise IC comparison, not by the energy-drift diagnostic; lowering's SWITCH constant equals ℓ_switch (REQ-DEC-037).
- `cargo test -p engine at_f32_floor_terminal` — force collapse with DECODE_MODE = 1: the quad is marked terminal (AT_F32_FLOOR) and never re-queued; the same symptom with DECODE_MODE = 0 switches instead; a non-collapsed quad at depth 21 decodes linearised and at depth 15 decodes full (REQ-SCHED-062).
- `cargo test -p engine linear_uniform_binding` — the QuadRequest struct has no x₀/J_D fields; the linear-path bind group binds the uniform and the full-decoder bind group does not (REQ-PAY-069).
- `cargo xtask bench decode-mode-occupancy` — measure deep-quad occupancy/register pressure with the two-path branch vs a baked linearised variant; record the result (REQ-SYS-038).
- `cargo xtask gate linear-decode-switchover` — at the switchover depth, linear vs full decode agree to O(h²) (REQ-DEC-036's check, moved here by R-154) (REQ-DEC-037).

## Notes
- Which `Decision` variant records `AT_F32_FLOOR` (and which the integration floor) is not stated by refinement_policy §6 — `Collapsed` reads as the candidate but is not named for it; see Gaps.
- If the benchmark shows the dead full-decode path hurts deep-quad occupancy, REQ-SYS-038 makes decode mode a baked variant; that is recorded in the PR, not decided by the implementer.
- RQ-99 ruled: R-113, option (a) — REQ-DEC-036 (x₀ and J_D) is built at M5 (TASK-M5-04); the switchover (REQ-DEC-033/037) stays here.
- R-154: REQ-DEC-036's check at the actual switchover depth joins REQ-DEC-037 here; M5 verified it over a depth sweep (TASK-M5-04).
