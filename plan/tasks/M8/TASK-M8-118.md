# TASK-M8-118 — Hold L_z on the GPU: the outcome tags across backends and the calibration of τ (R-916)

- **Milestone:** M8
- **Closes:** REQ-CHART-126, REQ-CHART-179
- **Depends on:** TASK-M2-69, TASK-M4-06
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
Hold `L_z`'s GPU side: under RQ-715's option (i), the outcome tags bit-identical on Metal and lavapipe beside TASK-M2-69's CPU, and the calibration of `τ`, provisional at M2 (R-919, F13 (b)).

## References
- `docs/design/principia_chart_composition.md` § "5.3 Constraints: hold `E`, and new: hold `L_z`"
- `docs/design/principia_chart_composition.md` § "5.4 What the constraints do to a momentum axis"
- `docs/design/principia_chart_composition.md` § "6.1 Refusals: genuine contradictions only"
- `docs/design/principia_chart_composition.md` § "12. Tests that can fail"
- `docs/design/principia_chart_reference.md` § "0.6 Energy normalisation — optional, and forbidden on some charts"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 6 — The canonical inverse policy, completed"

## Deliverables
- Under RQ-715's option (i): `kernel::decode::invariant::rigid`, the bench `hold-rigid-tau`, and the calibration proposal for `τ` (REQ-CHART-126).
- Under RQ-715's option (i): `crates/engine/tests/hold_cross_backend.rs`, package `engine`, with the tests `hold_outcomes_cross_backend`, `hold_outcomes`' GPU arm, and `hold_rigid_tau_tags`, the bench's lavapipe leg; a step running that binary in both the `gpu-metal suites` and `gpu-lavapipe suites` jobs of `.github/workflows/ci.yml`, since those jobs run only the selectors named in their steps; and its entries in `.config/nextest.toml`, as `engine::decode_cross_backend` has.

## Acceptance tests
- `cargo nextest run -p engine --test hold_cross_backend hold_outcomes_cross_backend` and hold_rigid_tau_tags, in CI's gpu-metal suites and gpu-lavapipe suites jobs — the tags bit-identical across backends; control: a tag from s = 0 and K̂ = 0 differs on the rigid draws (REQ-CHART-179).
- `cargo xtask bench hold-rigid-tau` — under RQ-715's option (i), against a criterion written before measuring (R-649), the proposal shows τ at masses log-uniform to 10⁻³, on the two GPU backends, and the kernel routine's cost. The Metal leg and the cost run on the human's Mac and wait for that run (R-186); the lavapipe leg is a tag measurement, no timing, and runs as `hold_rigid_tau_tags` in that same step of CI's `gpu-lavapipe suites` job (`PRIN_GPU_BACKEND=vulkan`, R-169), where the plan sets lavapipe up. A reviewer checks it, the human confirms it at the M8 gate (REQ-CHART-126). Under option (ii), REQ-CHART-126 retires.

## Notes
- **Build order (R-863):** no stage of the owner's order names it, so no stage edge is made; where it goes is RQ-744 (C).
- RQ-722 is for the rest charts. RQ-715 closes only the held-L_z half of RQ-701.
- Free charts take no tilt target on L*; a chart holding L_z by construction moves by value (R-720).
- Waits on its rulings item: RQ-715, RQ-722. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); R-917's hold holds it, no longer an edge: R-916 took TASK-M2-58 (was TASK-M4-37), the rulings applied, out of Depends on, and no M3 to M9 task starts until M2's gate passes, which waits on TASK-M2-58 as an M2 task. *Was:* "TASK-M4-37 in Depends on holds it."
- *8 Nov 2026 (R-873):* **Build order:** stage 3 of R-863's order, after stage 2 (TASK-M2-65), placed by R-873 (RQ-744 (A) (a) and (C)); TASK-M2-81 and TASK-M8-108, stage 4, depend on it. The note above that no stage edge is made is overtaken. *Was:* "- **Depends on:** TASK-M2-58, TASK-M4-33, TASK-M2-37, TASK-M2-52"
- *9 Nov 2026 (R-916):* the chart core is M2's (R-916, R-917, R-919), so this header follows plan/tasks.yaml. *Was:* "Closes: REQ-CHART-102, REQ-CHART-111, REQ-CHART-126"; "Depends on: TASK-M2-58, TASK-M4-33, TASK-M2-37, TASK-M2-52, TASK-M2-65".
- *9 Nov 2026 (R-916):* split: hold `L_z` on the CPU (REQ-CHART-102, REQ-CHART-111) is TASK-M2-69's; this task keeps `τ` (REQ-CHART-126) and the GPU tags (REQ-CHART-179). `τ` is read provisionally at M2 and confirmed here (R-919, F13 (b)). *Was (title):* "Hold L_z on any chart, beside energy normalisation"; *was (Acceptance):* the `hold_lz`/`hold_outcomes`/`hold_encode`/`constraint_axis_refused` line and the `hold_rank` line, now at TASK-M2-69.
