# TASK-M4-33 — The baked entries on the GPU, the axis cap and V1's decode time (R-916)

- **Milestone:** M4
- **Closes:** REQ-PERF-116
- **Depends on:** TASK-M2-66, TASK-M2-70
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
The baked side of TASK-M2-66's registry on the GPU: the closed-form pairs baked beyond the gallery, at or under the ledger's cap and created lazily on Apply, the virial pair's baked entry (TASK-M2-70) among them, and V1's decode time measured (§ 13).

## References
- `docs/design/principia_chart_composition.md` § "3. The primitive registry"
- `docs/design/principia_chart_composition.md` § "3.3 Every primitive's class"
- `docs/design/principia_chart_composition.md` § "6.1 Refusals: genuine contradictions only"
- `docs/design/principia_chart_composition.md` § "6.2 Labels: reduced capability, never a refusal"
- `docs/design/principia_chart_composition.md` § "7. Lowering: three ways a pair runs"
- `docs/design/principia_chart_composition.md` § "11. The axis cap"
- `docs/design/principia_chart_composition.md` § "12. Tests that can fail"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"

## Deliverables
- The baked entries in `crates/engine/src/variants.rs`, the virial pair's included; the ledger constant for the cap.
- V1's decode time: a warped axis's decode against the linear arm on the GPU, recorded, past +2% the warp becomes baked (§ 13 V1).

## Acceptance tests
- `cargo test -p engine baked_cap` — the count of baked pairs in the variant table at or under the ledger constant; control: one entry over fails; a chart switch to a baked pair creates its pipeline once, inside the re-integration; and V1's decode time: a warped axis's decode against the linear arm on the GPU, recorded, past +2% the warp becomes baked (§ 13 V1) (REQ-PERF-116).

## Notes
- **Build order (R-863, as R-916 and R-917 amend it):** the GPU adapter onto stage 3's registry, which is TASK-M2-66's in M2 (R-916): the baked entries, the cap and V1's decode time. It depends on TASK-M2-66 and TASK-M2-70, and starts after M2's gate (R-917's hold). *Was:* "stage 3, the primitive registry and composed axes, each primitive with its round-trip test. Its edge to stage 2 (TASK-M8-117) isn't made: it is in M4 and TASK-M8-117 in M8, so the edge would depend forward across milestones, and TASK-M8-117 already waits on it through TASK-M8-106 and TASK-M8-102, so the edge would close a cycle. RQ-744 (A) holds the choice."
- The cap's value is RQ-716's ruling. Nothing here changes a kernel bit.
- Waits on its rulings item: RQ-716, RQ-722. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); R-917's hold holds it, no longer an edge: R-916 took TASK-M2-58 (was TASK-M4-37), the rulings applied, out of Depends on, and no M3 to M9 task starts until M2's gate passes, which waits on TASK-M2-58 as an M2 task. *Was:* "TASK-M4-37 in Depends on holds it."
- *8 Nov 2026 (R-873):* **Build order:** stage 3, released by dispatch after stage 2 (TASK-M8-117), the milestone kept (RQ-744 (A) (b)); still no edge to the M8 task.
- *9 Nov 2026 (R-916, R-917):* the line above is a record: R-916 supersedes R-873 (A) (b), stage 3's release by dispatch with the milestone kept. Stage 3's maths is M2's, and this task, the registry's GPU adapter, keeps M4 and waits for M2's gate by R-917's hold, not for stage 2 (TASK-M8-117 then, TASK-M2-65 now, in M2).
- *8 Nov 2026 (R-872):* RQ-716 is closed: the cap criterion's 2 s start-up limit is the Mac's; issue #427 sets the Vulkan and Windows budget from its own measurements.
- *9 Nov 2026 (R-916):* the chart core is M2's (R-916, R-917, R-919), so this header follows plan/tasks.yaml. *Was:* "Closes: REQ-CHART-096, REQ-CHART-097, REQ-CHART-098, REQ-CHART-099, REQ-PERF-116, REQ-CHART-127, REQ-CHART-128"; "Depends on: TASK-M2-58, TASK-M2-59, TASK-M2-14".
- *9 Nov 2026 (R-916):* split: the registry, refusals, labels and digest on the CPU (REQ-CHART-096 to REQ-CHART-099, REQ-CHART-127, REQ-CHART-128) are TASK-M2-66's, the digest written by TASK-M2-61's baseline; V1's decode time joins REQ-PERF-116 from TASK-M8-100 (applied per R-369). *Was (title):* "The primitive registry, the refusals, the labels and the bitwise digest"; *was (Acceptance):* the `size_only_refused`, `excluded_primitives`, `pair_table_complete`, `lowering_labels`, `presets_bitwise`, `round_trip_complete` and `primitive_class` lines, now at TASK-M2-66.
