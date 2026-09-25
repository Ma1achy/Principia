# TASK-M4-19 — First kernel benchmarks: the march's bottleneck, N = 8 vs 16, and one quad thread-count constant

- **Milestone:** M4
- **Closes:** REQ-PERF-009, REQ-PERF-011, REQ-SYS-029, REQ-PERF-086
- **Depends on:** TASK-M4-06, TASK-M4-09
- **Needs (earlier milestones):** REQ-TOOL-006, REQ-TOOL-001, REQ-TOOL-002, REQ-SYS-003
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
The first measurements of the kernel the corpus leaves to the build. The march kernel is profiled at production dispatch granularity to confirm whether it is bandwidth- or arithmetic/occupancy/register-bound and whether it spills. N = 8 vs N = 16 is measured on a WebGPU target and recorded, with workgroup limits measured from the adapter rather than quoted; a validation check asserts N² ≤ `maxComputeInvocationsPerWorkgroup` for every N the tiers can select. One constant for the quad thread count lives in the shared source, and the docs that state it agree (R-43).

## References
- `docs/design/principia_dd_simstate_payload.md` § "3. The word buffer — `free_group_word`"
- `docs/design/principia_dd_simstate_payload.md` § "8. Build-time settles (measure / specify once running)"
- `docs/design/principia_dd_generation_root.md` § "3.3 `free_group_word` (uint4) — mixed-radix packing, **in a separate buffer**"
- `docs/design/principia_memory_tiers.md` § "4. The six quality tiers"
- `docs/contracts/principia_scheduler_contract.md` § "Part 6 — The settled policy"
- `decisions.md` § "R-43 — `N = 16` vs 8 is measured; the thread-count inconsistencies are fixed now *(RS-3)*"
- `docs/design/principia_systems_architecture.md` § "Still to check"
- `docs/read_first/principia_INDEX.md` § "Known open items"
- `docs/contracts/principia_lowering_contract.md` § "Part 5 — The resolution function (the "switch", concretely)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `xtask` benches `march-bottleneck` and `quad-n` (writing profiler schema v1 records).
- `crates/engine`: adapter-limit query and the N² check applied to every tier and Custom value.
- `crates/kernel`: the single quad-thread-count constant; doc edits conforming systems_architecture §5.5, memory_tiers §4, the scheduler and lowering contracts and the quality device note, with the "Removed lines" note.

## Acceptance tests
- `cargo xtask bench march-bottleneck` — the march kernel profiled at production dispatch granularity; bound type and spill counts recorded (REQ-PERF-009).
- `cargo xtask bench quad-n` — N = 8 vs N = 16 profiled on a WebGPU target and the result recorded; `cargo test -p engine workgroup_limits` asserts N² ≤ `maxComputeInvocationsPerWorkgroup` (measured) for every tier and Custom value (REQ-PERF-011).
- `cargo test -p kernel quad_thread_constant` plus perf review — one constant for the quad thread count in the shared source; all docs/config reference it (REQ-SYS-029).
- Proposal: N for Ultra and Extreme within N² ≤ 256, from the N = 8 vs 16 benchmark; the human confirms it at the M4 gate (REQ-PERF-086).

## Notes
- Gap: R-43 says to fix the tier table's N = 24 / 32 entries now but gives no replacement values; the N² check will fail on them until ruled.
- Closes, for gaps the corpus leaves open: REQ-PERF-086 (R-71 calibration) (REVIEW_QUEUE RQ-110 lists them for the human).
