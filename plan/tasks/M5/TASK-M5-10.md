# TASK-M5-10 — Memory estimator: payload, word buffer and render targets

- **Milestone:** M5
- **Closes:** REQ-PERF-015, REQ-PERF-018, REQ-PERF-025, REQ-PERF-031, REQ-PERF-093
- **Depends on:** TASK-M5-02, TASK-M4-11
- **Needs (earlier milestones):** REQ-PAY-003, REQ-PAY-008, REQ-PERF-005, REQ-PERF-008
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~280 lines

## Goal
One memory estimator reproduces payload §7's table and memory_tiers §4's recomputed table: payload
`(E+1) × bytes × render_px` at the ledger's current widths (144 B FTLE-on / 96 B FTLE-off), the word buffer
`16 B × (E+1) × live_pixels` when symbolic features are on, render targets `k_d × display_px × 4 + k_r × render_px × 4`
(k_d ≈ 3, k_r ≈ 3), and the process estimate with margin (bake texture, staging, transients, overhead) used by the fit
check.

## References
- `docs/contracts/principia_canonical_spec.md` § "6. Memory & deployment model *(authoritative: `memory_tiers`, `caching_contract`, `deep_zoom`, `systems_architecture`)*"
- `docs/design/principia_dd_simstate_payload.md` § "7. Memory"
- `docs/design/principia_dd_generation_root.md` § "3.3a The word buffer — a parallel cold buffer"
- `docs/design/principia_memory_tiers.md` § "3. Memory model — payload + render targets"
- `docs/design/principia_memory_tiers.md` § "Total GB (payload + render targets) by tier × display resolution"
- `docs/design/principia_memory_tiers.md` § "Principia — quality tiers & memory model (auto-mode reference)"
- `docs/design/principia_memory_tiers.md` § "8. Caveats"
- `docs/design/principia_memory_tiers.md` § "6. Auto-mode tier selection"
- `decisions.md` § "R-40 — Memory tiers and the quality device key off `eps` *(PL-5)*"
- `decisions.md` § "R-59 — Fix D1 to D6 as listed *(sheet §8)*"
- `decisions.md` § "R-73 — Apply the whole ruling-follow-up checklist now *(closes RQ-56)*"

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
## Deliverables
- `crates/engine/src/memory/estimate.rs`: payload, word, render-target and process terms; the fit check sums every term
  and applies the margin.
- Unit tests `crates/engine/tests/memory_estimate.rs` against the two doc tables.
- `cargo xtask bench memory-fit-margin`: estimator figures against measured process memory across tiers and resolutions, and the proposed margin.

## Acceptance tests
- `cargo test -p engine render_target_bytes` — the allocator's render-target bytes match the formula at several display sizes and render_scale values (REQ-PERF-015).
- `cargo test -p engine payload_estimate_table` — estimator at 1080p E=3 FTLE-on returns 1.194 + 0.133 = 1.327 GB; 4K E=3 FTLE-on 5.308 GB; the estimator's figure is unchanged with checkerboard on and off (R-113) (REQ-PERF-018).
- `cargo test -p engine memory_tiers_table` — estimator output for each tier × {1080p, 1440p, 4K} equals the formula at 144/96 B; the Ultra and Extreme rows and the totals that depend on them are provisional and are recomputed when REQ-PERF-086 lands (R-137) (REQ-PERF-025).
- Review checklist (perf) — the fit check sums every term and applies the margin (REQ-PERF-031).
- `cargo xtask bench memory-fit-margin` — the proposal records the margin with its method: estimator figures against measured process memory across tiers and resolutions; reviewer-checked, confirmed by the human at the M5 gate and recorded in decisions.md (REQ-PERF-093).

## Notes
- RQ-98 ruled: R-113 — REQ-PERF-005's "the estimator must credit checkerboard with no memory saving" joins REQ-PERF-018 here.
- RQ-100 ruled: R-113 — REQ-PERF-074's margin half is REQ-PERF-093, an M5 calibration closed here (memory_tiers §8 gives no number); the fps thresholds stay in M8 (TASK-M8-25).
- Calibrations proposed here (R-71; human confirmation at the M5 gate, then recorded in decisions.md): REQ-PERF-093.
