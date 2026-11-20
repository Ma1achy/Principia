# TASK-M4-06 — The baked compute variants: chart × link × occupant × FTLE, with copy_index a uniform

- **Milestone:** M4
- **Closes:** REQ-CHART-037, REQ-SYS-024, REQ-INT-065, REQ-INT-069, REQ-RENDER-031, REQ-RENDER-082, REQ-INT-083, REQ-CHART-152
- **Depends on:** TASK-M4-05, TASK-M2-25, TASK-M2-28, TASK-M3-14, TASK-M0-44, TASK-M2-66
- **Needs (earlier milestones):** REQ-CHART-001, REQ-CHART-002, REQ-CHART-013, REQ-CHART-017, REQ-CHART-028, REQ-CHART-029, REQ-CHART-031, REQ-DEC-007, REQ-INT-005, REQ-PAY-026, REQ-INT-035, REQ-INT-043, REQ-RENDER-027, REQ-SYS-074
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~480 lines

## Goal
Compute-side lowering, per lowering Part 3 and its appendix: chart type, link selection, integrator occupant and the FTLE shadow co-computation are monomorphised variants; wrapper config goes in `SimUniforms`, chart params in uniforms, and the ensemble's `copy_index` is a uniform — each copy is the same kernel dispatched again, and at E = 0 no copy is dispatched (R-102). Every chart of the appendix but the Anosova physical-frame row lowers to its stated Φ map, extra uniforms and flags and ends in canonicalise → (m, r, p) → wrapper(occupant), the shape sphere's lowered decode running canonicalise with step 3, the mirror, not taken, as its CPU decode does (R-544). Each render pixel integrates E+1 samples — copy 0 the un-jittered centre, copies 1..E at centred, footprint-scaled Halton (2,3) points (R-113) — plus each sample's Benettin shadow when the FTLE variant is bound. Of the two appendix charts M2 didn't build, the Burrau int lattice decodes through the DECODE fragment preset here; the Anosova physical-frame row is region D's third-body pair, TASK-M4-34's, which lowers it through `region_d_variant` and renders its golden, region D's GPU reference (R-914).

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
- `decisions.md` § "R-297 — Fast-math per shader stage: off for compute by default, an explicit and recorded opt-in; display may keep it *(amends R-84, R-116)*"
- `decisions.md` § "R-544 — The shape sphere chart is the whole sphere: pixel `(θ, φ)` decodes to the system whose signed `n̂` is that point, R-141's decode gauge removed for it, no hemisphere toggle, no remap and no fold at the equator, every pixel its own system; the mirror a fact about fates, tested and never identified, and shown as information in the initial-condition constructor *(amends R-113, R-141, R-537, R-59, R-178, R-408, R-503, R-538 and R-542; withdraws R-412; settles RQ-498 and RQ-504 where they turn on the hemisphere)*"
- `decisions.md` § "R-876 — An f32 chart may refuse a pixel inside the float-wide band at the warp's floor, the refusal labelled as rounding; the edge walk is repeated on the GPU when the shader decodes these charts *(closes RQ-740; refines R-708)*"

