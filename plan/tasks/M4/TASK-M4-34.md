# TASK-M4-34 — Region D: the third-body pair, its preset, and the angles and side ratios it carries

- **Milestone:** M4
- **Closes:** REQ-CHART-100, REQ-CHART-101, REQ-CHART-142, REQ-RENDER-123, REQ-CHART-165
- **Depends on:** TASK-M4-37, TASK-M4-33, TASK-M2-20, TASK-M4-06
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The first registered closed-form pair: `(ξ, η)` through the explicit-configuration path at any masses, region D as a preset of it, exact rest and, at equal masses, eq. 3's velocities (at other masses `q̄` is the Jacobi form of `p̂` at those masses, after `C`'s boost; no unequal-mass target runs eq. 3), its baked entry, and the interior angles and side ratios on top. Region D's GPU reference, the DECODE-preset golden of the lowering appendix's Anosova physical-frame row, reading region D, is this task's own, built on TASK-M4-06's variant table, and the task doesn't close until that reference reads region D as R-871 requires (R-914).

## References
- `docs/design/principia_chart_composition.md` § "8.2 The closed form"
- `docs/design/principia_chart_composition.md` § "8.3 The pairs it carries"
- `docs/design/principia_chart_composition.md` § "8.4 What the chart is: the domain"
- `docs/design/principia_chart_composition.md` § "8.6 The momentum, and the rescale per pixel"
- `docs/design/principia_chart_composition.md` § "12. Tests that can fail"
- `docs/design/principia_chart_composition.md` § "6.6 The symmetry rule"
- `docs/design/principia_chart_composition.md` § "10.4 The symmetry filter"
- `docs/design/principia_dd_validation_orbits.md` § "1.6 Agekyan–Anosova region D — the near-rest starts (R-592)"
- `docs/contracts/principia_lowering_contract.md` § "Appendix — worked enumeration of the current chart set"
- `docs/design/principia_colour_composition.md` § "6. Debug views as presets"
- `decisions.md` § "R-871 — Region D's chart is the plane, unmasked: the full disc rendered literally, every mirror and body swap declared, so the symmetry filter leaves exactly region D with no mask, on by default for the paper-frame presets only; its check, the filtered area 0.307 at equal masses and rest *(closes RQ-714; on RQ-738, which stays open)*"
- `decisions.md` § "R-914 — The stage 1 / stage 3 dependency cycle, RQ-752: option (a), the edge from TASK-M4-06 (the GPU compute variants) to TASK-M4-34 (region D) dropped; the GPU reference that reads region D moves into TASK-M4-34, which can't close until that reference reads region D as R-871 requires; R-871's substance unchanged, only the task that applies its link; TASK-M4-06 and the 217 tasks queued behind it released *(closes RQ-752)*"

## Deliverables
- `crates/kernel/src/chart/third_body.rs`: the pair, `Chart`, `ChartDecode`, `Tiltable`.
- `crates/kernel/src/stage/decode.rs`: its `DecodeVariant` and entry.
- The registry's rows and the preset.
- Choice V5 decided, with its evidence in the PR.
- Region D's velocity presets: exact rest the default, eq. 3 the second, eq. 3 reproducing Agekyan & Anosova's velocities in their own units before canonicalising, and the 19% spread at `I = 1` stated beside the preset (R-851; REQ-CHART-142).
- The lowering appendix's Anosova physical-frame row lowered as this pair, through `region_d_variant`: canonicalise entry with the signed canonicalisation, no region-D mask uniform, no fold to `(ξ, −η)` (R-871, R-914; REQ-CHART-165), and a per-chart checklist row in the PR mapping it to its registry entry and kernel variant.
- Region D's GPU reference: the DECODE-preset golden of the Anosova physical-frame chart in `fixtures/golden/decode_preset/`, rendered with native wgpu offscreen (R-110), reading region D through `in_region_d` on the unmasked full disc as R-871 requires (R-914; REQ-RENDER-123).

