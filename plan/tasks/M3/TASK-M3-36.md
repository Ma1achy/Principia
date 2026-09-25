# TASK-M3-36 — Cross-implementation checks: BodyPlane bit-for-bit and the change-10 re-run

- **Milestone:** M3
- **Closes:** REQ-VAL-028, REQ-VAL-119, REQ-VAL-036, REQ-VAL-035
- **Depends on:** TASK-M3-24, TASK-M3-07
- **Needs (earlier milestones):** REQ-CHART-015, REQ-CHART-028, REQ-CHART-029
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~350 lines

## Goal
The BodyPlane chart (today's slice) is kept and reproduces bit-for-bit against its recorded reference with the Python cross-check green; chart_reference §5.1 defines its Φ and names that reference and cross-check (R-72). The change-10 cross-checks are re-run with the patched NumPy reference and the divergence-vs-horizon table regenerated or retired (R-35), and any figure quoted past the predictability horizon cites its precision or shadowing evidence.

## References
- `docs/design/principia_chart_reference.md` § "5.1 One trait, one dispatch"
- `docs/design/principia_chart_reference.md` § "5.2 Tests that can fail"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_predictability_horizon.md` § "7. Corrections from external replication (report 4)"
- `decisions.md` § "R-35 — The change-10 cross-checks are re-run and the NumPy reference patched *(IE-7)*"
- `docs/design/principia_dd_validation_orbits.md` § "0.1 The mechanism, and the fix — measured, not proposed"
- `docs/design/principia_dd_validation_orbits.md` § "5. Immediate action"
- `open-questions.md` § "Open questions"
- `docs/design/principia_dd_predictability_horizon.md` § "5.1 Retrospective: which existing measurements are inside the horizon"
- `docs/design/principia_dd_predictability_horizon.md` § "7.6 Escape statistics are shadowing-robust to `t=80`, but `t=240` is untested"
- `docs/design/principia_dd_predictability_horizon.md` § "7.2 `lambda` is 0.6–0.8, not 1 — and `t` is not e-foldings"
- `decisions.md` § "R-119 — `t_max(f32)` is the GPU measurement *(closes RQ-87)*"

## Deliverables
- Doc change: `docs/design/principia_chart_reference.md` §5.1 — BodyPlane's Φ, the recorded reference dump and the Python cross-check (REQ-VAL-119).
- `crates/kernel` chart `BodyPlane` (a Φ only) and `xtask golden bodyplane` against `fixtures/golden/bodyplane/`.
- `xtask gate change10-rerun` driving `workbench/tb_az.py` with the overshoot patch against the Rust AZ; the regenerated (or retired) table in `docs/design/principia_dd_predictability_horizon.md`.

## Acceptance tests
- `cargo xtask golden bodyplane` — the BodyPlane render/dump is bitwise-identical to the recorded reference; the Python cross-check passes (REQ-VAL-028).
- chart_reference §5.1 gives BodyPlane's Φ and names the recorded reference dump and Python cross-check it matches; physics reviewer approved (REQ-VAL-119).
- `cargo xtask gate change10-rerun` — re-run outputs recorded; the divergence-vs-horizon table regenerated or marked retired; the f64 horizon figure and the measurement method REQ-VAL-071 applies to the GPU kernel are recorded (R-119) (REQ-VAL-036).
- Review (physics): each quoted long-t figure cites its precision or shadowing evidence; escape statistics at t = 240 stay labelled indicative (REQ-VAL-035).

## Notes
- Gap: BodyPlane's map, its recorded reference dump and the Python cross-check are prin-rs artefacts not in this repository.
- Definitions written here (R-72; physics reviewer approves before merge): REQ-VAL-119.
- RQ-87 ruled: R-119 — the change-10 re-run supplies the f64 figure and the method; `t_max(f32)` itself is REQ-VAL-071's GPU measurement (M4), which REQ-VAL-070's gate reads.
- Waits on RQ-103 (`REVIEW_QUEUE.md`): The prin-rs fixtures and slices the M3 re-runs need.
