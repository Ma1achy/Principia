# TASK-M4-06 — The baked compute variants: chart × link × occupant × FTLE, with copy_index a uniform

- **Milestone:** M4
- **Closes:** REQ-CHART-037, REQ-SYS-024, REQ-INT-065, REQ-INT-069, REQ-RENDER-031
- **Depends on:** TASK-M4-05, TASK-M2-28, TASK-M3-14
- **Needs (earlier milestones):** REQ-CHART-001, REQ-CHART-002, REQ-CHART-013, REQ-CHART-017, REQ-CHART-028, REQ-CHART-029, REQ-CHART-031, REQ-DEC-007, REQ-INT-005, REQ-PAY-026, REQ-INT-035, REQ-INT-043
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~480 lines

## Goal
Compute-side lowering, per lowering Part 3 and its appendix: chart type, link selection, integrator occupant and the FTLE shadow co-computation are monomorphised variants; wrapper config goes in `SimUniforms`, chart params in uniforms, and the ensemble's `copy_index` is a uniform — each copy is the same kernel dispatched again, and at E = 0 no copy is dispatched (R-102). Every chart of the appendix lowers to its stated Φ map, extra uniforms and flags and ends in canonicalise → (m, r, p) → wrapper(occupant). Each render pixel integrates E+1 samples, plus each sample's Benettin shadow when the FTLE variant is bound.

## References
- `docs/contracts/principia_lowering_contract.md` § "Appendix — worked enumeration of the current chart set"
- `decisions.md` § "R-26 — Every chart declares its domain function *(CD-6)*"
- `decisions.md` § "R-27 — Both Burrau charts are kept, each labelled with its quotient *(CD-7)*"
- `decisions.md` § "R-83 — The slice scale lives in q *(closes RQ-34)*"
- `decisions.md` § "R-104 — The new `system_image` value is `DoubleCover` *(closes RQ-64)*"
- `docs/contracts/principia_lowering_contract.md` § "Compute side"
- `docs/contracts/principia_lowering_contract.md` § "Part 5 — The resolution function (the "switch", concretely)"
- `decisions.md` § "R-102 — The ensemble isn't a baked variant *(closes RQ-62)*"
- `docs/contracts/principia_integrator_contract.md` § "Part 3 — Parameter ownership (who owns what)"
- `docs/contracts/principia_render_contract.md` § "Part 8 — Errata against the older design docs"
- `docs/contracts/principia_lowering_contract.md` § "Part 3a — The uniform read-side interface (tier features degrade by NaN, not by struct shape)"
- `docs/design/principia_dd_integrator.md` § "2. Consolidated contract"
- `docs/contracts/principia_canonical_spec.md` § "5. Temporal & rendering model *(authoritative: `temporal_architecture_note`, `render_contract`, `scheduler_contract`, `checkerboard_contract`)*"
- `docs/design/principia_core_design.md` § "4. Integrate and colour are separate passes — and now separate *mechanisms*"
- `docs/contracts/principia_lowering_contract.md` § "Part 2 — Two assembly mechanisms (the substrate split; the old "one mechanism" claim retires)"

## Deliverables
- `crates/kernel`: the variant axes as type parameters (chart Φ, link set, occupant, FTLE on/off); the nine appendix rows lowered; the invariant charts write tagged payloads for infeasible pixels in-kernel; the Burrau integer lattice's per-cell dispatch.
- `crates/engine/src/variants.rs`: the variant key and pipeline table; the copy dispatch loop (copy_index 0..E as a uniform, same pipeline).
- A per-chart checklist table in the PR mapping each appendix row to its registry entry and kernel variant.

## Acceptance tests
- Physics + code reviewers: one row per chart of the lowering appendix checked against the chart registry and the kernel variants (latent slice with its scale in q; shape sphere with `system_image: DoubleCover`; (L_z,E)/(L_z,K) writing tagged payloads in-kernel; ternary mass; Euclid ν plane annotation axis; θ×K strip requiring lock for tilt; Burrau lattice per-cell dispatch; Anosova through canonicalise) (REQ-CHART-037); backed by `cargo test -p kernel chart_lowering_rows`.
- `cargo test -p engine variant_selection` — a navigation edit changes only uniform buffers; a tier change of N or FTLE selects a different pre-built variant; a change of E selects none (REQ-SYS-024).
- `cargo test -p engine occupant_ftle_baked` — no runtime uniform branches on occupant or FTLE; each (occupant, FTLE) combination is a distinct compiled variant; E = 0 and E = 3 use the same pipeline, the copies differing only in the `copy_index` uniform (REQ-INT-065).
- `cargo test -p engine ftle_off_no_shadow` — the FTLE-off variant contains no shadow state; no ensemble kernel variant exists; E = 0 dispatches no copies (REQ-INT-069).
- `cargo test -p engine trajectories_per_pixel` — trajectory count per pixel = (E+1) with the FTLE-off variant and 2(E+1) with the FTLE-on variant (the tier names "below Medium" / "from Medium up" arrive with the tier table, REQ-PERF-014 in M5) (REQ-RENDER-031).

## Notes
- Open RQs: RQ-71.
- REQ-CHART-037 waits on RQ-71 (whether the shape sphere is `DoubleCover` or a 2-to-1 fold); the row is checked against the ruling.
- Blocked on the dispatch-shape gap (TASK-M4-05 notes) until ruled.
- Copy placement (Halton offsets) is REQ-INT-072 in M5; see the report's Gaps for what an M4 copy integrates.
