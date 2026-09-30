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
R-296 amends R-287 (RQ-175's measurement): explicit quantisation makes exact ties identical, but a value within an ulp
of a tie can differ on a backend whose display shaders compile with fast-math (Metal via wgpu), so a golden near a tie
keeps one reference per backend. R-269's half-way fixture is such a case: it keeps one reference per backend, and the
PR names it.

## References
- `decisions.md` § "R-269 — REQ-VAL-138 across backends: measure lavapipe, then zero steps or one reference per backend"
- `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"
- `decisions.md` § "R-287 — Fragment output quantises in the shader, so goldens share one reference across backends *(amends R-269)*"
- `decisions.md` § "R-296 — R-269's half-way fixture keeps one reference per backend; explicit quantisation makes exact ties identical, not values near one *(closes RQ-175; amends R-287)*"
- `docs/contracts/principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality"
- `docs/contracts/principia_parity_contract.md` § "6. The harness"

## Deliverables
- The golden path's fragment output (the runner's shaders and the M1 renderer's output stage, wherever they write an `Rgba8Unorm` target) quantises each channel in the shader: scale to 0..255, round half to even, then store the exact level, so the backend's float-to-unorm conversion has no tie to break (R-287).
- `xtask/src/golden.rs` (fallback, R-287): references are stored per backend (for example `<case>/<backend>.png`); the runner names the backend it rendered on, compares with that backend's reference, and fails naming the backend when that reference is missing. REQ-VAL-138's tolerance (max step 0) applies per backend.
- The self-test cases gain their Vulkan (lavapipe) references beside their Metal ones.
- R-269's scratch fragment as a fixture: with in-shader quantisation, Metal and lavapipe write identical bytes (max step 0) against one reference; without it (the control), they differ by one step.
  R-296 changes this deliverable: the fixture keeps one reference per backend (Metal and Vulkan), each rendered with in-shader quantisation, and the control (the backend's automatic conversion) keeps its per-backend references too.
- Negative controls for this task's tests (R-176, R-199).

## Acceptance tests
- CI log on the PR head: the R-269 fixture, quantised in the shader, renders max step 0 in the `gpu-metal` and `gpu-lavapipe` jobs, each against its own backend's reference (R-296); the evidence that the quantisation works: lavapipe's quantised bytes equal its automatic ones, and at the fixture's exact f32 ties (R × 255 exactly x + 0.5) both backends round to even; its control (the backend's automatic conversion) shows max step 1 between the backends (REQ-VAL-176).
- `cargo test -p xtask golden` — the fallback: a case with Metal and Vulkan references passes on each backend against its own and fails against the other's; a missing reference for the running backend fails naming the backend (REQ-VAL-176).
- `cargo xtask golden --all` passes in the `gpu-metal` and `gpu-lavapipe` jobs against one reference per case, or one per backend for a case the PR names (the R-269 fixture, R-296) (REQ-VAL-176).

## Notes
- The measurement's raw renders are in PR history only; the branch was deleted (R-272). Re-render the fixture in the task.
- RQ-175 ruled: R-296 — option 1, the half-way fixture takes R-287's fallback; parity contract §4 is qualified. The implementation committed locally on `task/TASK-M0-43` (1cd8cc2) changes only the fixture's references and the acceptance wording.
