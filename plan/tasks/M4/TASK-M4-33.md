# TASK-M4-33 — The primitive registry, the refusals, the labels and the bitwise digest

- **Milestone:** M4
- **Closes:** REQ-CHART-096, REQ-CHART-097, REQ-CHART-098, REQ-CHART-099, REQ-PERF-116
- **Depends on:** TASK-M4-37, TASK-M4-32, TASK-M2-14
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
The authoring layer gains the registry, every pair is validated against one refusal table, and every chart comes out of `resolve` with its lowering and its labels. The digest of today's charts is written first, on its own commit, so that every later task is held to it.

## References
- `docs/design/principia_chart_composition.md` § "3. The primitive registry"
- `docs/design/principia_chart_composition.md` § "6.1 Refusals: genuine contradictions only"
- `docs/design/principia_chart_composition.md` § "6.2 Labels: reduced capability, never a refusal"
- `docs/design/principia_chart_composition.md` § "7. Lowering: three ways a pair runs"
- `docs/design/principia_chart_composition.md` § "11. The axis cap"
- `docs/design/principia_chart_composition.md` § "12. Tests that can fail"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"

## Deliverables
- `fixtures/chart_composition/digest.json`, on the first commit, before any change.
- `crates/engine/src/chart_authoring/registry.rs`: the rows; `AxisSpec`, `ChartSpec`.
- `crates/engine/src/chart_authoring/validate.rs`: the new `Refusal` variants; the pair table.
- `resolve`'s lowering and labels; the ledger constant for the cap.

## Acceptance tests
- `cargo test -p engine size_only_refused` and `cargo test -p engine excluded_primitives` — I, R and ℓ refused by name; each row sampled on decodes; controls: a constant row declared free fails, a row naming a binary phase fails (REQ-CHART-096).
- `cargo test -p engine pair_table_complete` — the rank test agrees with the table both ways; controls: a row removed, a rank-2 pair added (REQ-CHART-097).
- `cargo test -p engine lowering_labels` — each of the three lowerings and its labels; control: an unbaked pair refused fails (REQ-CHART-098).
- `cargo test -p engine presets_bitwise` — every chart, preset and saved chart against the digest; controls: a mantissa bit flipped, a default field serialised (REQ-CHART-099).
- `cargo test -p engine baked_cap` — at or under the ledger's cap; control: one over (REQ-PERF-116).

## Notes
- The cap's value is RQ-716's ruling. Nothing here changes a kernel bit.
- Waits on its rulings item: RQ-716, RQ-722. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
