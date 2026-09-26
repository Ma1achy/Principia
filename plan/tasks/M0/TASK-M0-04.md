# TASK-M0-04 — The test harness: native wgpu self-test dispatch and proptest

- **Milestone:** M0
- **Closes:** REQ-SYS-065, REQ-VAL-151
- **Depends on:** TASK-M0-01
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~380 lines

## Goal
`crates/validation` carries the shared test harness. A native in-process `wgpu` harness opens a headless device (requesting no optional features), compiles a WGSL compute module, dispatches it over storage buffers and reads the result back in the same process (parity_contract §6). A shared proptest configuration records its seed on failure. The harness is shown able to fire before anything relies on it: its self-test includes the `extractBits` sign-extension failure of pitfalls §9. The negative-control registry that was once part of this task is TASK-M0-21, and TASK-M0-22 registers this task's controls (R-198).

## References
- `docs/contracts/principia_parity_contract.md` § "6. The harness"
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `docs/read_first/principia_01_pitfalls.md` § "3. Standing rules earned in this sequence"
- `docs/read_first/principia_01_pitfalls.md` § "9. A PARITY CHECK THAT MASKS THE BITS THE FORK LANDS IN"
- `decisions.md` § "R-85 — Native wgpu sets the Tier-N tolerances *(closes RQ-36)*"
- `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"
- `decisions.md` § "R-169 — The GPU CI jobs, and an install step for every toolchain *(closes G1, H5)*"
- `decisions.md` § "R-174 — The self-hosted runner runs only this repository's code *(closes H1)*"
- `decisions.md` § "R-186 — GitHub-hosted runners first; no self-hosted runner *(amends R-110, R-169, R-174)*"
- `decisions.md` § "R-198 — TASK-M0-04 is split into M0-04, M0-21, M0-22 and M0-23 *(closes RQ-135)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-182 — Escape fixtures are defined; proposed tolerances are provisional in CI *(closes T4, T5)*"
- `decisions.md` § "R-203 — The shared proptest case count is a calibration requirement, 256 provisional *(closes RQ-140)*"

## Deliverables
- `crates/validation/src/gpu.rs` — `GpuHarness::new()` (headless, no surface, no optional features; the backend is read from `PRIN_GPU_BACKEND=metal|vulkan`, an unset value defaults to metal on macOS and vulkan elsewhere and an unknown value is an error naming the variable, and the chosen backend is logged, R-169, R-206), `run_wgsl(module, entry, inputs) -> Vec<u32>`, and the adapter info (name, backend, driver) exposed for TASK-M0-19's session header.
- `crates/validation/src/prop.rs` — the shared proptest config (case count, seed printed on failure). The case count is REQ-VAL-151's calibration value: 256 proposed, used provisionally and marked so until the human confirms or changes it at the M0 gate (R-203, R-182).
- `.github/workflows/ci.yml` — two GPU jobs on GitHub-hosted runners (R-169, R-186): `gpu-metal` on `runs-on: macos-15` (Apple silicon, paravirtual Metal) with `PRIN_GPU_BACKEND=metal`, for the Metal correctness suites only, and `gpu-lavapipe` on `ubuntu-latest` with `sudo apt-get install -y mesa-vulkan-drivers` and `PRIN_GPU_BACKEND=vulkan`. Both run `cargo test -p validation gpu_harness`; `gpu-metal` also runs `metal_hosted_probe`. *Dormant (R-186):* R-174's same-repository guard, `if: github.event_name == 'push' || github.event.pull_request.head.repo.full_name == github.repository`, applies to a self-hosted job and is added back only if one is (R-174).
- Harness self-tests: a WGSL identity kernel; a WGSL kernel reading a top-bit-set word with the i32 and the u32 `extractBits` overloads; `metal_hosted_probe`.

## Acceptance tests
- `cargo test -p validation gpu_harness` — a WGSL identity dispatch round-trips 2¹⁶ u32 words bit-exact on native in-process wgpu, in CI on GitHub-hosted `macos-15` (Metal) and on lavapipe (R-110, R-186) (REQ-SYS-065).
- `cargo test -p validation gpu_harness_can_fire` — on words with bit 31 set, the i32 `extractBits` dispatch differs from the u32 one: the harness's reachable output includes the sign-extension failure (pitfalls §9).
- `cargo test -p validation metal_hosted_probe`, the first check on `macos-15` (R-186): wgpu finds an adapter whose backend is Metal, and the harness's M0 fixture, the 2¹⁶-word identity dispatch, round-trips bit-exact. If it fails or is flaky (any failure in 10 consecutive runs), stop and raise a REVIEW_QUEUE entry proposing the self-hosted runner. The agent then scripts its setup, and the human approves it (REQ-SYS-065).
- CI log on the PR head: `gpu-metal` (macos-15) and `gpu-lavapipe` (ubuntu-latest) each run `gpu_harness` green, and the adapter info each prints names the Metal and the Vulkan (llvmpipe/lavapipe) backend; review checklist (code): no job uses a self-hosted runner (REQ-SYS-065).
- `cargo test -p validation gpu_backend_env` — `PRIN_GPU_BACKEND` set to `dx12` (or any unknown value) fails naming the variable; unset selects metal on macOS and vulkan elsewhere; `vulkan` and `metal` select that backend; the chosen backend is logged (R-206) (REQ-SYS-065).
- `cargo test -p validation prop_seed` — a property made to fail prints the seed it failed on, and re-running with that seed fails on the same case.
- `cargo test -p validation prop_seed -- --nocapture` — the shared config runs 256 cases per property, and the output marks the value provisional (R-182); proposal in the PR: 256 with its evidence, confirmed or changed by the human at the M0 gate (R-203) (REQ-VAL-151).

## Notes
- R-198 split the old TASK-M0-04: the control registry and `cargo xtask controls` are TASK-M0-21; controls for every test merged before TASK-M0-22, this task's among them, and `controls` in `cargo xtask ci` are TASK-M0-22 (which closes REQ-VAL-007); R-196's per-PR mutation gate is TASK-M0-23; its nightly run is TASK-M0-19.
- This task's tests register no negative controls, because the registry does not exist yet; TASK-M0-22 registers them (R-198). Each self-test above is still shown able to fire in the PR: `gpu_harness_can_fire` is its own discriminating comparison, and the PR shows each other test going red on a contaminated input (qa §3).
- Reviewers and pitfalls are unchanged by the split: `physics` stays because the `extractBits` self-test is a pitfall §9 regression (PIT-9), and PIT-3 because the harness is shown able to fire before its output is read.
- R-110 as amended by R-186: the harness's CI adapters are GitHub-hosted `macos-15` (Metal) and lavapipe on `ubuntu-latest`, both on every commit; `GpuHarness` selects one from `PRIN_GPU_BACKEND` (R-169). There is no self-hosted runner, and R-174 is dormant.
- The harness lives in `crates/validation` ("the validation harness" in the plan layout) so that the codegen self-test and, from M4, the native parity suite share one device path.
