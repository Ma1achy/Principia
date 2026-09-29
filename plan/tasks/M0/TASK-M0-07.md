# TASK-M0-07 — The generation root: ledger schema, metadata gate and static layout checks

*Split by R-240: the static layout check (REQ-GEN-003) is TASK-M0-35.*

- **Milestone:** M0
- **Closes:** REQ-GEN-002, REQ-GEN-024
- **Depends on:** TASK-M0-01, TASK-M0-22
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~560 counted lines with the fixture ledger (R-240; the whole was ~450 before RQ-151 measured 1003)

## Goal
`crates/ledger` is the generation root (debug_tooling_plan step 0a), the root of the build DAG. It holds the Rust layout definition's types — a ledger entry carries generation-root §3.8's metadata: name; location as (word, offset, width) or a scalar index; type u-bits | f32 | f16-pair | fixed16; scale lin | log | cyclic | diverging | categorical(n) | flag; range; optional sentinel and tier gate; provenance kernel | decode | reduction | cpu; consumers — and the generator driver `cargo xtask codegen`, which refuses to generate when any entry is incomplete, naming the field, and first runs the static check over every packed word: no two fields overlap, every bit is covered or explicitly reserved, and every width fits its range.

## References
- `docs/design/principia_dd_generation_root.md` § "1. What it is"
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"
- `docs/design/principia_dd_generation_root.md` § "5. Tests (properties any generator must satisfy)"
- `docs/design/principia_debug_tooling_plan.md` § "H. Codegen self-test (the tooling that tests the tooling)"
- `docs/design/principia_debug_tooling_plan.md` § "Build order within Phase 0 (the only forced staggering)"
- `docs/contracts/principia_render_contract.md` § "Part 6 — The debug catalogue (first build target)"
- `docs/design/principia_dd_colouring.md` § "2. Consolidated contract"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_dd_generation_root.md` § "3.4 `SimState` scalars — with presentation metadata"
- `docs/contracts/principia_gui_state_contract.md` § "4. The occupant model — typed by signature, free inside"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-240 — TASK-M0-07 is split in two *(closes RQ-151)*"
- `decisions.md` § "R-241 — `xtask` may depend on `ledger`, for `cargo xtask codegen` *(amends systems_architecture §7.1)*"
- `decisions.md` § "R-242 — The layout check's range and width rules *(closes RQ-151's rules)*"
- `docs/design/principia_systems_architecture.md` § "7.1 Crate map"

## Deliverables
- `crates/ledger/src/schema.rs` — `Entry`, `Location`, `FieldType`, `Scale`, `Provenance`, `Consumer`, as §3.8; a builder returning `Err(IncompleteEntry { field, missing })` for an incomplete entry.
- `crates/ledger/src/check.rs` — the static check per packed word: overlap, coverage-or-reserved, width vs range.
- `crates/ledger/src/gen/mod.rs` — the generator driver (validate → static check → emit); the emitters land in TASK-M0-09, -10 and -13.
- `xtask/src/codegen.rs` — `cargo xtask codegen` (regenerates the checked-in generated files), depending on `ledger` directly; `cargo xtask deps`'s edge table gains `xtask` → `ledger` (R-241).
- The static check (`check.rs`, the `layout_static` tests) is TASK-M0-35's; the driver's static-check step lands there (R-240).
- A fixture ledger for the tests (a packed word with fields, reserved bits and ranges).

## Acceptance tests
- `cargo test -p ledger metadata_gate` — deleting one entry's `scale` makes generation fail with that field's name; the same for every other required §3.8 key (REQ-GEN-002).
- Moved to TASK-M0-35 (R-240): `cargo test -p ledger layout_static` — static disjointness/coverage check over the ledger: an overlapping pair, an undeclared uncovered bit and a width too narrow for its range each fail, naming the word and the bits; the fixture ledger passes (REQ-GEN-003).
- Definition: §3.8's derived-field location kind and vector type written into dd_generation_root §3.8 and approved by the physics reviewer (REQ-GEN-024).

## Notes
- Depends on TASK-M0-22 in place of TASK-M0-04 (R-198 split it): this task's tests register negative controls, which come before the tests that need them (R-176). TASK-M0-04 is still reached through TASK-M0-22.
- The generated-file guard (a hand edit to a generated file is detected) is REQ-GEN-010, M1; not built here.
- The link registry (generation-root §3.9) is the second root; its requirements are M2 (REQ-GEN-013 onward), so it is not built here.
- See Gaps: §3.8 has no location kind for a derived (not stored) catalogue field.
- Closes, for gaps the corpus leaves open: REQ-GEN-024 (R-72 definition) (classification accepted by R-132).
- R-240 (closes RQ-151): built at 1003 counted lines; split in two, the static check to TASK-M0-35. R-241: `xtask` depends on `ledger`. R-242: a `Range` end is closed, open or unbounded, and an unnamed entry is reported as "entry #i (unnamed)".
