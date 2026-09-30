# TASK-M0-43 — Fragment output quantised in the shader; one golden reference across backends

- **Milestone:** M0
- **Closes:** REQ-VAL-176
- **Depends on:** TASK-M0-06
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~150 lines

## Goal
R-269's measurement (run 36719287172) found lavapipe and Metal one step apart on exact half-way values: Metal rounds
each tie up, lavapipe to even. Each backend is byte-stable against itself. So each golden case keeps one reference
per backend, and the runner compares a render only with the reference for the backend it rendered on (R-269).
R-287 amends that: fragment output quantises explicitly in the shader (round half to even, then store), so every
backend writes identical bytes and each golden case keeps one reference across backends. Per-backend references stay
the fallback for any case whose bytes still differ.

## References
- `decisions.md` § "R-269 — REQ-VAL-138 across backends: measure lavapipe, then zero steps or one reference per backend"
- `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"
- `decisions.md` § "R-287 — Fragment output quantises in the shader, so goldens share one reference across backends *(amends R-269)*"
- `docs/contracts/principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality"
- `docs/contracts/principia_parity_contract.md` § "6. The harness"

## Deliverables
- The golden path's fragment output (the runner's shaders and the M1 renderer's output stage, wherever they write an `Rgba8Unorm` target) quantises each channel in the shader: scale to 0..255, round half to even, then store the exact level, so the backend's float-to-unorm conversion has no tie to break (R-287).
- `xtask/src/golden.rs` (fallback, R-287): references are stored per backend (for example `<case>/<backend>.png`); the runner names the backend it rendered on, compares with that backend's reference, and fails naming the backend when that reference is missing. REQ-VAL-138's tolerance (max step 0) applies per backend.
- The self-test cases gain their Vulkan (lavapipe) references beside their Metal ones.
- R-269's scratch fragment as a fixture: with in-shader quantisation, Metal and lavapipe write identical bytes (max step 0) against one reference; without it (the control), they differ by one step.
- Negative controls for this task's tests (R-176, R-199).

## Acceptance tests
- CI log on the PR head: the R-269 fixture renders identical bytes in the `gpu-metal` and `gpu-lavapipe` jobs (max step 0 against one reference), and its control (the backend's automatic conversion) shows max step 1 (REQ-VAL-176).
- `cargo test -p xtask golden` — the fallback: a case with Metal and Vulkan references passes on each backend against its own and fails against the other's; a missing reference for the running backend fails naming the backend (REQ-VAL-176).
- `cargo xtask golden --all` passes in the `gpu-metal` and `gpu-lavapipe` jobs against one reference per case (REQ-VAL-176).

## Notes
- The measurement's raw renders are in PR history only; the branch was deleted (R-272). Re-render the fixture in the task.
