# TASK-M2-02 — Decoder: the latent type, the mass block and the configuration block

- **Milestone:** M2
- **Closes:** REQ-DEC-010, REQ-DEC-011, REQ-DEC-039, REQ-DEC-012, REQ-DEC-013, REQ-DEC-015, REQ-DEC-028, REQ-SYS-010, REQ-INT-002
- **Depends on:** TASK-M2-01
- **Needs (earlier milestones):** REQ-PAY-017, REQ-SYS-004, REQ-PAY-014
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~400 lines

## Goal
The first two blocks of the shared decoder exist in the one Rust source, generic over `Real`: the 8-component latent type with the fixed block layout z[0:2] config, z[2:6] momentum, z[6:8] mass (no coordinate on a gauge direction); D_mass — μ_k = μ_max·tanh(z_μk), (m₀, m₁, m₂) = softmax(0, μ₁, μ₂), body 0 the reference, DEGENERATE(M01_TINY) iff M₀₁ < ε and nothing else in the block tags; and D_cfg — the canonical-frame hyperspherical decode α = (π/2)σ(z_α), β = πσ(z_β), ρ̃ = cos α·(1,0), λ̃ = sin α·(cos β, sin β), unweighted and reconstructed so CoM = 0 and I = 1 identically, in the dimensionless system G = M = I = 1. The M01_TINY ε is proposed with its evidence.

## References
- `docs/design/principia_dd_decoder.md` § "3.1 Mass"
- `docs/design/principia_chart_reference.md` § "0.1 Masses"
- `docs/contracts/principia_chart_decoder_contract.md` § "Link functions — the mass-simplex / logit question"
- `decisions.md` § "R-10 — μ_max and q_max are settled *(closes RQ-8, amends R-5)*"
- `docs/design/principia_dd_decoder.md` § "5. Unit tests"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_dd_decoder.md` § "3.2 Configuration — hyperspherical mass-weighted Jacobi"
- `docs/design/principia_chart_reference.md` § "0.2 Configuration — hyperspherical mass-weighted Jacobi"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 2 — The decoder"
- `decisions.md` § "R-21 — `α_min = 0` *(CD-1)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Degenerate configs: open & unguarded, approach as a limit"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 1 — The physics: what the 8D actually is"
- `docs/contracts/principia_chart_decoder_contract.md` § "Design axioms (the six that must survive contact with a code agent)"
- `docs/design/principia_dd_decoder.md` § "2. Consolidated contract (every clause that binds this component)"
- `docs/design/principia_chart_reference.md` § "1. The 8D latent chart — the reference coordinate system"
- `docs/contracts/principia_canonical_spec.md` § "3. The physics & manifold model *(authoritative: `chart_decoder_contract`, `dd_decoder`)*"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"

## Deliverables
- `crates/kernel/src/decode/latent.rs` (the `Latent<R>` type, exactly 8 components, block accessors).
- `crates/kernel/src/decode/mass.rs` and `crates/kernel/src/decode/config.rs`, calling only the generated links and constants of TASK-M2-01.
- The DEGENERATE reason type with its first member `M01_TINY` (the full closed set is written in TASK-M2-24).
- Tests in `crates/kernel/tests/decode_blocks.rs`, run at both `f64` and `f32` instantiations natively.
- The M01_TINY ε calibration proposal (a `fixtures/gates/m01_tiny/` measurement and the PR text).

## Acceptance tests
- `cargo test -p kernel decode_mass` — z_μ = 0 → (⅓, ⅓, ⅓); the Burrau point μ₁ = log(b/c), μ₂ = log(a/c) → masses (c, b, a)/(a+b+c); Σmᵢ = 1 on fuzzed z (REQ-DEC-010).
- `cargo test -p kernel decode_m01_tiny` — decoder test 4: fuzz z_μ including saturation; the tag fires exactly on M₀₁ < ε (ε: REQ-DEC-039, calibrated) and nowhere else in the mass block; tagged outputs are never NaN and never dropped; the fire branch is exercised by a mass input below ε (REQ-DEC-011).
- Calibration: the ε proposal states whether ε is one of ε_μ, ε_z, ε_q, ε_w or a new constant, with the M₀₁ range reachable at μ_max = 5 as evidence; reviewer-checked, confirmed by the human at the M2 gate and recorded in `decisions.md` (REQ-DEC-039).
- `cargo test -p kernel decode_config` — z = 0 → α = π/4, β = π/2, ‖ρ̃‖ = ‖λ̃‖; ‖ρ̃‖² + ‖λ̃‖² = 1 on fuzzed z (REQ-DEC-012).
- `cargo test -p kernel decode_reconstruct` — r₁ − r₀ = ρ, total CoM = 0 and I = Σmᵢ‖rᵢ‖² = 1 on fuzzed z; decoder test 6: recompute ρ̃, λ̃ from the emitted (m, rᵢ) and recover (α, β, R̃ = 1) (REQ-DEC-013).
- `cargo test -p kernel decode_alpha_orientation` — decoder test 9: α → 0 ⇒ ‖r₁ − r₀‖ maximal and ‖λ‖ minimal; monotone crossover at π/4 (REQ-DEC-015).
- Review (physics): the decoder has no endpoint-landing or exclusion guard; degenerate configurations are only approached as limits (REQ-DEC-028).
- Review (physics): no global mass/physics parameter exists outside z and the chart; rotation and scale are fixed by the canonical-frame decode (ρ̃ on +x, R̃ = 1), not by latent coordinates; the latent type has exactly 8 components (REQ-SYS-010).
- `cargo test -p kernel units_dimensionless` — decoded ICs have total mass 1 and I = 1; G is the constant 1 (REQ-INT-002).

## Notes
- PIT-3 "check the measurement can fire": at μ_max = 5, M₀₁ = (1 + e^{μ₁})/(1 + e^{μ₁} + e^{μ₂}) is bounded below by about 6.7×10⁻³ for every finite z, so M01_TINY cannot fire from the latent chart unless ε exceeds that. The calibration must say where the tag is reachable (chart maps that set masses directly — the ternary plot, the δm strip) and the test must drive that path, not only z.
- Gap G9: decoder §5 tests 1 and 6 say "precision-appropriate tolerance" / "to tolerance" with no value; only the cross-backend factor (REQ-DEC-043) is a calibration requirement.
- REQ-DEC-014 (exact-pole ingestion) is closed with encode in TASK-M2-15, because "ingest" is the encode path.
