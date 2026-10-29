# TASK-M6-08 — The decoder switchover and AT_F32_FLOOR: DECODE_MODE, collapse detection, the linear-path uniform

- **Milestone:** M6
- **Closes:** REQ-DEC-033, REQ-DEC-037, REQ-SCHED-062, REQ-PAY-069, REQ-SYS-038, REQ-TOOL-159
- **Depends on:** TASK-M6-01, TASK-M6-02, TASK-M6-07, TASK-M4-07
- **Needs (earlier milestones):** REQ-PAY-064, REQ-SCHED-024, REQ-SYS-024, REQ-PERF-077, REQ-TOOL-158, REQ-DEC-051
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-9
- **Size:** ~430 lines

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
- `decisions.md` § "R-395 — REQ-TOOL-019's "no banding" holds with absolute coordinates up to ℓ_switch, checked at the M1 gate, and through the per-quad local coordinates beyond it, at M5 and M6 *(closes RQ-242)*"
- `decisions.md` § "R-599 — Collapse is judged per axis: an axis along which the field is saturated and the samples are identical is genuinely flat and does not count; a quad stops only when every axis along which the field varies has collapsed, or at the screen floor; the readout names the saturated axis *(closes RQ-554; refines R-90's terminal stop)*"
- `decisions.md` § "R-632 — On the f32 GPU path the linearised decoder's floor is the measured value, quoted with `|x₀|`, measured by a gate and taken as a calibration; the ensemble ratio censors an unparted start at `T` and compares the median of measured over predicted with 1; column 8 is never asserted; the Cartesian drift latches take `dE_reg`'s encoding; the plan's three corrections are applied *(closes RQ-570, RQ-575, RQ-578, RQ-581, RQ-588, RQ-589 and RQ-590)*"

## Deliverables
- `crates/engine/src/deep/switchover.rs`: bitwise IC comparison of adjacent samples (not the energy-drift diagnostic), `quad.collapsed`, the `ℓ_switch = 20` bound shared with lowering's `SWITCH` constant, the switch/stop response keyed off `DECODE_MODE`.
- `crates/engine/src/dispatch/linear_uniform.rs`: the linear-path bind group with the `x₀`/`J_D` uniform; `QuadRequest` unchanged (no `x₀`/`J_D` fields).
- `crates/kernel`: a workgroup-uniform branch on `DECODE_MODE` between full and linear decode.
- Events `DECODE_SWITCHOVER` and `AT_F32_FLOOR` emitted to the scheduler's event stream (consumed by TASK-M6-21).
- `cargo xtask bench decode-mode-occupancy` (two-path branch vs baked linear variant).

## Acceptance tests
- `cargo test -p engine collapse_triggers_switchover` — force bitwise-identical adjacent ICs on the full decoder (depth ~22, and at shallow depth with tiny q₁,q₂); assert DECODE_MODE flips to the linearised path and the quad keeps splitting (REQ-DEC-033).
- `cargo test -p engine switchover_trigger` — quads at depth 20/21 with distinct adjacent ICs carry FULL/LIN; a shallow tiny-q quad whose adjacent ICs are bitwise identical switches early; collapse is detected by bitwise IC comparison, not by the energy-drift diagnostic; lowering's SWITCH constant equals ℓ_switch (REQ-DEC-037).
- `cargo test -p engine at_f32_floor_terminal` — force collapse with DECODE_MODE = 1: the quad is marked terminal (AT_F32_FLOOR) and never re-queued; the same symptom with DECODE_MODE = 0 switches instead; a non-collapsed quad at depth 21 decodes linearised and at depth 15 decodes full; a band quad at `10⁴·ê_k` on the latent chart switches and keeps refining along its unsaturated axis to the screen floor, never stopped at `AT_F32_FLOOR` above it (control: a collapse judged over both axes together stops it and fails), and a quad saturated along both in-plane axes stops (control: a rule that never stops it fails) (R-599) (REQ-SCHED-062).
- `cargo test -p engine linear_uniform_binding` — the QuadRequest struct has no x₀/J_D fields; the linear-path bind group binds the uniform and the full-decoder bind group does not (REQ-PAY-069).
- `cargo xtask bench decode-mode-occupancy` — measure deep-quad occupancy/register pressure with the two-path branch vs a baked linearised variant; record the result (REQ-SYS-038).
- `cargo xtask gate linear-decode-switchover` — at the switchover depth, linear vs full decode agree to O(h²) (REQ-DEC-036's check, moved here by R-154) (REQ-DEC-037).
- `cargo test -p engine past_switch_routing` — R-395's routing: quads at depths 21 and 30 with distinct adjacent ICs (no collapse) are dispatched with DECODE_MODE = LIN and their sample positions come from c and δ only; a quad forced onto the full path at depth 21 fails the check; a quad at depth 20 or shallower may take the full path (REQ-TOOL-159).

## Notes
- Which `Decision` variant records `AT_F32_FLOOR` (and which the integration floor) is not stated by refinement_policy §6 — `Collapsed` reads as the candidate but is not named for it; see Gaps.
- If the benchmark shows the dead full-decode path hurts deep-quad occupancy, REQ-SYS-038 makes decode mode a baked variant; that is recorded in the PR, not decided by the implementer.
- RQ-99 ruled: R-113, option (a) — REQ-DEC-036 (x₀ and J_D) is built at M5 (TASK-M5-04); the switchover (REQ-DEC-033/037) stays here.
- R-154: REQ-DEC-036's check at the actual switchover depth joins REQ-DEC-037 here; M5 verified it over a depth sweep (TASK-M5-04).
- R-395 (RQ-242): beyond ℓ_switch REQ-TOOL-019's "no banding" holds through the per-quad local coordinates. TASK-M5-04
  builds that path (REQ-TOOL-158); this task, which builds `DECODE_MODE`'s switchover, checks that every quad past
  ℓ_switch is routed onto it (REQ-TOOL-159; the human's "M5/M6", applied per R-369).
- **R-537 EX8 (20 Oct 2026):** in a repeat the zoom is on the canonical quad, and far from the root the f32 centre is taken relative to the root cell's own origin; TASK-M8-57 builds that on this task's path (REQ-TOOL-182).
- **R-599 (25 Oct 2026, RQ-554 (c)):** collapse is judged per in-plane axis: a saturated axis, its samples identical, is genuinely flat and does not count, so a band quad keeps refining along its unsaturated axis to the screen floor, and a quad stops at `AT_F32_FLOOR` only when every axis along which the field varies has collapsed; the switchover is unchanged; the readout's naming of the saturated axis is TASK-M8-57's (REQ-GUI-397). *Was:* "**RQ-554 (R-570's port, perf review 5624936602 of PR #279, P2):** a band quad on a latent or flat chart, saturated along one in-plane axis only, collapses, switches and stops terminally at `AT_F32_FLOOR` above the screen floor while its other axis stays structured; whether it keeps refining along that axis is open. REQ-SCHED-062's terminal stop is built as written meanwhile; only its behaviour on band quads waits (`rq: RQ-554`)."
- **R-632 (27 Oct 2026, RQ-578):** on the f32 GPU path the linear path's collapse, where `AT_F32_FLOOR` fires, is at the measured floor (REQ-DEC-051, TASK-M5-04), quoted with `|x₀|`, not ~depth 50. `past_switch_routing`'s quads at depths 21 and 30 with distinct adjacent ICs, and every fixture here that needs distinct ICs past the switchover, use coordinates where the measured floor admits them (REQ-TOOL-159). RQ-578 put no hold on this task.
