# TASK-M4-01 — The shared kernel compiled to f32 SPIR-V and WGSL, under the compile-discipline lint

- **Milestone:** M4
- **Closes:** REQ-SYS-020, REQ-SYS-023, REQ-SYS-025, REQ-INT-058, REQ-INT-060, REQ-INT-062, REQ-INT-063, REQ-INT-067
- **Depends on:** TASK-M3-07
- **Needs (earlier milestones):** REQ-INT-004, REQ-INT-008, REQ-INT-016, REQ-SYS-015, REQ-SYS-004, REQ-GEN-015, REQ-INT-028, REQ-INT-038, REQ-TOOL-039
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~450 lines

## Goal
The whole M3 physics kernel — decode, canonicalise, the wrapper with its detectors and every occupant — is built by rust-gpu to f32 SPIR-V and translated by naga to WGSL from `crates/kernel`, the same crate that builds the CPU-f64 reference. Nothing of the physics exists a second time. A CI lint holds the kernel source to the GPU compile-discipline checklist and the integrator contract's "rules the new kernel must hold by construction" (comparison-only branches, no precision-dependent constant in a branch, bounded loops with a `done` flag and zero `break`, no non-finite literals, folds seeded from the first element, clamp before every float→int cast), so every later M4 task compiles against a kernel that already obeys them.

## References
- `docs/contracts/principia_canonical_spec.md` § "1. The substrate (the defining decision)"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_systems_architecture.md` § "6. Cross-cutting invariants (the load-bearing walls)"
- `docs/design/principia_core_design.md` § "1. Physics defined once, compiled twice — now *structurally*, not by discipline"
- `docs/contracts/principia_lowering_contract.md` § "Part 2 — Two assembly mechanisms (the substrate split; the old "one mechanism" claim retires)"
- `docs/contracts/principia_lowering_contract.md` § "Part 4 — The precompile rule"
- `docs/design/principia_dd_integrator.md` § "1. What it is"
- `docs/design/principia_dd_integrator.md` § "2. Consolidated contract"
- `docs/contracts/principia_parity_contract.md` § "Tier L — exact, no tolerance (the bulk of the suite)"
- `docs/contracts/principia_parity_contract.md` § "1. The principle"
- `docs/contracts/principia_canonical_spec.md` § "2. The determinism law (the other defining decision)"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `docs/contracts/principia_integrator_contract.md` § "Rules the new kernel must hold by construction"
- `docs/notes/principia_gpu_determinism_note.md` § "The discipline (each rule = one measured failure)"
- `docs/contracts/principia_integrator_contract.md` § "Part 4 — Determinism, and the substep as the subtle seam"
- `docs/design/principia_dd_integrator.md` § "3.3 Substep law — and the determinism pin, made concrete"
- `docs/notes/principia_gpu_determinism_note.md` § "The one-line law"
- `docs/contracts/principia_integrator_contract.md` § "Part 1 — The shape: a swappable `step()` slot inside a fixed wrapper"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- `crates/kernel`: the GPU build — rust-gpu compile of the kernel crate to SPIR-V entry points (build script or `xtask` step), naga SPIR-V → WGSL translation and naga validation as a build check. No `.wgsl` compute source is written by hand or checked in.
- `crates/kernel/tests/discipline_lint.rs`: a source lint over the kernel's physics and driver layers — no `break` in march loops, no bare `loop {`, no counted `for` with a mid-body exit (`for s in 0..N_sub`), no `f32::INFINITY` / `NAN` / `T::infinity()` literal, no runtime-indexed array in the kernel (constant indices or unrolled pairs), every float→int cast preceded by an f32 clamp, and no `Real`-dependent constant in an `if`/`while` condition. Each rule has a seeded-violation fixture that makes the lint fail.
- `crates/kernel/tests/naga_validate.rs`: naga validation of the translated module; the all-unusable fold test.
- `crates/kernel/tests/single_source.rs`: the GPU module and the CPU reference are built from `crates/kernel` only; a repo-wide grep finds no WGSL compute source and no second copy of step / wrapper / detector / decode logic outside `crates/kernel`.
- Any fold still seeded from a non-finite or zero value in the M3 kernel is reseeded from the first element (the `dt_max` fold is the recorded instance).

## Acceptance tests
- `cargo test -p kernel naga_validate` — the compiled kernel passes naga validation (no infinite float literal) and an all-unusable input folds to a defined value, not 0 or inf (REQ-INT-063).
- `cargo test -p kernel discipline_lint` — the lint finds no `break` in march loops, no `for s in 0..N_sub`, no bare `loop {` (every loop has a count bound usable by the watchdog and dispatch chunking), no `f32::INFINITY`/NaN literal, no dynamic index and no unclamped float→int cast; naga validation passes (REQ-INT-058, REQ-INT-062, REQ-INT-067). Each rule's seeded violation turns the lint red.
- `cargo test -p kernel precision_constants` plus the physics reviewer's audit of every branch condition — no `Real`-dependent threshold (the prin-rs `SYNC_EPS` / `LAND_EPS_REL` pattern) in any branch; a tolerance that scales with precision appears only where the value is used (REQ-INT-060).
- `cargo test -p kernel single_source` plus the code reviewer's inspection of the source tree — both the GPU shader module and the CPU reference build from the kernel crate; no hand-written WGSL compute source; exactly one implementation of step / wrapper / detectors, instantiated at f32 and f64 (parity Tier L part 1: verified by inspection, never by agreement of results) (REQ-SYS-020, REQ-SYS-023, REQ-SYS-025).

## Notes
- REQ-SYS-025's check is by inspection, never by agreement of results (parity contract Tier L, part 1); the reviewer records the inspection in the PR.
- The lint is the installed check for the review-checklist requirements it covers; it stays in CI for every later task.
- RQ-97 ruled: R-113 — the GPU legs dropped from the M3 verifies (REQ-INT-007, 028, 029, 030, 031) are covered by M4's REQ-VAL-059, REQ-VAL-061 and REQ-VAL-072 on this build; the WGSL → browser-compiler leg is REQ-VAL-144 (M8).