## Acceptance tests
- `cargo test -p kernel region_d_plane` and `cargo test -p engine region_d_variant` — Table 1's pixels against the import; edges; √ℓ; I = 1; the stereographic identity; controls: bodies swapped, q unscaled, the mass ratio inverted, d dropped at unequal masses, where Table 1's equal masses can't see it (REQ-CHART-100).
- `cargo test -p kernel closed_inverse_round_trip` — `forward(inverse(x)) = x`, every closed row at f64 and f32, each reading back its `Holds`; mutation: named per row, firing there (§ 12) (REQ-CHART-101).
- `cargo test -p engine region_d_velocity_presets` — REQ-CHART-142's arms and controls.
- Physics + code reviewers: the PR's checklist row for the lowering appendix's Anosova physical-frame row against the chart registry and the kernel variant, no mask uniform; backed by `cargo test -p engine region_d_variant` (REQ-CHART-165).
- `cargo xtask golden decode-preset` — the DECODE preset for the Anosova physical-frame chart matches a CPU-decoded reference image, without special-casing, region D read through `in_region_d` on the unmasked full disc (R-871); this task doesn't close until it does (R-914; REQ-RENDER-123).

## Notes
- **Build order (R-863):** stage 3, the primitive registry and composed axes, each primitive with its round-trip test. Its edge to stage 2 (TASK-M8-117) isn't made: it is in M4 and TASK-M8-117 in M8, so the edge would depend forward across milestones, and TASK-M8-117 already waits on it through TASK-M8-106 and TASK-M8-102, so the edge would close a cycle. RQ-744 (A) holds the choice.
- R-798's remap (REQ-CHART-093, TASK-M9-45) shares § 8.3's and § 8.2's lines: one source. PA-1's route is unchanged.
- The target frames (D11, Lehto's lens, Breen's) stay those targets' tested functions (R-72) unless RQ-729 rules for the presets (design § 8.5): this pair does not depend on them either way.
- The preset's base point, centre and window (R-466's definition of a preset) are undefined on this pair, which isn't two latent coordinate axes: RQ-711, read at TASK-M4-37, or a task that defines them under R-72 as TASK-M9-45 does (design § 20). Only those home values wait: the pair, the mask, the tests and the TASK-M4-06 edge do not.
- RQ-736's filter order overtakes RQ-714's mask for the preset: the full disc, signed, with D its filter's output (design §§ 8.4 and 10.4). What the preset draws until TASK-M8-120 lands is on RQ-736's list.
- Waits on its rulings item: RQ-713, RQ-714, RQ-736 for the preset's domain. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
- *7 Nov 2026 (R-845, R-851, R-843):* RQ-714's time and velocities are ruled; its outside, a mask or the full disc under the filter order, is still open, so the preset's domain waits on it, and RQ-728's labelled D is read with it.
- *8 Nov 2026 (R-873):* **Build order:** stage 3, released by dispatch after stage 2 (TASK-M8-117), the milestone kept (RQ-744 (A) (b)); still no edge to the M8 task.
- *8 Nov 2026 (R-871):* RQ-714 is ruled (b), with RQ-738 conflict 3 read as its (c): the preset is the full disc rendered literally, unmasked, the window holding the lens, the mirror and the three body swaps declared, region D drawn as a contour, no mask built; at equal masses and rest the symmetry filter (TASK-M8-120) leaves 0.307, D's own. REQ-CHART-100 says so. R-888 (RQ-711): the zoom row reads 1× at the preset's saved view.
- *9 Nov 2026 (R-914, RQ-752 (a)):* the edge from TASK-M4-06 to this task is dropped, since it closed a cycle through stage 2 (TASK-M8-117 waits, through TASK-M8-124, TASK-M8-48, TASK-M8-18 and TASK-M4-17, on TASK-M4-06). Region D's GPU reference, which reads region D, moves from TASK-M4-06 into this task, which can't close until it reads region D as R-871 requires (REQ-RENDER-123, split from REQ-RENDER-082), and with it the lowering appendix's Anosova physical-frame row, which is this pair (REQ-CHART-165, split from REQ-CHART-037; physics review 5829515661, P1, applied per R-369). This task gains TASK-M4-06 in Depends on, whose variant table and golden harness the reference renders through (applied per R-369); stage 3's release after stage 2 (R-873) is unchanged. *Was (Goal, the last sentence):* "TASK-M4-06 lowers its row and renders its golden once RQ-714 rules, by the edge TASK-M4-37 applies then." *Was:* "- **Closes:** REQ-CHART-100, REQ-CHART-101, REQ-CHART-142"; "- **Depends on:** TASK-M4-37, TASK-M4-33, TASK-M2-20".
