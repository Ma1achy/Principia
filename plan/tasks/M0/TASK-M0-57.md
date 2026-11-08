# TASK-M0-57 — The DX12 opt-in: a cargo feature and `PRIN_GPU_BACKEND=dx12`, off by default, local testing only (R-862)

- **Milestone:** M0
- **Closes:** REQ-SYS-097
- **Depends on:** TASK-M0-04
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~200 lines

## Goal
The owner's DX12 order of 4 Nov 2026, ported as R-862: DX12 as "a small, opt-in change: behind a cargo feature and
PRIN_GPU_BACKEND=dx12, off by default", documented as "local testing only, untested in CI", in no required or hosted CI
job (R-711 item 2). The reason, the owner's: "Chrome on Windows runs WebGPU on D3D12, so native DX12 runs preview web-v1
behaviour on Windows." A default build is unchanged; a build with the feature, on Windows, can run the GPU suites on
DX12 by setting the variable. It is what the runner's DX12 run, Phase 2, needs (TASK-M0-58).

## References
- `decisions.md` § "R-862 — DX12 is an opt-in, off by default: a cargo feature and `PRIN_GPU_BACKEND=dx12`, documented "local testing only, untested in CI", in no required or hosted CI job; native DX12 previews web-v1's behaviour on Windows, where Chrome runs WebGPU on D3D12 *(amends R-169 and R-206)*"
- `decisions.md` § "R-206 — An unset `PRIN_GPU_BACKEND` defaults by platform"
- `decisions.md` § "R-169 — The GPU CI jobs, and an install step for every toolchain *(closes G1, H5)*"
- `decisions.md` § "R-711 — One measurement-only self-hosted runner, `malachys-pc-rtx2080s`, on the owner's PC, with no pull-request trigger on any self-hosted job; DX12 may run on it as a measurement; PC benchmarks stay apart from the Mac's performance gates; a recorded run on it can serve Paper 2's non-Metal parity evidence; its jobs have their own workflow; the signing rules *(closes RQ-695; amends R-174, R-186 and R-656)*"

## Deliverables
- A cargo feature, `dx12`, off by default, in every crate that selects a wgpu backend from `PRIN_GPU_BACKEND`
  (`validation`'s `GpuHarness`, `gui`'s mock canvas), and in each crate that builds wgpu (`engine`, `gui`,
  `validation`), adding wgpu's DX12 backend there only under the feature, at the version and backends those crates
  share.
- The selection: with the feature, `PRIN_GPU_BACKEND=dx12` selects DX12 and logs it, as R-206 has every backend
  logged. Without it, `dx12` is refused as an error naming the variable and the feature, never quietly another backend.
  An unset value never selects DX12: metal on macOS, vulkan elsewhere (R-206). Every other value reads as before.
- The documentation, worded "local testing only, untested in CI": on the feature in each `Cargo.toml`, and where the
  harness documents `PRIN_GPU_BACKEND`'s values.
- `xtask/tests/ci_self_hosted.rs`: `dx12_in_no_required_or_hosted_job`, no job of any workflow but R-711's runner's
  named measurements enabling the feature (by `--features`, `-F` or `--all-features`) or setting
  `PRIN_GPU_BACKEND=dx12`, with its controls.

## Acceptance tests
- `cargo test -p validation gpu_backend_env` — default features: `dx12` fails naming the variable and the feature;
  unset selects metal on macOS and vulkan elsewhere; `metal` and `vulkan` as before (REQ-SYS-097).
- `cargo test -p validation --features dx12 gpu_backend_env` — with the feature: `dx12` selects DX12; unset still
  never DX12 (REQ-SYS-097).
- `cargo test -p xtask --test ci_self_hosted` — `dx12_in_no_required_or_hosted_job` passes on the workflows as
  committed (REQ-SYS-097).
- `cargo test -p validation -p xtask --features validation/controls,xtask/controls` — the negative controls: the
  feature's selection removed, `dx12` accepted without the feature, and a hosted job setting `PRIN_GPU_BACKEND=dx12`,
  each failing (R-176).
- On a Windows machine, by hand, never as a CI job: `PRIN_GPU_BACKEND=dx12 cargo test -p validation --features dx12
  gpu_harness`, its log naming the DX12 adapter, the output posted to the PR (REQ-SYS-097).

## Notes
- No default, physics, tolerance or required check changes (R-862: "a small, opt-in change"). A default build is
  byte-for-byte the build it was: the merged tests that refuse `dx12` hold unchanged, since `dx12` stays an error
  without the feature.
- "In no CI job" reads "in no required or hosted CI job" (R-711 item 2). The runner's DX12 run is TASK-M0-58's, a named
  measurement, never a required check (R-859).
- `xtask`'s dependency check resolves `--all-features` (R-194); confirm in the PR that it builds nothing DX12 on a
  non-Windows host, wgpu's DX12 backend being Windows-only, or say what it builds.
- A failure seen only on DX12 on the owner's card is handled as R-719 says, the same card's Vulkan run its second
  discriminator.
