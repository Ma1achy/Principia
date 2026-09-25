# TASK-M5-11 — Memory budgets, the hard cap and continuous pressure

- **Milestone:** M5
- **Closes:** REQ-PERF-019, REQ-PERF-021, REQ-PERF-023, REQ-PERF-029, REQ-PERF-030, REQ-SCHED-050, REQ-VAL-082, REQ-PERF-087
- **Depends on:** TASK-M5-09, TASK-M5-10
- **Needs (earlier milestones):** REQ-TOOL-001
- **Reviewers:** code, qa, perf
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~450 lines

## Goal
Memory is budgeted before it is allocated and watched continuously. The cache's hard cap is a fraction of the
budget device characterisation detects (never a constant); the requirement `quads × N² × (E+1) × sizeof(SimState)` is
checked against adapter limits and a misfit reduces the tier and reports it; a forced setting that fails to allocate
drops to a working tier with a message. Pressure is three states re-evaluated through the session — comfortable,
pressured (stop growing the tree, keep serving, report cap-bound), reclaiming (evict). The low-cap path is exercised
deliberately. The full process footprint is measured, not inferred.

## References
- `docs/design/principia_dd_simstate_payload.md` § "7. Memory"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "6.2 Memory pressure — CONTINUOUS, not an error condition"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "Three states, not two"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "6.2a Budgeting before allocating"
- `docs/design/principia_memory_tiers.md` § "4.1 The same six tiers on the three axes"
- `docs/design/principia_memory_tiers.md` § "7. The "are you sure?" safety system (three severities, none blocking)"
- `docs/design/principia_temporal_architecture_note.md` § "The one accepted cost (stated plainly)"
- `docs/design/principia_temporal_architecture_note.md` § "Decisions — DECIDED (ratified in conversation; recorded here so they don't evaporate)"
- `docs/design/principia_temporal_architecture_note.md` § "Footguns (all re-applications of disciplines already established)"
- `docs/contracts/principia_caching_contract.md` § "Part 7 — The current-state cache (resume points, hard-capped)"
- `docs/design/principia_quality_device_note.md` § "What this subsystem resolves (two previously-open questions)"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "And test it deliberately"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/engine/src/memory/{budget.rs, pressure.rs}`: cap from the detected budget, pre-allocation fit, tier
  fallback with its message, the three-state machine.
- `crates/engine/tests/memory_pressure.rs`: scripted budget drop; injected allocation failure at Extreme; low-cap run.
- `cargo xtask bench process-memory`: peak process memory per tier config.

## Acceptance tests
- `cargo xtask bench process-memory` — record peak process memory for the tier configs on a 16 GB unified-memory machine (REQ-PERF-019).
- `cargo test -p engine pressure_states` — a scripted budget drop mid-session moves the state through pressured to reclaiming (REQ-PERF-021).
- `cargo test -p engine budget_before_allocate` — an adapter with small limits causes a reported tier reduction, never a failed allocation (REQ-PERF-023).
- `cargo test -p engine hard_cap_fraction` — hard cap scales with the reported available memory for each tier (REQ-PERF-029).
- `cargo test -p engine forced_tier_fallback` — inject allocation failure at Extreme; app falls back to a lower tier and shows the fallback message (REQ-PERF-030).
- `cargo test -p engine cache_cap_bytes` — cache bytes never exceed the cap; entries hold no past-t state; cap scales with the detected budget (REQ-SCHED-050).
- `cargo test -p engine low_cap_never_blanks` — low-cap run: no blank frame, frame time within budget, cap-bound reported (REQ-VAL-082).
- Proposal: the pressured-onset threshold (fraction of the hard cap) with evidence from the scripted budget-drop run; the human confirms it at the M5 gate (REQ-PERF-087).

## Notes
- "An OOM path that has never executed is an OOM path that does not work": the low-cap test must drive the system
  through pressured and reclaiming, and the blank-frame assertion must be able to fire (pitfalls §3, §9).
- The cap fraction and the pressured/reclaiming thresholds are not given by the corpus (see Gaps).
- Waits on RQ-88 (`REVIEW_QUEUE.md`): Eviction order: deepest first, or cost-weighted resistance?.
- Waits on RQ-100 (`REVIEW_QUEUE.md`): Existing requirements closed after the task that needs them.
- Closes, for gaps the corpus leaves open: REQ-PERF-087 (R-71 calibration) (REVIEW_QUEUE RQ-110 lists them for the human).
