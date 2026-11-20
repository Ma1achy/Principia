# Machines

The rule: the PC does only what the Mac can't.

## The Mac

The build machine. The orchestrator, the implementers and the reviewers run here; builds go through the job
queue (`orch/` on the X10 drive, outside the repo) with sccache.

## The PC: two roles, kept apart

The PC (RTX 2080 Super, Windows, WSL2, the E: drive) is both a CI runner and a second compute box. The two
roles use different accounts and different folders, so the runner's security never depends on what the
compute box runs.

**1. CI runner: locked down, unchanged.**
- Triggers: push to main, schedule and workflow_dispatch only; the repo and actor guard on every job;
  `contents: read`; no secrets; SHA-pinned actions; no Docker; never a required check.
- Runs as its own service account, in its own folders under `E:\actions-runner` and `E:\ci-cache`. Nothing
  else writes there.
- Jobs: `win-gpu.yml` (Vulkan suites on the real NVIDIA card, nightly and on request).
- Containment proof, on request only: `gh workflow run win-gpu.yml --ref main -f only=win-contain`.

**2. Compute box: an ordinary development machine.**
- Your own account, its own clone at `D:\src\Principia`, never the runner's folders.
- It takes only work the Mac can't do:
  - anything needing the NVIDIA GPU or Windows: the C4 GPU tasks, DX12, Vulkan on real hardware;
  - compute too long or heavy for the Mac: big renders, long sweeps, the replication runs;
  - mutation testing: the weekly sweep of the kernel (TASK-M0-73). Its surviving mutants come to the Mac as a
    `to:mac` issue, and each real one is a missing test, added in the next PR that touches that file.
- The Mac hands it work through a `to:pc` issue in `Ma1achy/principia-ops`, one request per issue. Tasks it
  owns are marked `machine: pc` in `plan/tasks.yaml`, so the Mac's orchestrator doesn't dispatch them. Its
  branches and PRs are reviewed like any other, on the Mac.
- GitHub access is your own `gh auth login` on that account. The runner's account has none.

**Sharing the machine**
- The runner's nightly is at 05:17 UTC. Long compute jobs should avoid it, or accept that both run slower.
- The GPU: the top ~10% of VRAM is faulty and capped at 7296 MiB. Every GPU job, including your other
  projects', stays under it. Run the 20-second memtest before and after long GPU runs.
- Disk: E: must keep at least 30 GB free for the runner (its jobs fail clearly below that).

**Later, if useful:** the Mac's job queue sends heavy jobs (big renders, sweeps) to the PC over SSH, so the
orchestrator can use the PC without a second session. Not needed before the GPU checkpoint.
