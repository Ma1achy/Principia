# TASK-M2-10 — The Burrau family: Euclid's parametrisation, the ν axis and the Euclid plane

- **Milestone:** M2
- **Closes:** REQ-CHART-021, REQ-CHART-022, REQ-CHART-023, REQ-CHART-013, REQ-SYS-016, REQ-VAL-016, REQ-VAL-118
- **Depends on:** TASK-M2-07, TASK-M2-09, TASK-M1-10
- **Needs (earlier milestones):** REQ-PAY-004, REQ-TOOL-022, REQ-PAY-020
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~450 lines

## Goal
The Burrau family enters as ordinary Φ maps into the shared decoder: Euclid's a = m²−n², b = 2mn, c = m²+n² with the normalised Burrau convention (right angle at the origin, each mass equal to its opposite side, m = (c, b, a)/(a+b+c)), stated as the convention in use; the continuous ν = n/m axis with θ(ν) = arctan((1−ν²)/(2ν)); the (ν, K) chart taking K from §2's construction; and the Euclid plane, m(u) = m_min(m_max/m_min)^u on [1, 32] as an annotation axis and ν(v) linear on [1/32, 31/32], decoding from ν alone (Burrau masses, rest) with the primitive-triple overlay. Body indices are 0-based throughout and the 1-based literature convention is translated at the boundary (R-22).

## References
- `docs/design/principia_chart_reference.md` § "4.1 The discrete family — Euclid's parametrisation"
- `docs/design/principia_chart_reference.md` § "4.2 Positions and masses — Burrau convention"
- `decisions.md` § "R-22 — Body indices are 0-based; pair ids name the opposite side *(CD-2, amended)*"
- `docs/design/principia_chart_reference.md` § "4.3 Continuous interpolation — the `ν` axis"
- `docs/design/principia_chart_reference.md` § "4.5 The Burrau-family chart maps"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"
- `docs/design/principia_dd_decoder.md` § "6. Deferred / flagged"
- `docs/design/principia_chart_reference.md` § "Chart reference — the maths, for implementation"
- `docs/design/principia_dd_simstate_payload.md` § "`sample_descriptor` (low 16 bits of `packed_a`) — 10 used, rest reserved"
- `docs/design/principia_dd_generation_root.md` § "3.1 `sample_descriptor` (u32)"
- `docs/design/principia_dd_generation_root.md` § "6. Deferred / flagged"
- `docs/contracts/principia_canonical_spec.md` § "11. Still open / downstream (not yet fully in the corpus)"
- `docs/design/principia_debug_tooling_plan.md` § "B. Payload field views — `sample_descriptor` (bit-packed u32)"
- `docs/design/principia_chart_reference.md` § "5.2 Tests that can fail"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/kernel/src/chart/burrau.rs` (the embed γ: ν → configuration and masses, the (ν, K) chart, the Euclid plane) and `crates/kernel/src/burrau.rs` (Euclid triples, the 1-based → 0-based translation).
- The Euclid plane's annotation-axis descriptor and a statistics guard that refuses 2D density on it.
- Tests `crates/kernel/tests/burrau.rs`; the ν = ½ tolerance proposal (`fixtures/gates/burrau_nu_half/`).

## Acceptance tests
- `cargo test -p kernel burrau_euclid` — (m, n) = (2, 1) gives (3, 4, 5) and masses (5, 4, 3)/12; the §4.1 rows (3,2)→(5,12,13), (4,1)→(15,8,17), (4,3)→(7,24,25) reproduce; Jacobi ρ = (a/c, 0), λ = (−ab/(c(b+c)), b/c); the chart states the normalised convention (REQ-CHART-021).
- `cargo test -p kernel burrau_nu_continuous` — positions, masses and Jacobi vectors are continuous in ν; primitive-triple ν values reproduce the discrete family; the (ν, K) chart's momenta come from the §2 construction (REQ-CHART-022).
- `cargo test -p kernel euclid_plane` — varying u at fixed v gives identical ICs; the overlay marks exactly the primitive-triple lattice points (m, n ∈ ℤ, m > n, gcd = 1, m − n odd) (REQ-CHART-023).
- `cargo test -p kernel annotation_axis` — Euclid plane: varying m at fixed ν gives identical ICs; the decoder ignores the annotation axis; quantitative 2D-density queries on it are refused (REQ-CHART-013).
- `cargo test -p kernel body_indices` — a collision of bodies 0 and 1 reports pair id 2 through the payload pair-id map; ICDescriptor fields are m0 m1 m2; Burrau (3, 4, 5) loads with 0-based masses (5, 4, 3)/12 (REQ-SYS-016).
- `cargo test -p kernel burrau_nu_half` — decode ν = 1/2 and compare with the (m, n) = (2, 1) construction and the classical configuration; tolerance: REQ-VAL-118 (calibrated) (REQ-VAL-016).
- Calibration: the proposal compares the ν = 1/2 decode with the (2, 1) construction at f64 and states the tolerance; reviewer-checked, confirmed by the human at the M2 gate, recorded in `decisions.md` (REQ-VAL-118).

## Notes
- Gap G22: the (ν, K) chart's K warp constants — the corpus gives K(t) = K_max·t^γ_K for the invariant charts (calibrated by REQ-CHART-044, TASK-M2-12) but doesn't say the Burrau K axes share those defaults.
- Both Burrau shape charts' `system_image` labels are closed with the folded chart (TASK-M2-11, REQ-CHART-025).
- Waits on RQ-100 (`REVIEW_QUEUE.md`): Existing requirements closed after the task that needs them.
