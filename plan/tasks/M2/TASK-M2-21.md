# TASK-M2-21 — Link provenance: the ViewState record, link swap and the link-variation diagnostic

- **Milestone:** M2
- **Closes:** REQ-SYS-011, REQ-CHART-033, REQ-VAL-024
- **Depends on:** TASK-M2-05, TASK-M2-15, TASK-M2-20
- **Needs (earlier milestones):** REQ-GEN-008
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~380 lines

## Goal
The exported ViewState records (z₀, q₁, q₂), the chart id and params, the axis warps and the link id per block, so any chart — tilted or not — saves and restores exactly and a figure's provenance pins its links (R-106: the link ids are the chart's per-block registry entries). Decode and encode consume only registry links, a link swap swaps both directions and changes provenance, and the link-variation diagnostic inverts a canonicalised IC under each registry link and reports residual_L and κ_L without ever changing the default link.

## References
- `docs/contracts/principia_chart_decoder_contract.md` § "Integrity: the link is part of the experiment"
- `docs/contracts/principia_chart_decoder_contract.md` § "The operation table"
- `docs/design/principia_dd_generation_root.md` § "4. Seams (obligations → integration tests)"
- `docs/design/principia_dd_generation_root.md` § "2. Consolidated contract"
- `docs/design/principia_dd_decoder.md` § "4. Seams (its side of each — the integration-test list)"
- `docs/design/principia_dd_encode.md` § "2. Consolidated contract"
- `docs/design/principia_dd_encode.md` § "4. Seams (obligations → integration tests)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Link variation as a conditioning diagnostic (not a default change)"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 7 — Ground-truth ingestion (the validation programme's demand)"
- `decisions.md` § "R-106 — The link ids are the chart's link functions *(closes RQ-66)*"

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
## Deliverables
- The ViewState provenance record in `crates/engine/src/contract/view_state.rs` (serialise / restore).
- `crates/validation/src/link_variation.rs` (the diagnostic; per-IC link choice recorded in provenance).
- Tests `crates/engine/src/contract/tests/view_state.rs`, `crates/kernel/tests/link_swap.rs`.

## Acceptance tests
- `cargo test -p engine viewstate_roundtrip` — export a tilted view with a non-default link, reload, assert bitwise-identical z per pixel and identical link ids (REQ-SYS-011).
- `cargo test -p kernel link_swap` — property: T2 round-trips via the registry inverses; swapping a link changes the provenance and swaps the decode and encode directions together (REQ-CHART-033; the sim-key change and re-integration are REQ-CHART-052, TASK-M4-08).
- Review (physics): the diagnostic reports residual_L and κ_L per link; provenance records the link used; the default link is unchanged by validation code (REQ-VAL-024).

## Notes
- RQ-95 ruled: R-113 — REQ-CHART-033 is split: this task asserts registry-only links, provenance and the two-direction swap; "a link swap changes the sim key, recompiles and re-integrates from t = 0" is REQ-CHART-052 (TASK-M4-08). This settles Gap G23.
