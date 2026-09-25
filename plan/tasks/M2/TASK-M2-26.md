# TASK-M2-26 — The ROUNDTRIP preset and the one encode entry

- **Milestone:** M2
- **Closes:** REQ-RENDER-025, REQ-ENC-002
- **Depends on:** TASK-M2-20, TASK-M2-23, TASK-M2-25
- **Needs (earlier milestones):** REQ-COL-003, REQ-RENDER-005, REQ-COL-001
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-5, PIT-9
- **Size:** ~320 lines

## Goal
The ROUNDTRIP fragment preset renders per pixel the z → D → E → D physical residual, recomputed in the fragment and log-scaled: dark everywhere except clamp boundaries, feasibility edges and the mirror tie, each tagged expected — the encode-side sibling of the invariant-chart gradient. No kernel variant is involved (R-75). With every consumer now in place, encode is confirmed as the one path for entered states (lock re-centre, physical IC entry, Burrau (m, n) entry, invariant-pair lookup, literature import, the T-tests, ROUNDTRIP) and never for clicked pixels.

## References
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 8 — Round-trip contracts and the new debug view"
- `docs/design/principia_dd_encode.md` § "4. Seams (obligations → integration tests)"
- `docs/design/principia_dd_decoder.md` § "4. Seams (its side of each — the integration-test list)"
- `decisions.md` § "R-41 — `DEBUG_MODE` uses the baked variants, not flag bits *(RS-1 (b))*"
- `docs/contracts/principia_render_contract.md` § "Cross-check views (the seams)"
- `docs/design/principia_debug_tooling_plan.md` § "A. Kernel debug dispatch modes (skip integration; reuse payload slots as scratch)"
- `decisions.md` § "R-75 — The kernel keeps one debug mode *(closes RQ-26)*"
- `docs/design/principia_colour_composition.md` § "6. Debug views as presets"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 1 — What encode is: a quotient map onto a section"
- `docs/design/principia_dd_encode.md` § "1. What it is"
- `docs/design/principia_dd_encode.md` § "2. Consolidated contract"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 7 — Ground-truth ingestion (the validation programme's demand)"
- `docs/contracts/principia_canonical_spec.md` § "3. The physics & manifold model *(authoritative: `chart_decoder_contract`, `dd_decoder`)*"
- `docs/design/principia_systems_architecture.md` § "0. The ladder — the organising abstraction"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_core_design.md` § "5. Canonicalisation is the one seam to (m, r, p)"
- `decisions.md` § "R-82 — One mirror test, one seed rule *(closes RQ-33)*"

## Deliverables
- The ROUNDTRIP preset (`crates/render/src/presets/roundtrip.rs` + WGSL), locked, with the tagged-expected mask (clamp, feasibility edge, mirror tie).
- Golden suite `fixtures/golden/roundtrip_preset/` (latent and (L_z, E) charts).

## Acceptance tests
- `cargo xtask golden roundtrip-preset` — ROUNDTRIP render of the latent and (L_z, E) charts: residual below ε_phys (REQ-ENC-024) except at tagged-expected regions; the kernel variant set is unchanged (exactly the bring-up variant) (REQ-RENDER-025).
- Review (physics + code): every listed host-side consumer calls the one encode entry point; the ROUNDTRIP view is a fragment preset, not a kernel mode (R-75); clicked-pixel locks do not call encode (REQ-ENC-002).

## Notes
- Gaps G1, G2 and G15 apply (the WGSL encode's provenance, nonlinear charts, ‖·‖_phys).
- Waits on RQ-84 (`REVIEW_QUEUE.md`): One decode source vs "the two decode ports".
