# TASK-M4-34 — Region D: the third-body pair, its preset, and the angles and side ratios it carries

- **Milestone:** M4
- **Closes:** REQ-CHART-100, REQ-CHART-101
- **Depends on:** TASK-M4-37, TASK-M4-33, TASK-M2-20
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The first registered closed-form pair: `(ξ, η)` through the explicit-configuration path at any masses, region D as a preset of it, exact rest and eq. 3's velocities, its baked entry, and the interior angles and side ratios on top. TASK-M4-06 lowers its row and renders its golden once RQ-714 rules, by the edge TASK-M4-37 applies then.

## References
- `docs/design/principia_chart_composition.md` § "8.2 The closed form"
- `docs/design/principia_chart_composition.md` § "8.3 The pairs it carries"
- `docs/design/principia_chart_composition.md` § "8.4 What the chart is: the domain"
- `docs/design/principia_chart_composition.md` § "8.6 The momentum, and the rescale per pixel"
- `docs/design/principia_chart_composition.md` § "12. Tests that can fail"
- `docs/design/principia_dd_validation_orbits.md` § "1.6 Agekyan–Anosova region D — the near-rest starts (R-592)"

## Deliverables
- `crates/kernel/src/chart/third_body.rs`: the pair, `Chart`, `ChartDecode`, `Tiltable`.
- `crates/kernel/src/stage/decode.rs`: its `DecodeVariant` and entry.
- The registry's rows and the preset.
- Choice V5 decided, with its evidence in the PR.

## Acceptance tests
- `cargo test -p kernel region_d_plane` and `cargo test -p engine region_d_variant` — Table 1's pixels against the import; edges; √ℓ; I = 1; the stereographic identity; controls: bodies swapped, q unscaled, the mass ratio inverted, d dropped at unequal masses, where Table 1's equal masses can't see it (REQ-CHART-100).
- `cargo test -p kernel closed_inverse_round_trip` — every closed row at f64 and f32; control: a sign flipped (REQ-CHART-101).

## Notes
- R-798's remap (REQ-CHART-093, TASK-M9-45) shares § 8.3's and § 8.2's lines: one source. PA-1's route is unchanged.
- The target frames (D11, Lehto's lens, Breen's) stay those targets' tested functions (R-72) unless RQ-729 rules for the presets (design § 8.5): this pair does not depend on them either way.
- Waits on its rulings item: RQ-713, RQ-714. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
