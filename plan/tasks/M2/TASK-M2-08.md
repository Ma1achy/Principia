# TASK-M2-08 — The shape-sphere chart: shape_vec, the chart maps, the closed-form inverse and landmarks

- **Milestone:** M2
- **Closes:** REQ-CHART-018, REQ-CHART-019, REQ-CHART-020, REQ-CHART-036, REQ-INT-003, REQ-VAL-023, REQ-CHART-048, REQ-CHART-049
- **Depends on:** TASK-M2-05, TASK-M2-07
- **Needs (earlier milestones):** REQ-SYS-009
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-3
- **Size:** ~450 lines

## Goal
`ShapeSphere { n0, e1, e2, I, phase }` exists as a Φ into the shared decoder, in R-14's one convention: shape_vec u = ‖ρ̃‖² − ‖λ̃‖², v = 2ρ̃·λ̃, w = 2(ρ̃ ∧ λ̃), n = (u, v, w)/I; the spherical map θ = 2πs, φ = π(1 − t) (L⁺ at the top, no polar buffer) and the exponential map about (n₀, e₁, e₂) as the nonlinear chart; and the closed-form inverse a = I(1+n₀)/2, … , ψ = φ_f + atan2(q, p), unweighted and reconstructed per §0.2. The landmarks are computed from the shape map with the current masses (R-14, R-50), not hard-coded.

## References
- `docs/design/principia_chart_reference.md` § "3. Shape-sphere chart `(θ, φ)`"
- `docs/design/principia_chart_reference.md` § "3.1 Forward map (already implemented as `shape_vec`)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 1 — The physics: what the 8D actually is"
- `decisions.md` § "R-14 — One shape-sphere convention, the IC Inspector's *(closes RQ-12, corrects R-12's premise)*"
- `docs/design/principia_chart_reference.md` § "3.3 The chart map"
- `docs/contracts/principia_inverse_encode_contract.md` § "Chart-aware validation"
- `decisions.md` § "R-12 — The shape-sphere chart map stays as the markdown has it *(closes RQ-10)*"
- `docs/design/principia_chart_reference.md` § "3.2 Inverse — closed form"
- `docs/design/principia_chart_reference.md` § "5.2 Tests that can fail"
- `docs/design/principia_dd_integrator.md` § "3.7 The shape readout and winding (live, per macro-step — lockstep, ratified)"
- `decisions.md` § "R-50 — The shape-sphere collision landmarks are mass-weighted *(CO-1 (a))*"
- `docs/design/principia_chart_reference.md` § "3.4 Landmarks at known fixed coordinates"
- `docs/notes/ic_inspector_scratchpad.md` § "Status — as built (supersedes stale details below)"
- `docs/design/principia_chart_reference.md` § "2. Invariant-momentum charts `(Lz, E)` and `(Lz, K)`"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-117 — The lowering appendix's shape-sphere row uses (θ, φ) *(closes RQ-85)*"
- `decisions.md` § "R-124 — Apply the R-25, R-50 and R-102 follow-ups now *(closes RQ-92)*"

## Deliverables
- `crates/kernel/src/shape.rs` (`shape_vec`, landmarks) and `crates/kernel/src/chart/shape_sphere.rs` (spherical and exponential maps, the closed-form inverse).
- A cross-check fixture of `n` on random ICs generated from the IC Inspector's JS (`docs/gui/reference/ic_inspector.html`, run under node) into `fixtures/ic_inspector/shape_vec.json`, with the script that produced it.
- Tests `crates/kernel/tests/shape_sphere.rs`; gate `cargo xtask gate shape-roundtrip`.

## Acceptance tests
- `cargo test -p kernel shape_vec` — L⁺ (bodies 0→1→2 anticlockwise, equilateral) → w = +1; collision of 0 and 1 → n = (−1, 0, 0); n cross-checked against the IC Inspector's JS on random ICs (REQ-CHART-018).
- `cargo test -p kernel shape_sphere_chart_map` — (s, t) = (·, 1) → L⁺ (0, 0, +1); (s, t) = (·, 0) → L⁻; φ spans [0, π] with no excised cap; the exp-map at d = 0 returns n₀ (REQ-CHART-019).
- `cargo xtask gate shape-roundtrip` — shape_vec(decode(u, v)) == n(u, v) to ~1e−14 over random masses and shapes (REQ-CHART-020).
- `cargo test -p kernel shape_landmarks` — BC₀₁ → (−1, 0, 0), L⁺ → (0, 0, +1), all binary collisions at w = 0, equal masses 120° apart; n cross-checked against the IC Inspector's JS on random ICs (REQ-CHART-036).
- `cargo test -p kernel shape_landmarks_mass_weighted` — dd_integrator test 9: BC₀₁ → (−1, 0, 0) for any masses; L⁺ → (0, 0, +1); random masses → all collisions at w = 0; equal masses → azimuths 180°, 60°, 300° (REQ-INT-003).
- `cargo test -p kernel shape_dot_on_collision_landmark` — property: random masses, bodies i and j brought together: the dot's distance to the BC landmark opposite k → 0 and to the others stays > 0 (REQ-VAL-023).
- Definition: the source of the shape sphere's and the invariant charts' held values (m_fixed, p_fixed and its frame, φ_f; fixed geometry and masses) written into chart_reference §2–§3 and approved by the physics reviewer (REQ-CHART-048).
- Proposal: the tolerance of `n` against the IC Inspector's JS, with the measured max difference on the fixture as evidence; the human confirms it at the M2 gate (REQ-CHART-049).

## Notes
- Gap G8: Φ_S²(u,v) = (n(θ,φ), m_fixed, p_fixed) — the corpus doesn't say where the held masses and momenta come from (z₀'s mass and momentum blocks, or chart params), whether p_fixed is taken before or after the fibre-phase rotation that C then undoes, nor φ_f's default.
- The Euler landmarks for unequal masses (R-126: the Euler central configurations) are built with the physics site generators in TASK-M7-09 (REQ-COL-021); this task builds the collision landmarks and L±.
- Gap G9: the IC Inspector cross-check tolerance isn't stated. The n-to-1 hemisphere label (R-141) and the projections are TASK-M2-28.
- RQ-85 ruled: R-117 — the lowering appendix's shape-sphere row is conformed to R-14's (θ, φ) map, the one built here.
- RQ-92 ruled: R-124 — R-50 is applied: dd_integrator §3.7 no longer calls B18 open; the landmarks are mass-weighted, as this task builds them.
- Closes, for gaps the corpus leaves open: REQ-CHART-048 (R-72 definition), REQ-CHART-049 (R-71 calibration) (classification accepted by R-132).
