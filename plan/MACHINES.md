# Machines

## The Mac

The build machine. The orchestrator and the implementers run here; builds go through the job queue (`orch/`
on the X10 drive, outside the repo) with sccache.

## Reviews

Reviewers don't build: they read the diff and CI's results. So they cost little and can run on either
machine. Put them on the PC when the Mac is busy: a Claude Code session there runs the `reviewer` and
`physics-reviewer` agents on PRs the Mac's orchestrator lists as ready, and posts its reviews with `gh`.

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
- Your own account, a separate clone (e.g. `D:\src\Principia`, or `~/principia` in WSL), never the runner's
  folders.
- A Claude Code session there takes tasks that need the GPU or long compute, from the same `plan/PLAN.md`,
  on its own branches and PRs, reviewed like any other. Mark those tasks `machine: pc` in `plan/tasks.yaml`
  so the Mac's orchestrator doesn't dispatch them too.
- Good work for it:
  - the C4 GPU tasks, and the GPU-kernel suites on real hardware while developing them;
  - large renders (README images, galleries, the golden frames' references);
  - long measurements: convergence ladders, horizon sweeps, cross-checks against prin-rs;
  - the C8 replication runs;
  - mutation testing, on request.
- GitHub access is your own `gh auth login` on that account. The runner's account has none.

**Sharing the machine**
- The runner's nightly is at 05:17 UTC. Long compute jobs should avoid it, or accept that both run slower.
- The GPU: the top ~10% of VRAM is faulty and capped at 7296 MiB. Every GPU job, including your other
  projects', stays under it. Run the 20-second memtest before and after long GPU runs.
- Disk: E: must keep at least 30 GB free for the runner (its jobs fail clearly below that).

**Later, if useful:** the Mac's job queue sends heavy jobs (big renders, sweeps) to the PC over SSH, so the
orchestrator can use the PC without a second session. Not needed before the GPU checkpoint.
