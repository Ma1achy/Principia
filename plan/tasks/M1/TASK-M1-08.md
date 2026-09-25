# TASK-M1-08 — The generated field-view catalogue and the scanned debug registry

- **Milestone:** M1
- **Closes:** REQ-GEN-009, REQ-GEN-010, REQ-GEN-011, REQ-TOOL-017, REQ-TOOL-020
- **Depends on:** TASK-M1-03, TASK-M1-05, TASK-M1-06
- **Needs (earlier milestones):** REQ-GEN-001, REQ-GEN-002, REQ-GEN-008, REQ-TOOL-004
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~450 lines

## Goal
The ledger generates the fourth artefact beside Rust pack/unpack, the host export decoder and the WGSL unpack: the WGSL debug catalogue — exactly one field view and one test per ledger field, emitted into `frag/debug/generated/` and surfaced by the same filesystem scan as hand-written debug occupants (registry entries `{id, slot, source, category, uniformSchema, inputDomains}`, tagged `category: debug`). The catalogue is exhaustive by construction: a new ledger field appears in the picker or generation fails. The shader and its test call the same generated accessor symbols.

## References
- `docs/contracts/principia_gui_state_contract.md` § "3. The registry is the scanned filesystem — for the *fragment* side; compute occupants are Rust build variants"
- `docs/design/principia_dd_generation_root.md` § "1. What it is"
- `docs/design/principia_dd_generation_root.md` § "Drill-down — the Generation Root (layout table + link registry)"
- `docs/design/principia_dd_simstate_payload.md` § "Drill-down — the SimState payload (consolidated authoritative spec)"
- `docs/design/principia_dd_generation_root.md` § "4. Seams (obligations → integration tests)"
- `docs/contracts/principia_render_contract.md` § "Part 5 — Struct inspection SDK"
- `docs/contracts/principia_lowering_contract.md` § "Part 2 — Two assembly mechanisms (the substrate split; the old "one mechanism" claim retires)"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_systems_architecture.md` § "6. Cross-cutting invariants (the load-bearing walls)"
- `docs/design/principia_systems_architecture.md` § "7. Hierarchy and dependency (the build DAG, abstract)"
- `docs/design/principia_debug_tooling_plan.md` § "Principle"
- `docs/design/principia_dd_generation_root.md` § "2. Consolidated contract"
- `docs/contracts/principia_render_contract.md` § "Part 6 — The debug catalogue (first build target)"
- `docs/design/principia_dd_colouring.md` § "4. Seams (obligations → integration tests)"
- `docs/design/principia_debug_tooling_plan.md` § "B. Payload field views — `sample_descriptor` (bit-packed u32)"
- `docs/design/principia_debug_tooling_plan.md` § "C. Payload field views — `times` (u32), f16-packed scalars & `free_group_word` (separate buffer)"
- `docs/design/principia_debug_tooling_plan.md` § "D. Payload field views — `SimState` scalars (ledger §3.4)"
- `docs/design/principia_debug_tooling_plan.md` § "E. Payload field views — `ICDescriptor` (64 B) & the live-state block"
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"

## Deliverables
- `crates/ledger`: catalogue emission — per-field WGSL view sources (baked one per field via TASK-M1-05's entry point) and the paired Rust test stubs that call the Rust twin of the same accessor.
- `crates/render/src/registry.rs`: the directory scan of `shaders/wgsl/frag/**` (and `frag/debug/generated/`) producing registry entries; `category: debug` tagging.
- Generated-file guard extended to the catalogue output.
- Tests: `crates/ledger/tests/catalogue.rs`, `crates/render/tests/registry.rs`.

## Acceptance tests
- `cargo test -p render generated_views_in_registry` — after ledger codegen, each generated field view appears as a debug registry entry (REQ-GEN-009).
- `cargo test -p ledger one_source_four_artefacts` — mutating one ledger entry changes all four artefacts together; editing a generated file makes the generated-file guard fail (REQ-GEN-010).
- `cargo test -p ledger new_field_in_catalogue` — adding a field with metadata makes it appear in the generated catalogue; without metadata generation fails naming the field (REQ-GEN-011).
- `cargo test -p ledger shader_and_test_share_accessor` — each view's WGSL and its test reference the same generated accessor symbols (REQ-TOOL-017).
- `cargo test -p ledger catalogue_equals_ledger` — the catalogue's field list equals the ledger's field list (REQ-TOOL-020).

## Notes
- The per-view colouring (numeric template, sentinel styling, categorical palettes) is TASK-M1-09 and TASK-M1-10; this task emits the views with the ledger's `scale` and a placeholder ramp call and proves exhaustiveness.
- Exhaustiveness tests are shown able to fire: a deliberately unregistered field must fail `catalogue_equals_ledger` (PIT-3).
