# TASK-M4-34 — Region D: the third-body pair, its preset, and the angles and side ratios it carries

- **Milestone:** M4
- **Closes:** REQ-CHART-100, REQ-CHART-101
- **Depends on:** TASK-M4-37, TASK-M4-33, TASK-M2-20
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The first registered closed-form pair: `(ξ, η)` through the explicit-configuration path at any masses, region D as a preset of it, exact rest and, at equal masses, eq. 3's velocities (at other masses `q̄` is the Jacobi form of `p̂` at those masses, after `C`'s boost; no unequal-mass target runs eq. 3), its baked entry, and the interior angles and side ratios on top. TASK-M4-06 lowers its row and renders its golden once RQ-714 rules, by the edge TASK-M4-37 applies then.

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
- `cargo test -p kernel closed_inverse_round_trip` — `forward(inverse(x)) = x`, every closed row at f64 and f32, each reading back its `Holds`; mutation: named per row, firing there (§ 12) (REQ-CHART-101).

## Notes
- R-798's remap (REQ-CHART-093, TASK-M9-45) shares § 8.3's and § 8.2's lines: one source. PA-1's route is unchanged.
- The target frames (D11, Lehto's lens, Breen's) stay those targets' tested functions (R-72) unless RQ-729 rules for the presets (design § 8.5): this pair does not depend on them either way.
- The preset's base point, centre and window (R-466's definition of a preset) are undefined on this pair, which isn't two latent coordinate axes: RQ-711, read at TASK-M4-37, or a task that defines them under R-72 as TASK-M9-45 does (design § 20). Only those home values wait: the pair, the mask, the tests and the TASK-M4-06 edge do not.
- RQ-736's filter order overtakes RQ-714's mask for the preset: the full disc, signed, with D its filter's output (design §§ 8.4 and 10.4). What the preset draws until TASK-M8-120 lands is on RQ-736's list.
- Waits on its rulings item: RQ-713, RQ-714, RQ-736 for the preset's domain. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
