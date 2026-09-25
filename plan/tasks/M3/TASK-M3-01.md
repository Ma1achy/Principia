# TASK-M3-01 — Kernel core: the generic Real, the force and the shared pair distances

- **Milestone:** M3
- **Closes:** REQ-INT-004, REQ-INT-016, REQ-INT-036, REQ-PERF-001, REQ-PERF-002
- **Depends on:** TASK-M2-15
- **Needs (earlier milestones):** REQ-PAY-017, REQ-SYS-004, REQ-SYS-015, REQ-DEC-005, REQ-INT-002
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-9
- **Size:** ~400 lines

## Goal
The physics layer of the shared kernel exists in `crates/kernel`: one source generic over its `Real` type, instantiated at f32 (the GPU build), f64 (the CPU default) and double-double (the same source, CPU), with the three-body force `aᵢ = Σ_{j≠i} mⱼ(rⱼ − rᵢ)/‖rⱼ − rᵢ‖³` (G = 1) computed from one set of pairwise distances that the energy evaluation and `r_min` reuse. The physics layer is a transcription with no allocation; the driver layer that follows (TASK-M3-04) is written fresh to integrator contract Part 2c's five rules.

## References
- `docs/contracts/principia_canonical_spec.md` § "1. The substrate (the defining decision)"
- `docs/design/principia_core_design.md` § "Principle: divergence is the observable"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/contracts/principia_integrator_contract.md` § "The split"
- `docs/design/principia_dd_integrator.md` § "3.1 Force (G = 1)"
- `docs/contracts/principia_integrator_contract.md` § "COM projection and invariant monitoring"
- `docs/contracts/principia_integrator_contract.md` § "Rules the new kernel must hold by construction"

## Deliverables
- `crates/kernel/src/real.rs` — the `Real` trait (the arithmetic, `fma`, comparisons the kernel may use) with impls for `f32`, `f64` and a double-double type `Dd`; no precision-dependent constant is reachable from a branch (Part 2c rule 1).
- `crates/kernel/src/physics/pairs.rs` — `PairGeom<R>`: the three separation vectors, `d²`, `‖·‖` and `‖·‖³` per pair, computed once per force evaluation and shared by the force, the potential term of E and `r_min`.
- `crates/kernel/src/physics/force.rs` — the force of dd_integrator §3.1 over the three pairs, reading `PairGeom`.
- `crates/kernel/src/physics/energy.rs` — `E = Σ‖pᵢ‖²/2mᵢ − Σ mᵢmⱼ/rᵢⱼ` and `L_z`, the potential reading `PairGeom` (no second distance computation).
- `#![no_std]` physics module (no `Vec`/`Box`/`dyn`/`std`), and a CI lint in `xtask` that greps `crates/kernel/src/{physics,driver}` for `Vec`, `Box`, `dyn`, `std::`, `alloc::` and fails on any hit.
- Golden-IC test fixtures: the z = 0 golden IC's decoded `(m, r, p)` (from the M2 decoder) run through the force at each instantiation.

## Acceptance tests
- `cargo test -p kernel real_instantiations` — the kernel instantiates at f32, f64 and double-double and the same golden-IC force/energy test runs on each (REQ-INT-004).
- `cargo test -p kernel force_known_state` — the force on a hand-computed three-body state matches to f64 rounding, and Σ mᵢaᵢ = 0 (REQ-INT-036).
- `cargo test -p kernel pair_geometry_shared` — instrumented build: one pairwise-distance computation per force evaluation; the energy's potential term and `r_min` read the same `PairGeom` (REQ-PERF-001; review checklist: `‖·‖³` shared with `r_min`).
- `cargo xtask lint no-alloc` — zero `Vec`/`Box`/`dyn`/`std`/`alloc` hits in `physics/` and the driver; a planted `Vec` in a scratch branch makes it fail (REQ-PERF-002).
- Review (code, physics): the physics layer (step/deriv/Hamiltonians) is a transcription with 0 Vec/Box/dyn/std, and the driver layer carries no prin-rs lineage that violates Part 2c rules 1–5 (REQ-INT-016).

## Notes
- Part 2c rule 4 (no `T::infinity()` seeds in folds; reseed min-folds from the first element) applies to every fold introduced here and in the driver — the §9 pitfall family.
- Gap: the regularised occupants' physics (Heggie, logH, the Mikkola–Tanikawa leapfrog) is to be transcribed from prin-rs (REQ-INT-016), which is not in this repository; see TASK-M3-07/08.