## Deliverables
- `crates/kernel`: the variant axes as type parameters (chart Φ, link set, occupant, FTLE on/off); the appendix's rows lowered but the Anosova physical-frame row, TASK-M4-34's (R-914); the invariant charts write tagged payloads for infeasible pixels in-kernel; the Burrau integer lattice's per-cell dispatch.
- `crates/engine/src/variants.rs`: the variant key and pipeline table, each pipeline created through TASK-M0-44's compute-pipeline entry point with the compute fast-math setting (R-297); the copy dispatch loop (copy_index 0..E as a uniform, same pipeline) and the per-copy offsets (copy 0 at the centre, copies 1..E at Halton (2,3) points 1..E minus ½, scaled to the footprint).
- The DECODE-preset golden for the Burrau int lattice in `fixtures/golden/decode_preset/`, rendered with native wgpu offscreen (R-110).
- A per-chart checklist table in the PR mapping each appendix row but the Anosova physical-frame row (TASK-M4-34's, R-914) to its registry entry and kernel variant.

## Acceptance tests
- Physics + code reviewers: one row per chart of the lowering appendix checked against the chart registry and the kernel variants (latent slice with its scale in q; shape sphere lowering (s, t) → (θ, φ) → n → (ρ̃, λ̃) by R-14's map (R-117) with `system_image: bijective` over the whole sphere and canonicalise's step 3 not taken (R-544; *was:* "`system_image: n-to-1`, n = 2 (R-141)"); (L_z,E)/(L_z,K) writing tagged payloads in-kernel; ternary mass; Euclid ν plane annotation axis; θ×K strip requiring lock for tilt; Burrau lattice per-cell dispatch, its `system_image` bijective (R-113); the Anosova row is TASK-M4-34's, REQ-CHART-165, R-914) (REQ-CHART-037); backed by `cargo test -p kernel chart_lowering_rows`.
- `cargo test -p engine variant_selection` — a navigation edit changes only uniform buffers; a tier change of N or FTLE selects a different pre-built variant; a change of E selects none (REQ-SYS-024).
- `cargo test -p engine occupant_ftle_baked` — no runtime uniform branches on occupant or FTLE; each (occupant, FTLE) combination is a distinct compiled variant; E = 0 and E = 3 use the same pipeline, the copies differing only in the `copy_index` uniform (REQ-INT-065).
- `cargo test -p engine ftle_off_no_shadow` — the FTLE-off variant contains no shadow state; no ensemble kernel variant exists; E = 0 dispatches no copies (REQ-INT-069).
- `cargo test -p engine trajectories_per_pixel` — trajectory count per pixel = (E+1) with the FTLE-off variant and 2(E+1) with the FTLE-on variant (the tier names "below Medium" / "from Medium up" arrive with the tier table, REQ-PERF-014 in M5) (REQ-RENDER-031).
- `cargo test -p engine copy_offsets` — copy 0's offset is exactly zero; copy k's offset equals (Halton_k − ½) scaled to the footprint for k = 1..E (REQ-INT-083).
- `cargo xtask golden decode-preset` — the DECODE preset for the Burrau int lattice matches a CPU-decoded reference image, without special-casing (REQ-RENDER-082).
- `cargo test -p engine gpu_f32_band_edge_walk` — the GPU edge walk over HeldLz, HeldK and the Burrau charts, run in the GPU job with its criterion written first, its result in the PR: band refusals labelled rounding, no GPU-accepted non-system; controls: a band refusal drawn as the edge fails, an accepted non-system fails (REQ-CHART-152).

## Notes
- RQ-71 ruled: R-141 — the shape sphere's row carries `system_image: n-to-1`, n = 2; `DoubleCover` stays only for the full-range Burrau chart (R-157) (REQ-CHART-037).
- R-544 (21 Oct 2026) amends that: the shape sphere's row is bijective over the whole sphere, and its baked decode skips canonicalise's mirror step, so the GPU variant generated from the shared source never restores R-141's gauge; `DoubleCover` stays the full-range Burrau chart's (physics review 5603309678 of PR #273, P3; applied per R-369).
- RQ-85 ruled: R-117 — the appendix's shape-sphere row is conformed to R-14's (θ, φ) map (applied in step 7).
- RQ-92 ruled: R-124 — the dispatch-shape conflict is settled (copies dispatched again, R-102); this task is unblocked.
- RQ-95 ruled: R-113 — REQ-CHART-014's lattice clause is REQ-CHART-037's here; REQ-RENDER-027's Burrau int lattice and Anosova half is REQ-RENDER-082, closed here.
- RQ-98 ruled: R-113 — REQ-INT-072's copy offsets are REQ-INT-083, closed here (H and σ²_T stay in TASK-M5-18); REQ-RENDER-031's verify is parameterised by (E, FTLE on/off), the tier names checked by REQ-PERF-014 in M5.
- R-297: every compute pipeline is created through TASK-M0-44's entry point, which takes the fast-math setting (off by default); the setting joins the sim key in TASK-M4-08.
- R-481 G7 (20 Oct 2026): link parameters are a uniform, not a variant axis; the link ids stay baked. The uniform is TASK-M4-21's.
- *8 Nov 2026 (R-876, R-871):* RQ-740 is ruled (R-876): on the f32 charts R-708 covers, a refusal inside the band one float wide at the warp's floor is labelled rounding, never drawn as the edge, and the edge walk is repeated on the GPU when the shader decodes these charts (REQ-CHART-152). RQ-714 (R-871): this task's golden reads `in_region_d` with region D drawn as a contour until TASK-M8-120 lands; TASK-M2-58 applies the edge to TASK-M4-34. *Was:* "- **Closes:** REQ-CHART-037, REQ-SYS-024, REQ-INT-065, REQ-INT-069, REQ-RENDER-031, REQ-RENDER-082, REQ-INT-083"
- *9 Nov 2026 (R-914, RQ-752 (a)):* the edge to TASK-M4-34 is dropped and never applied: it closed a cycle through stage 2 (R-873). The GPU reference that reads region D, the Anosova physical-frame chart's DECODE-preset golden, moves into TASK-M4-34 (REQ-RENDER-123, split from REQ-RENDER-082); this task keeps the Burrau int lattice's golden. The Anosova physical-frame row is region D's third-body pair (chart_composition § 8.2), so its lowering goes too: TASK-M4-34 lowers it through `region_d_variant`, unmasked and never folded to `(ξ, −η)` (REQ-CHART-165, split from REQ-CHART-037; physics review 5829515661, P1, applied per R-369). It waits on no task of the chart composition design, and the tasks behind it are released. *Was (Goal):* "Every chart of the appendix lowers to its stated Φ map,"; "The two appendix charts M2 didn't build (the Burrau int lattice and Anosova) decode through the DECODE fragment preset." *Was (Deliverables):* "the nine appendix rows lowered;". *Was (Acceptance, the clause):* "; Anosova through canonicalise)". *Was (Deliverables, the checklist):* "mapping each appendix row to its registry entry". *Was (Deliverables):* "- DECODE-preset goldens for the Burrau int lattice and Anosova in `fixtures/golden/decode_preset/`, rendered with native wgpu offscreen (R-110)." *Was (Acceptance):* "`cargo xtask golden decode-preset` — the DECODE preset for the Burrau int lattice and the Anosova physical-frame chart matches a CPU-decoded reference image, without special-casing (REQ-RENDER-082)."
- *9 Nov 2026 (R-916):* the chart core is M2's (R-916, R-917, R-919), so this header follows plan/tasks.yaml. *Was:* "Depends on: TASK-M4-05, TASK-M2-25, TASK-M2-28, TASK-M3-14, TASK-M0-44".
- *9 Nov 2026 (R-916):* split: REQ-CHART-152's CPU arm, `cargo test -p kernel f32_band_label`, and the band's definition in chart_reference § 2.1 are TASK-M2-66's REQ-CHART-169; this task keeps the GPU edge walk. *Was (Acceptance):* the `f32_band_label` line and the band's review line; *was (Deliverables):* "The f32 band defined in chart_reference § 2.1".
- *9 Nov 2026 (code review 5834056023 of PR #465):* the GPU edge walk's acceptance line names its command, `cargo test -p engine gpu_f32_band_edge_walk`, run in the GPU job. *Was (Acceptance, the start):* "GPU edge walk: the GPU edge walk over HeldLz".
- *12 Nov 2026 (R-926 item 5; the chart audit's F7; applied per R-369: a mechanical consequence of a ruling):* the integrator entry holds no chart code (REQ-INT-116), so chart type and link selection are no longer compute variants: the set is occupant × FTLE shadow, `copy_index` a uniform. The Goal's chart and link clauses ("chart type, link selection, integrator occupant and the FTLE shadow co-computation are monomorphised variants") and the per-chart Φ lowering are kept as their *Was* until this file is rewritten.
