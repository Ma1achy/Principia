# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-201: TASK-M0-19's gaps: where a hosted gate run gets each requirement's result, a discrete adapter's VRAM, and the bench's own values *(REQ-SYS-067, REQ-TOOL-001, REQ-TOOL-121)*

- **File, section:**
  - `plan/tasks/M0/TASK-M0-19.md` § Deliverables: "`cargo xtask gate-report --milestone <Mn>`, which lists every
    requirement in that milestone's and every earlier milestone's gate block (`plan/MILESTONES.md`) with the result of
    its acceptance run"; "`gate-report --bench-results <dir>` reads their `prin profile` files, and marks each such
    requirement \"awaiting the human's run\" until they are supplied (R-186)".
  - `decisions.md` § "R-177 — Cadence *(closes G4, C6)*": "`gate.yml` (`workflow_dispatch`, input: milestone) runs
    every suite and writes the gate report."
  - `docs/design/principia_dd_telemetry_and_tiers.md` § 5, "The session header": "memory: {\"unified\": {bytes}} or
    {\"discrete\": {vram_bytes, ram_bytes}}"; § 2: "VRAM or unified memory size".
  - `plan/tasks/M0/TASK-M0-19.md` § Goal: "`cargo xtask bench <bench>` runs a fixed benchmark [...] A result is compared
    with its baseline through `prin profile diff`."
- **What:**
  1. **A hosted gate run's per-requirement result has no defined source.** The suites `gate.yml` runs report pass or
     fail per suite, not per requirement, and nothing maps a requirement to the suite that verifies it. 57 of M0's 120
     gate requirements are `verify.method: review checklist`, whose result no hosted suite produces.
  2. **A supplied benchmark file has no defined verdict.** The corpus says how a bench requirement waits, not how its
     supplied `prin profile` file passes or fails, nor how the file is matched to its requirement.
  3. **A discrete adapter's VRAM has no source.** The header's `discrete` variant requires `vram_bytes`, and wgpu, the
     harness's GPU API, reports no memory size on any backend.
  4. **The bench's own values are not given:** `trivial-kernel`'s frame count, and the threshold `cargo xtask bench`
     passes to `prin profile diff`, which requires one (REQ-TOOL-119). No requirement gates a bench before M3's
     REQ-PERF-004.
- **Applied per R-204 in PR (TASK-M0-19), each open for a veto:**
  1. `gate-report --results <file>` reads a JSON object, requirement id to `"pass"` or `"fail"`. `gate.yml` writes it
     from the suites by verification method: unit test, property test and numerical gate from the CPU jobs (`cpu`,
     `xtask-ci`); golden image from both GPU jobs; GUI screenshot from the screenshot job. A requirement with no result
     is "missing" and fails the report, so review-checklist requirements fail a hosted gate report until this is ruled.
  2. A benchmark requirement's file is `<dir>/<REQ-id>.jsonl`; it is "supplied" when its first line is a profiler
     schema v1 header line. "Awaiting" and "supplied" are listed and do not fail the report; the verdict on a supplied
     file is left to the task that closes the first benchmark requirement (M3, REQ-PERF-004).
  3. The harness refuses a discrete or virtual adapter ("wgpu reports no VRAM size"), so the bench fails on one rather
     than write a size it does not know. Integrated GPUs, CPU rasterisers (lavapipe) and Apple silicon record unified
     memory, the machine's RAM. telemetry §2 says so.
  4. `trivial-kernel` times 1000 frames, each one dispatch and readback of the kernel over the harness's 2^16-word
     fixture, compiled once outside the timing. `cargo xtask bench` runs the diff at `--threshold 0%`, so every rise in
     a scope's p95 is listed, and reports a rise without failing.
- **Options seen:**
  - Item 1: (a) **as applied, plus review-checklist results from the merged PRs (recommended):** a review-checklist
    requirement passes when the task that closes it (`plan/tasks.yaml`) has merged with its reviewers' approvals, which
    `gate-report` reads through `gh`; (b) as applied, review-checklist requirements listed "reviewed at merge" and not
    failed; (c) per-requirement results from each task's acceptance commands, run one by one by `gate-report`.
  - Item 2: (a) **as applied (recommended)**; (b) a supplied file passes when `prin profile diff` against the bench's
    baseline shows no regression at the requirement's own threshold, defined from M3.
  - Item 3: (a) **as applied (recommended)** until a discrete machine runs a bench; (b) read VRAM through wgpu-hal from
    each backend (Vulkan's device-local heaps, DXGI's dedicated video memory), an unsafe, per-backend addition;
    (c) write `memory: null` for a discrete adapter, a second meaning for R-308's null.
  - Item 4: (a) **as applied (recommended)**; (b) a frame count and a gating threshold as calibration requirements
    (R-71), proposed with evidence from the human's Mac.
- **Needed:** accept or choose per item. TASK-M0-19's PR builds the applied forms; item 1 decides whether a hosted M0
  gate report can pass.

---
