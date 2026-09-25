# TASK-M4-06 — The baked compute variants: chart × link × occupant × FTLE, with copy_index a uniform

- **Milestone:** M4
- **Closes:** REQ-CHART-037, REQ-SYS-024, REQ-INT-065, REQ-INT-069, REQ-RENDER-031, REQ-RENDER-082, REQ-INT-083
- **Depends on:** TASK-M4-05, TASK-M2-25, TASK-M2-28, TASK-M3-14
- **Needs (earlier milestones):** REQ-CHART-001, REQ-CHART-002, REQ-CHART-013, REQ-CHART-017, REQ-CHART-028, REQ-CHART-029, REQ-CHART-031, REQ-DEC-007, REQ-INT-005, REQ-PAY-026, REQ-INT-035, REQ-INT-043, REQ-RENDER-027
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~480 lines

## Goal
Compute-side lowering, per lowering Part 3 and its appendix: chart type, link selection, integrator occupant and the FTLE shadow co-computation are monomorphised variants; wrapper config goes in `SimUniforms`, chart params in uniforms, and the ensemble's `copy_index` is a uniform — each copy is the same kernel dispatched again, and at E = 0 no copy is dispatched (R-102). Every chart of the appendix lowers to its stated Φ map, extra uniforms and flags and ends in canonicalise → (m, r, p) → wrapper(occupant). Each render pixel integrates E+1 samples — copy 0 the un-jittered centre, copies 1..E at centred, footprint-scaled Halton (2,3) points (R-113) — plus each sample's Benettin shadow when the FTLE variant is bound. The two appendix charts M2 didn't build (the Burrau int lattice and Anosova) decode through the DECODE fragment preset.

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
- `decisions.md` § "R-141 — The shape sphere is 2-to-1 over its φ hemispheres *(closes RQ-71, corrects R-104)*"
- `decisions.md` § "R-157 — `DoubleCover` stays, for the full-range Burrau chart *(closes RQ-127, amends R-141)*"

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-117 — The lowering appendix's shape-sphere row uses (θ, φ) *(closes RQ-85)*"
- `decisions.md` § "R-124 — Apply the R-25, R-50 and R-102 follow-ups now *(closes RQ-92)*"
## Deliverables
- `crates/kernel`: the variant axes as type parameters (chart Φ, link set, occupant, FTLE on/off); the nine appendix rows lowered; the invariant charts write tagged payloads for infeasible pixels in-kernel; the Burrau integer lattice's per-cell dispatch.
- `crates/engine/src/variants.rs`: the variant key and pipeline table; the copy dispatch loop (copy_index 0..E as a uniform, same pipeline) and the per-copy offsets (copy 0 at the centre, copies 1..E at Halton (2,3) points 1..E minus ½, scaled to the footprint).
- DECODE-preset goldens for the Burrau int lattice and Anosova in `fixtures/golden/decode_preset/`, rendered with native wgpu offscreen (R-110).
- A per-chart checklist table in the PR mapping each appendix row to its registry entry and kernel variant.

## Acceptance tests
- Physics + code reviewers: one row per chart of the lowering appendix checked against the chart registry and the kernel variants (latent slice with its scale in q; shape sphere lowering (s, t) → (θ, φ) → n → (ρ̃, λ̃) by R-14's map (R-117) with `system_image: n-to-1`, n = 2 (R-141); (L_z,E)/(L_z,K) writing tagged payloads in-kernel; ternary mass; Euclid ν plane annotation axis; θ×K strip requiring lock for tilt; Burrau lattice per-cell dispatch, its `system_image` bijective (R-113); Anosova through canonicalise) (REQ-CHART-037); backed by `cargo test -p kernel chart_lowering_rows`.
- `cargo test -p engine variant_selection` — a navigation edit changes only uniform buffers; a tier change of N or FTLE selects a different pre-built variant; a change of E selects none (REQ-SYS-024).
- `cargo test -p engine occupant_ftle_baked` — no runtime uniform branches on occupant or FTLE; each (occupant, FTLE) combination is a distinct compiled variant; E = 0 and E = 3 use the same pipeline, the copies differing only in the `copy_index` uniform (REQ-INT-065).
- `cargo test -p engine ftle_off_no_shadow` — the FTLE-off variant contains no shadow state; no ensemble kernel variant exists; E = 0 dispatches no copies (REQ-INT-069).
- `cargo test -p engine trajectories_per_pixel` — trajectory count per pixel = (E+1) with the FTLE-off variant and 2(E+1) with the FTLE-on variant (the tier names "below Medium" / "from Medium up" arrive with the tier table, REQ-PERF-014 in M5) (REQ-RENDER-031).
- `cargo test -p engine copy_offsets` — copy 0's offset is exactly zero; copy k's offset equals (Halton_k − ½) scaled to the footprint for k = 1..E (REQ-INT-083).
- `cargo xtask golden decode-preset` — the DECODE preset for the Burrau int lattice and the Anosova physical-frame chart matches a CPU-decoded reference image, without special-casing (REQ-RENDER-082).

## Notes
- RQ-71 ruled: R-141 — the shape sphere's row carries `system_image: n-to-1`, n = 2; `DoubleCover` stays only for the full-range Burrau chart (R-157) (REQ-CHART-037).
- RQ-85 ruled: R-117 — the appendix's shape-sphere row is conformed to R-14's (θ, φ) map (applied in step 7).
- RQ-92 ruled: R-124 — the dispatch-shape conflict is settled (copies dispatched again, R-102); this task is unblocked.
- RQ-95 ruled: R-113 — REQ-CHART-014's lattice clause is REQ-CHART-037's here; REQ-RENDER-027's Burrau int lattice and Anosova half is REQ-RENDER-082, closed here.
- RQ-98 ruled: R-113 — REQ-INT-072's copy offsets are REQ-INT-083, closed here (H and σ²_T stay in TASK-M5-18); REQ-RENDER-031's verify is parameterised by (E, FTLE on/off), the tier names checked by REQ-PERF-014 in M5.
