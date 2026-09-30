# TASK-M0-43 — One golden reference per GPU backend

- **Milestone:** M0
- **Closes:** REQ-VAL-176
- **Depends on:** TASK-M0-06
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~120 lines

## Goal
R-269's measurement (run 36719287172) found lavapipe and Metal one step apart on exact half-way values: Metal rounds
each tie up, lavapipe to even. Each backend is byte-stable against itself. So each golden case keeps one reference
per backend, and the runner compares a render only with the reference for the backend it rendered on (R-269).

## References
- `decisions.md` § "R-269 — REQ-VAL-138 across backends: measure lavapipe, then zero steps or one reference per backend"
- `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"
- `docs/contracts/principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality"
- `docs/contracts/principia_parity_contract.md` § "6. The harness"

## Deliverables
- `xtask/src/golden.rs`: references are stored per backend (for example `<case>/<backend>.png`); the runner names the backend it rendered on, compares with that backend's reference, and fails naming the backend when that reference is missing. REQ-VAL-138's tolerance (max step 0) applies per backend.
- The self-test cases gain their Vulkan (lavapipe) references beside their Metal ones.
- A fixture from R-269's scratch fragment that shows a render passes against its own backend's reference and fails against the other's.
- Negative controls for this task's tests (R-176, R-199).

## Acceptance tests
- `cargo test -p xtask golden` — a case with Metal and Vulkan references passes on each backend against its own and fails against the other's (the R-269 fixture); a missing reference for the running backend fails naming the backend (REQ-VAL-176).
- `cargo xtask golden --all` passes in the `gpu-metal` and `gpu-lavapipe` jobs, each against its own references (REQ-VAL-176).

## Notes
- The measurement's raw renders are in PR history only; the branch was deleted (R-272). Re-render the fixture in the task.
