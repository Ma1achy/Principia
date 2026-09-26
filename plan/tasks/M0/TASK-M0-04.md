# TASK-M0-04 — The test harness: native wgpu self-test dispatch, proptest and negative controls

- **Milestone:** M0
- **Closes:** REQ-VAL-007, REQ-SYS-065
- **Depends on:** TASK-M0-01
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~400 lines

## Goal
`crates/validation` carries the shared test harness. A native in-process `wgpu` harness opens a headless device (requesting no optional features), compiles a WGSL compute module, dispatches it over storage buffers and reads the result back in the same process (parity_contract §6). A shared proptest configuration records its seed on failure. And a negative-control registry: every test registers the discriminating control that must make it fail — a mutation, a contaminated input, a sign-flipped variant or a comparison that must differ — and `cargo xtask controls` runs every control and fails if any control passes or any test has none, so no test in the suite is one that cannot fail (philosophy §4.4; pitfalls §9's general form).

## References
- `docs/contracts/principia_parity_contract.md` § "6. The harness"
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `docs/read_first/principia_01_pitfalls.md` § "3. Standing rules earned in this sequence"
- `docs/read_first/principia_01_pitfalls.md` § "9. A PARITY CHECK THAT MASKS THE BITS THE FORK LANDS IN"
- `decisions.md` § "R-85 — Native wgpu sets the Tier-N tolerances *(closes RQ-36)*"
- `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"
- `decisions.md` § "R-169 — The GPU CI jobs, and an install step for every toolchain *(closes G1, H5)*"
- `decisions.md` § "R-174 — The self-hosted runner runs only this repository's code *(closes H1)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"

## Deliverables
- `crates/validation/src/gpu.rs` — `GpuHarness::new()` (headless, no surface, no optional features; the backend is read from `PRIN_GPU_BACKEND=metal|vulkan`, and an unset or unknown value is an error naming the variable, R-169), `run_wgsl(module, entry, inputs) -> Vec<u32>`, and the adapter info (name, backend, driver) exposed for TASK-M0-19's session header.
- `crates/validation/src/control.rs` — the negative-control registry (`negative_control!(test, description, control)`) and a `controls` feature under which each control runs its test against the control input. A test and its control are matched by a shared test-name attribute (`#[control_for = "<test name>"]`); crates reach the macro through a dev-dependency on `crates/validation` (R-176).
- `crates/validation/src/prop.rs` — the shared proptest config (case count, seed printed on failure).
- `xtask/src/controls.rs` — `cargo xtask controls`: lists the workspace's tests, runs `cargo test --features controls` in each crate that declares the feature (a crate without it is skipped and reported, not failed, R-176), and fails when a control passes or a test in a controls crate has no control; registered in `cargo xtask ci`.
- Controls for TASK-M0-01's `deps` tests, the only tests merged before this task (TASK-M0-02 and TASK-M0-03 now depend on this one and register their own, R-176).
- `.github/workflows/ci.yml` — two GPU jobs (R-169): `gpu-metal` on `runs-on: [self-hosted, macOS, ARM64]` with `PRIN_GPU_BACKEND=metal`, and `gpu-lavapipe` on `ubuntu-latest` with `sudo apt-get install -y mesa-vulkan-drivers` and `PRIN_GPU_BACKEND=vulkan`. Both run `cargo test -p validation gpu_harness`. The self-hosted job runs only for pushes and for PRs from this repository: `if: github.event_name == 'push' || github.event.pull_request.head.repo.full_name == github.repository`; fork PRs get the CPU and lavapipe jobs only (R-174).
- Harness self-tests: a WGSL identity kernel; a WGSL kernel reading a top-bit-set word with the i32 and the u32 `extractBits` overloads.

## Acceptance tests
- `cargo test -p validation gpu_harness` — a WGSL identity dispatch round-trips 2¹⁶ u32 words bit-exact on native in-process wgpu, in CI on the self-hosted Metal runner and on lavapipe (R-110).
- `cargo test -p validation gpu_harness_can_fire` — on words with bit 31 set, the i32 `extractBits` dispatch differs from the u32 one: the harness's reachable output includes the sign-extension failure.
- CI log on the PR head: `gpu-metal` and `gpu-lavapipe` each run `gpu_harness` green, and the adapter info each prints names the Metal and the Vulkan (llvmpipe/lavapipe) backend; review checklist (code): the `gpu-metal` job carries the R-174 `if:` guard (REQ-SYS-065).
- `cargo test -p validation gpu_backend_env` — `PRIN_GPU_BACKEND` unset or `dx12` fails naming the variable; `vulkan` and `metal` select that backend (REQ-SYS-065).
- `cargo xtask controls` — every test in the workspace has a registered negative control and every control makes its test fail; a fixture test with no control, and one whose control passes, each fail the command (REQ-VAL-007).
- Review checklist (qa §3): no test is arithmetically impossible or true by construction (the n_hot < N² quantile case, a distinct-value count bounded below the claimed effect) (REQ-VAL-007).

## Notes
- Every later task's tests register their controls here; the qa reviewer checks each control is discriminating (qa §3).
- R-110 (RQ-79): the harness's CI adapters are a self-hosted Apple-silicon runner (Metal) and lavapipe, both on every commit; `GpuHarness` selects one from `PRIN_GPU_BACKEND` (R-169). The runner itself is registered by the human under its own macOS user account (R-174; `plan/HUMAN_SETUP.md`).
- The harness lives in `crates/validation` ("the validation harness" in the plan layout) so that the codegen self-test and, from M4, the native parity suite share one device path.
