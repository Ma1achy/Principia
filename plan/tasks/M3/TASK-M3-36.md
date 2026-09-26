# TASK-M3-36 — Cross-implementation checks: BodyPlane bit-for-bit and the change-10 re-run

- **Milestone:** M3
- **Closes:** REQ-VAL-028, REQ-VAL-119, REQ-VAL-036, REQ-VAL-035, REQ-VAL-146
- **Depends on:** TASK-M3-24, TASK-M3-07
- **Needs (earlier milestones):** REQ-CHART-015, REQ-CHART-028, REQ-CHART-029
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~350 lines

## Goal
The BodyPlane chart (today's slice) is kept and reproduces the Python reference's output, recorded here as its fixture: bit-exact where the operation order is identical, otherwise within a calibrated tolerance (R-166), with the Python cross-check green; chart_reference §5.1 defines its Φ and names that reference and cross-check (R-72). The change-10 cross-checks are re-run with the patched NumPy reference and the divergence-vs-horizon table regenerated or retired (R-35), and any figure quoted past the predictability horizon cites its precision or shadowing evidence.

## References
- `docs/design/principia_chart_reference.md` § "5.1 One trait, one dispatch"
- `docs/design/principia_chart_reference.md` § "5.2 Tests that can fail"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_predictability_horizon.md` § "7. Corrections from external replication (report 4)"
- `decisions.md` § "R-35 — The change-10 cross-checks are re-run and the NumPy reference patched *(IE-7)*"
- `docs/design/principia_dd_validation_orbits.md` § "0.1 The mechanism, and the fix — measured, not proposed"
- `docs/design/principia_dd_validation_orbits.md` § "5. Immediate action"
- `open-questions.md` § "Pending-changes register"
- `docs/design/principia_dd_predictability_horizon.md` § "5.1 Retrospective: which existing measurements are inside the horizon"
- `docs/design/principia_dd_predictability_horizon.md` § "7.6 Escape statistics are shadowing-robust to `t=80`, but `t=240` is untested"
- `docs/design/principia_dd_predictability_horizon.md` § "7.2 `lambda` is 0.6–0.8, not 1 — and `t` is not e-foldings"
- `decisions.md` § "R-119 — `t_max(f32)` is the GPU measurement *(closes RQ-87)*"
- `decisions.md` § "R-166 — The fixtures"
- `decisions.md` § "R-159 — The prin-rs reference set is imported *(closes RQ-102 and RQ-103, with R-160 to R-167)*"
- `docs/contracts/principia_parity_contract.md` § "3. The load-bearing discipline: never accumulate before comparing"

## Deliverables
- Doc change: `docs/design/principia_chart_reference.md` §5.1 — BodyPlane's Φ, the recorded reference dump and the Python cross-check (REQ-VAL-119).
- `crates/kernel` chart `BodyPlane` (a Φ only) and `xtask golden bodyplane` against `fixtures/golden/bodyplane/`, the fixture recorded from the Python reference (`docs/reference/prin-rs/tools/xcheck`, `reference/*.py`) (R-166).
- Calibration proposal for the tolerance where the operation order differs (REQ-VAL-146).
- `xtask gate change10-rerun` driving `workbench/tb_az.py` with the overshoot patch against the Rust AZ; the regenerated (or retired) table in `docs/design/principia_dd_predictability_horizon.md`.

## Acceptance tests
- `cargo xtask golden bodyplane` — the BodyPlane render/dump against the fixture recorded from the Python reference: bit-exact where the operation order is identical (parity_contract), otherwise within the calibrated tolerance (REQ-VAL-146); the Python cross-check passes (REQ-VAL-028).
- `cargo xtask golden bodyplane --propose` — the operations whose order differs from the Python reference listed, BodyPlane's difference against the fixture measured and the tolerance stated; the human confirms it at the M3 gate and it is recorded in decisions.md (REQ-VAL-146).
- chart_reference §5.1 gives BodyPlane's Φ and names the reference it matches, the Python reference's output recorded at M3 as a fixture (R-166); physics reviewer approved (REQ-VAL-119).
- `cargo xtask gate change10-rerun` — re-run outputs recorded; the divergence-vs-horizon table regenerated or marked retired; the f64 horizon figure and the measurement method REQ-VAL-071 applies to the GPU kernel are recorded (R-119) (REQ-VAL-036).
- Review (physics): each quoted long-t figure cites its precision or shadowing evidence; escape statistics at t = 240 stay labelled indicative (REQ-VAL-035).

## Notes
- *Was: "Gap: BodyPlane's map, its recorded reference dump and the Python cross-check are prin-rs artefacts not in this repository."* RQ-103 ruled: R-166 (d) — the Python cross-check and its reference are imported (`docs/reference/prin-rs/tools/xcheck`, `reference/*.py`; reference, not authority, R-159) and this task records their output as the fixture.
- Definitions written here (R-72; physics reviewer approves before merge): REQ-VAL-119.
- RQ-87 ruled: R-119 — the change-10 re-run supplies the f64 figure and the method; `t_max(f32)` itself is REQ-VAL-071's GPU measurement (M4), which REQ-VAL-070's gate reads.
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-VAL-146.
