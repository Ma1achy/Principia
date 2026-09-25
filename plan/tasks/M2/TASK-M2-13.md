# TASK-M2-13 — The ternary mass plot, mixed-axis charts and per-pixel mass

- **Milestone:** M2
- **Closes:** REQ-CHART-026, REQ-CHART-008, REQ-DEC-006
- **Depends on:** TASK-M2-06, TASK-M2-10
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~360 lines

## Goal
`MassSimplex` fixes the geometry at ν₀ and maps x = u, y = (1−u)v, m₀ = 1−x−y, m₁ = x, m₂ = y, shrunk toward the barycentre by ε_m = 10⁻⁴, with rest momenta and the Burrau point as an overlay marker. Mixed-axis charts take each axis's latent coordinate or derived quantity with its warp, fix the other six by a stated convention, compose as Φ_mixed(u, v) = D(z(u, v)) with no new GPU logic, and inherit both axes' feasibility. Masses are always produced by the decode: every chart declares `requires_per_pixel_mass` (true for the ternary plot, the Burrau family and mixed charts with a mass axis), and under it the kernel never reads `SimUniforms.m[3]`.

## References
- `docs/design/principia_chart_reference.md` § "4.5 The Burrau-family chart maps"
- `docs/design/principia_chart_reference.md` § "4.4 Relaxations — each gives another chart"
- `decisions.md` § "R-22 — Body indices are 0-based; pair ids name the opposite side *(CD-2, amended)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "The four axis kinds — a closed set"
- `docs/design/principia_chart_reference.md` § "1.3 Mixed-axis charts"
- `docs/contracts/principia_inverse_encode_contract.md` § "Chart-aware validation"
- `docs/design/principia_dd_decoder.md` § "2. Consolidated contract (every clause that binds this component)"
- `docs/contracts/principia_lowering_contract.md` § "Compute side"

## Deliverables
- `crates/kernel/src/chart/mass_simplex.rs` and `crates/kernel/src/chart/mixed.rs`.
- The `requires_per_pixel_mass` declaration on every chart built so far.
- Tests `crates/kernel/tests/mass_mixed.rs` and `crates/engine/tests/per_pixel_mass_gpu.rs`.

## Acceptance tests
- `cargo test -p kernel ternary_mass` — every pixel's masses are ≥ ε_m and sum to 1; the Burrau marker lands at the Burrau masses (REQ-CHART-026).
- `cargo test -p kernel mixed_axis` — the (α, p_ρ) chart decodes through the shared decoder; feasibility is the conjunction of both axes' checks (REQ-CHART-008).
- `cargo test -p engine per_pixel_mass` — for each chart with `requires_per_pixel_mass`, `SimUniforms.m[3]` set to garbage leaves per-pixel decoded masses, descriptors and decode labels unchanged on the GPU decode stage (REQ-DEC-006).

## Notes
- REQ-DEC-006's "outcomes unchanged" can only be checked on decode-time outcomes (DEGENERATE, infeasible, the descriptor) until the integrator lands (M3/M4).
- ν₀ is supplied by the lock (lowering appendix); with no lock, lookup's partial-spec default ν = 1/2 applies (inverse_encode Part 6).
