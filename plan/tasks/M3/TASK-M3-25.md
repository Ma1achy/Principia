# TASK-M3-25 — The independent convergence reference (Brutus-style)

- **Milestone:** M3
- **Closes:** REQ-VAL-025, REQ-VAL-131
- **Depends on:** TASK-M3-24
- **Needs (earlier milestones):** REQ-DEC-005
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-10
- **Size:** ~450 lines

## Goal
A separate CPU arbitrary-precision integrator with convergence gating — raise the precision and tighten the tolerance until the result stops changing — exists in `crates/validation`, sharing no source with the kernel. It is the reference for the integration-floor probe and Burrau ground truth; double-double is a fast screen only and RK45 stays the inspector's reference (R-33). Its protocol (the precision/tolerance ladder, the stop rule, when it runs, its harness interface) is defined in the validation note (R-72).

## References
- `docs/contracts/principia_canonical_spec.md` § "7. Precision & validation model *(authoritative: `parity_contract`, `validation_ground_truth_note`, `core_design`)*"
- `docs/contracts/principia_canonical_spec.md` § "11. Still open / downstream (not yet fully in the corpus)"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `decisions.md` § "R-33 — The independent convergence reference is Brutus-style *(IE-5, amended)*"
- `docs/contracts/principia_parity_contract.md` § "1. The principle"
- `docs/notes/principia_validation_ground_truth_note.md` § "The correctness factoring: validate on CPU, inherit on GPU"
- `docs/read_first/principia_00_philosophy.md` § "Why using the same kernel is load-bearing"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `crates/validation/src/reference/brutus.rs` — arbitrary-precision (e.g. an MPFR/`rug`-class dependency, dev-only) Bulirsch–Stoer-style integrator with the convergence loop; no `crates/kernel` import.
- Doc change: `docs/notes/principia_validation_ground_truth_note.md` — the protocol (definition, REQ-VAL-131).
- `xtask gate reference-convergence`.

## Acceptance tests
- `cargo xtask gate reference-convergence` — the reference output is unchanged between successive precision/tolerance steps before it is accepted; a build importing `crates/kernel` fails the dependency check (REQ-VAL-025).
- The doc states the precision/tolerance ladder and the stop rule, when the reference runs, and its harness interface; physics reviewer approved (REQ-VAL-131).

## Notes
- The choice of arbitrary-precision library is a build dependency, not a corpus decision; it must not be shared with the kernel.
- Definitions written here (R-72; physics reviewer approves before merge): REQ-VAL-131.
