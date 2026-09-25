# TASK-M2-28 — Shape-sphere display: the 2-to-1 hemisphere fold and the equal-area projection

- **Milestone:** M2
- **Closes:** REQ-CHART-002, REQ-RENDER-026, REQ-CHART-047
- **Depends on:** TASK-M2-08, TASK-M2-14
- **Needs (earlier milestones):** REQ-GUI-001, REQ-SYS-009
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
The shape-sphere chart shows its redundancy honestly: `system_image = n-to-1` with n = 2 (2-to-1 over the φ hemispheres, which decode to the same system, R-141) carries its fold, and the chart renders one hemisphere and says so, or both with the redundancy flagged. It defaults to the equirectangular projection and offers an equal-area alternative (Mollweide or Hammer–Aitoff) for quantitative area comparisons, where the Lagrange poles aren't compressed.

## References
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 1 — The physics: what the 8D actually is"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"
- `docs/design/principia_chart_reference.md` § "3.3 The chart map"
- `docs/contracts/principia_inverse_encode_contract.md` § "Chart-aware validation"
- `decisions.md` § "R-59 — Fix D1 to D6 as listed *(sheet §8)*"
- `decisions.md` § "R-14 — One shape-sphere convention, the IC Inspector's *(closes RQ-12, corrects R-12's premise)*"
- `decisions.md` § "R-104 — The new `system_image` value is `DoubleCover` *(closes RQ-64)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-141 — The shape sphere is 2-to-1 over its φ hemispheres *(closes RQ-71, corrects R-104)*"

- `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
## Deliverables
- The hemisphere label / redundancy flag and the projection option on the shape-sphere chart (`crates/kernel/src/chart/shape_sphere.rs`, the label carried in the render; the GUI selector and toggle are REQ-GUI-161, M8).
- Golden images under `fixtures/golden/shape_sphere/`, rendered with native wgpu offscreen (R-110): one per projection and one per hemisphere label / redundancy flag (R-113).

## Acceptance tests
- `cargo xtask golden shape-sphere` — the shape-sphere render carries either one labelled hemisphere or both with a redundancy flag; `cargo test -p engine chart_descriptors` reports n-to-1 with n = 2 (R-141) (REQ-CHART-002).
- `cargo xtask golden shape-sphere-projection` — one golden per projection (equirectangular default, the equal-area alternative) of the shape-sphere chart (REQ-RENDER-026; the M8 selector and toggle are REQ-GUI-161, TASK-M8-06).
- Definition: the equal-area projection and its keying written into chart_reference §3.3 and approved by the physics reviewer (REQ-CHART-047).

## Notes
- RQ-71 ruled: R-141 — the φ hemispheres are reflection-equivalent, so the shape sphere is n-to-1 (n = 2); `DoubleCover` is retired.
- R-113 settles Gap G5: at M2 the two requirements are verified by golden image (one per projection and label); the projection selector and hemisphere toggle live in the Manifold view's Chart section at M8, shown when the chart is the shape sphere (REQ-GUI-161, TASK-M8-06).
- Gap G6: which equal-area projection (Mollweide or Hammer–Aitoff), and whether the projection is a different chart map (sim key, re-integrates) or a display remap.
- RQ-95 ruled: R-113 (Gap G5 above).
- Closes, for gaps the corpus leaves open: REQ-CHART-047 (R-72 definition) (classification accepted by R-132).
