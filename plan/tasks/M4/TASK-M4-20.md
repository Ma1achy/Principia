# TASK-M4-20 — Compute fast-math on against off: the march's speed, and the differences measured and reported

- **Milestone:** M4
- **Closes:** REQ-PERF-094, REQ-VAL-177
- **Depends on:** TASK-M4-19, TASK-M4-08, TASK-M0-44
- **Needs (earlier milestones):** REQ-SYS-074, REQ-TOOL-141, REQ-TOOL-001
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-3, PIT-10
- **Size:** ~300 lines

## Goal
Compute fast-math is off by default and on only as an opt-in optimisation for speed; with it on, parity is measured,
not exact, and the differences between on and off are measured and reported (R-297). This task gives both halves of
that choice their numbers. The benchmark times the march with the setting off and on, on the human's Mac (R-186), so
turning it on is an informed choice. The difference report runs the same fixture survey both ways and reports what
changes: branch-word forks per step, the largest continuous-word differences and the outcome-class fractions. Its
control, off against off, reports exactly zero.

## References
- `decisions.md` § "R-297 — Fast-math per shader stage: off for compute by default, an explicit and recorded opt-in; display may keep it *(amends R-84, R-116)*"
- `decisions.md` § "R-84 — Branch decisions across precisions *(closes RQ-35)*"
- `decisions.md` § "R-186 — GitHub-hosted runners first; no self-hosted runner *(amends R-110, R-169, R-174)*"
- `docs/contracts/principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality"
- `docs/contracts/principia_parity_contract.md` § "Tier B — integer-exact *given the same branch decisions* (integer & packed fields)"
- `docs/contracts/principia_parity_contract.md` § "5. The aggregate-survey agreement test (what certifies "the picture is the same")"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "1.1 The fixed suite — comparability across devices"
- `docs/notes/principia_gpu_determinism_note.md` § "Why branches are special (continuous divergence is fine; branch divergence is not)"

## Deliverables
- `xtask` bench `fast-math`: the march kernel at production dispatch granularity on one fixture survey, with the
  setting off and on, each writing a profiler schema v1 file whose header records the compute mode (REQ-TOOL-141).
- `cargo xtask gate fast-math-diff`, in the `gpu-metal` job: the same fixture survey marched with the setting off and
  on, and a report, written into the gate report, of the branch-word forks per step (`N_sub`, `state`, terminals), the
  largest difference per continuous word and the outcome-class fractions; the off-against-off control.
- Negative controls for this task's tests (R-176).

## Acceptance tests
- `cargo xtask bench fast-math`, run on the human's Mac (R-186) — frame time and march throughput with the setting off and on, and their ratio, compared through `prin profile diff` and recorded in the PR; the PR waits for that run (REQ-PERF-094).
- `cargo xtask gate fast-math-diff` in the `gpu-metal` job — the report lists, on against off, the branch-word forks per step, the largest difference per continuous word and the outcome-class fractions, and is written into the gate report; off against off reports zero forks and zero differences (the control) (REQ-VAL-177).

## Notes
- The report states differences; it doesn't assert them to be zero, since with fast-math on parity is measured, not
  exact (R-297). Only its control asserts: off against off is exactly zero.
- PIT-10: a per-step fork count is not a trajectory result; the outcome-class fractions are the survey-level view
  (parity contract §5).
