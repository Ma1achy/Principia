# TASK-M4-18 — The permanent parity suite in CI: trajectories, Tier S and the label-flip report

- **Milestone:** M4
- **Closes:** REQ-VAL-057, REQ-VAL-062, REQ-VAL-079, REQ-VAL-056
- **Depends on:** TASK-M4-03, TASK-M4-04, TASK-M4-06, TASK-M3-27, TASK-M3-35
- **Needs (earlier milestones):** REQ-VAL-026, REQ-VAL-034, REQ-VAL-050, REQ-SYS-017
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-10, PIT-9
- **Size:** ~400 lines

## Goal
The parity suite becomes permanent: a native in-process `#[test]` in CI on every commit that instantiates the CPU-f64 kernel, dispatches the GPU build via native `wgpu`, reads back `SimState` and diffs it — branch decisions bit-exact on identical per-step inputs, continuous words within envelope, and labels on chaotic trajectories reported, not failed. Full trajectories are compared structurally only (Tier S): outcome class only on fixtures marked non-chaotic, `t_end` in a window, consistent divergence-onset time. A label-flip report over full trajectories on two backends states its domain. Native parity is green before any browser work begins.

## References
- `docs/contracts/principia_canonical_spec.md` § "2. The determinism law (the other defining decision)"
- `docs/contracts/principia_canonical_spec.md` § "7. Precision & validation model *(authoritative: `parity_contract`, `validation_ground_truth_note`, `core_design`)*"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_core_design.md` § "1. Physics defined once, compiled twice — now *structurally*, not by discipline"
- `docs/contracts/principia_parity_contract.md` § "6. The harness"
- `docs/notes/principia_gpu_determinism_note.md` § "The parity consequence (what shared source does and does not buy)"
- `decisions.md` § "R-84 — Branch decisions across precisions *(closes RQ-35)*"
- `docs/contracts/principia_parity_contract.md` § "Tier S — structural only (asserting the pointwise gap would assert a falsehood)"
- `docs/contracts/principia_parity_contract.md` § "3. The load-bearing discipline: never accumulate before comparing"
- `docs/notes/principia_gpu_determinism_note.md` § "Why branches are special (continuous divergence is fine; branch divergence is not)"
- `docs/read_first/principia_01_pitfalls.md` § "10. GENERALISING A STATELESS RESULT TO A TRAJECTORY"
- `docs/contracts/principia_canonical_spec.md` § "1. The substrate (the defining decision)"
- `docs/contracts/principia_parity_contract.md` § "2. The three tiers"
- `docs/contracts/principia_canonical_spec.md` § "11. Still open / downstream (not yet fully in the corpus)"

## Deliverables
- `crates/validation/tests/parity_suite.rs`: the full CPU-vs-GPU `SimState` diff, per field tagged with its tier.
- Fixture set with the non-chaotic / chaotic marking (`fixtures/gates/parity_fixtures`).
- `xtask` gate `label-flip-report`: full-trajectory run on two GPU backends, the flip rate and, per flip, the stateless agreement of the branch operation; report header states the domain.
- CI: the parity job on every commit; the browser build (M8) depends on it.

## Acceptance tests
- `cargo test -p validation parity_suite` — `cargo test` runs the CPU-vs-GPU `SimState` diff; branch decisions exact on shared per-step inputs; continuous words within the windowed tolerance; chaotic-trajectory label flips are reported, not asserted equal (REQ-VAL-057).
- `cargo test -p validation tier_s_structural` plus QA review — no pointwise end-state assertion exists for chaotic pixels; outcome-class equality is asserted only on fixtures marked non-chaotic (REQ-VAL-062).
- `cargo xtask gate label-flip-report` — parity run over full trajectories on two backends; the label-flip rate and, for each flip, that the branch operation itself agreed on identical inputs (705 boundary states, 0 forks); the report header states the domain (fixed inputs vs trajectory) (REQ-VAL-079).
- Code reviewer on the CI config — the native parity/determinism test is green and gating before browser work (REQ-VAL-056).

## Notes
- The non-Metal parity run gates Paper 2, not the build (R-58); if only Metal is available the report says so.
- Waits on RQ-79 (`REVIEW_QUEUE.md`): CI frequency, GPU hardware and browsers the corpus doesn't schedule.
