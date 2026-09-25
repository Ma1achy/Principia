# TASK-M4-16 — The pixel inspector's engine side: f64 trace, f32 GPU trace and the divergence overlay

- **Milestone:** M4
- **Closes:** REQ-TOOL-040, REQ-TOOL-041, REQ-TOOL-042, REQ-VAL-070, REQ-VAL-071, REQ-SYS-028
- **Depends on:** TASK-M4-04, TASK-M4-06, TASK-M3-27, TASK-M3-36
- **Needs (earlier milestones):** REQ-GUI-008, REQ-INT-055, REQ-SYS-017, REQ-VAL-036, REQ-VAL-035, REQ-INT-005, REQ-VAL-034
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-10
- **Size:** ~450 lines

## Goal
Divergence is the observable. Click a pixel → its trajectories in shape and real space: the shared kernel on the CPU at f64 or higher with adaptive stepping, the on-demand single-IC f32 GPU trace of the same kernel, and an overlay/diff exposing divergence time and separation growth without reconciling them. A match-integrator mode binds the active GPU occupant, coefficients and step size on the f64 build; adaptive RK45 is reserved for near-collision chasing. The CPU↔GPU continuous cross-check runs only for t < t_max(f32) and says "not applicable" beyond; t_max(f32) is measured against the GPU kernel directly. The atlas, the inspector and the prebake run one kernel and one chart set.

## References
- `docs/contracts/principia_canonical_spec.md` § "7. Precision & validation model *(authoritative: `parity_contract`, `validation_ground_truth_note`, `core_design`)*"
- `docs/design/principia_core_design.md` § "Principle: divergence is the observable"
- `docs/design/principia_core_design.md` § "4. Integrate and colour are separate passes — and now separate *mechanisms*"
- `docs/contracts/principia_integrator_contract.md` § "Part 6 — Three boundaries stated honestly (deferred work lands here)"
- `docs/design/principia_dd_integrator.md` § "2. Consolidated contract"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_dd_predictability_horizon.md` § "4.1 The two kernels have different horizons"
- `docs/design/principia_dd_generation_root.md` § "Refinement — the scaling exponent"
- `decisions.md` § "R-93 — The f32 predictability horizon gates the cross-check only *(closes RQ-44)*"
- `decisions.md` § "R-35 — The change-10 cross-checks are re-run and the NumPy reference patched *(IE-7)*"
- `decisions.md` § "R-105 — R-93's re-run is R-35's *(closes RQ-65)*"
- `docs/design/principia_dd_predictability_horizon.md` § "6. Open"
- `docs/design/principia_dd_predictability_horizon.md` § "7.2 `lambda` is 0.6–0.8, not 1 — and `t` is not e-foldings"
- `docs/read_first/principia_00_philosophy.md` § "Why using the same kernel is load-bearing"
- `docs/read_first/principia_00_philosophy.md` § "Three rungs, one kernel, and the standard of truth rises with the cost"
- `docs/read_first/principia_00_philosophy.md` § "1. The one-sentence version"
- `docs/design/principia_dd_predictability_horizon.md` § "4.3 It bounds what the renderer may claim"

## Deliverables
- `crates/engine/src/inspector.rs`: `inspect(chart, uv, sim_key, mode)` → f64 trace, f32 GPU trace, divergence time and separation growth; modes: match-integrator, adaptive RK45.
- `crates/validation`: the cross-check gate on `t < t_max(f32)`, reading the recorded value.
- `xtask` gate `f32-horizon`: GPU-kernel runs measuring the f32 predictability horizon; the value recorded in `fixtures/gates/`.

## Acceptance tests
- `cargo test -p engine inspector_traces` — on a Burrau IC the inspector returns both traces and a divergence time; no path nudges one trace toward the other (REQ-TOOL-040).
- `cargo test -p engine inspector_match_integrator` — match-integrator mode binds the active GPU occupant/params on the f64 CPU build; RK45 is not a wrapper occupant (REQ-TOOL-041).
- Code reviewer — debug views are fragment occupants; the diff calls the shared kernel twice, at two precisions (REQ-TOOL-042).
- `cargo test -p validation crosscheck_horizon_gate` — the gate reads the recorded re-run value of t_max(f32); the inspector/cross-check past it reports "not applicable" rather than a divergence (REQ-VAL-070).
- `cargo xtask gate f32-horizon` — the measured f32 horizon, recorded from GPU-kernel runs (REQ-VAL-071).
- `cargo test -p validation atlas_vs_inspector` plus code review — one shared kernel crate serves atlas, inspector and prebake; one IC through the atlas path (f32) and the inspector path (f64) agrees inside the f32 horizon, within the REQ-VAL-064 Tier N tolerances (REQ-SYS-028).

## Notes
- The GUI window hosting the inspector is M8; this task delivers the engine surface and its tests.
- Gaps (see report): how the on-demand f32 GPU trace is produced without a stored history, the definition of "divergence time", and which t_max(f32) value governs when the R-35 re-run and the GPU measurement differ.
