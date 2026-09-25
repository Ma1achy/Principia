# TASK-M2-03 — Decoder: free momentum, the factorised join and the ICDescriptor

- **Milestone:** M2
- **Closes:** REQ-DEC-001, REQ-DEC-003, REQ-DEC-004, REQ-DEC-019, REQ-DEC-030, REQ-DEC-026, REQ-DEC-027, REQ-PAY-034, REQ-PAY-035, REQ-VAL-018, REQ-VAL-020
- **Depends on:** TASK-M2-02
- **Needs (earlier milestones):** REQ-PAY-001, REQ-PAY-002, REQ-PAY-020, REQ-PAY-017
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-3
- **Size:** ~480 lines

## Goal
The decoder is complete for the free path: D_mom decodes qₖ = q_max·(2σ(z_qk) − 1), p_ρ = (q₀, q₁), p_λ = (q₂, q₃) and the crossed Jacobi-to-particle map p₀ = −p_ρ − (m₀/M₀₁)p_λ, p₁ = p_ρ − (m₁/M₀₁)p_λ, p₂ = p_λ with no mass unweighting (R-28); the decode entry runs mass → config → momentum → derived invariants and each block reads only its z slice and earlier blocks; and the decode stage computes the ICDescriptor's derived quantities (K₀, V₀, virial_ratio and the rest of the ledger's twelve fields, E₀ derived at read, never stored) post-decode, pre-integration. z = 0 decodes to the golden IC in f64 and f32.

## References
- `docs/contracts/principia_canonical_spec.md` § "3. The physics & manifold model *(authoritative: `chart_decoder_contract`, `dd_decoder`)*"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 2 — The decoder"
- `docs/design/principia_dd_decoder.md` § "2. Consolidated contract (every clause that binds this component)"
- `docs/design/principia_chart_reference.md` § "1. The 8D latent chart — the reference coordinate system"
- `docs/design/principia_dd_decoder.md` § "3.4 Momentum"
- `docs/design/principia_chart_reference.md` § "0.3 Momentum — free Jacobi momenta"
- `decisions.md` § "R-28 — Momentum decode has no mass unweighting *(CD-8)*"
- `decisions.md` § "R-10 — μ_max and q_max are settled *(closes RQ-8, amends R-5)*"
- `docs/design/principia_dd_decoder.md` § "5. Unit tests"
- `docs/design/principia_coordinate_conventions_note.md` § "Per-axis signedness: "if it makes sense for that axis""
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 3 — Charts"
- `docs/design/principia_dd_decoder.md` § "3.6 ICDescriptor derived quantities (decode-time, pre-integration)"
- `docs/design/principia_dd_decoder.md` § "6. Deferred / flagged"
- `decisions.md` § "R-22 — Body indices are 0-based; pair ids name the opposite side *(CD-2, amended)*"
- `decisions.md` § "R-62 — `rho_mag` and `lambda_mag` *(amends R-22)*"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `docs/design/principia_dd_generation_root.md` § "3.6 `ICDescriptor` (12 × f32)"
- `docs/design/principia_dd_validation_orbits.md` § "3. Proposed suite"
- `docs/design/principia_dd_validation_orbits.md` § "1.3 Lagrange and Euler central configurations — the only analytic ones"
- `docs/design/principia_dd_validation_orbits.md` § "5. Immediate action"

## Deliverables
- `crates/kernel/src/decode/momentum.rs` and `crates/kernel/src/decode/mod.rs` (the ordered decode entry, returning the canonical (m, r, p) seam type and the DEGENERATE tag).
- `crates/kernel/src/decode/descriptor.rs` filling the generated `ICDescriptor` (ledger §3.6) from the decoded IC.
- Tests in `crates/kernel/tests/decode_join.rs` and `crates/kernel/tests/ic_descriptor.rs` at `f64` and `f32`; `crates/ledger/tests/ic_descriptor_layout.rs`.
- The golden-IC fixture `fixtures/golden_ic/z0.json` with every derived value hand-computed.

## Acceptance tests
- `cargo test -p kernel decode_factorised` — each block depends only on its z slice (and earlier blocks); the call order mass → config → momentum → invariants is asserted (REQ-DEC-001).
- `cargo test -p kernel decode_block_layout` — z with one block non-zero at a time moves only that block's physical outputs (config: α, β; momentum: p_ρ, p_λ; mass: m) (REQ-DEC-003).
- Review (physics): the decode source calls D_mass before D_cfg before D_mom before invariant evaluation, and the invariant-momentum construction reads the already-decoded masses and configuration (REQ-DEC-004).
- `cargo test -p kernel decode_momentum` — Σpᵢ = 0 to machine precision on fuzzed z with unequal masses (catches the crossed m₀/m₁ factors); z_q = 0 gives rest; |qₖ| ≤ q_max (REQ-DEC-019).
- `cargo test -p kernel decode_no_mass_unweighting` — an unequal-mass fixture matches the dd_decoder §3.4 formula with no mass-unweighting step (REQ-DEC-030).
- `cargo test -p kernel decode_block_independence` — decoder test 2: perturbing z_μ leaves (α, β) and (p_ρ, p_λ) bit-identical, and likewise for each block (REQ-DEC-026).
- `cargo test -p kernel decode_ranges` — decoder test 3 over fuzzed z including ±large values: α ∈ (0, π/2), β ∈ [0, π], |qₖ| ≤ q_max; masses stay in the open simplex (REQ-DEC-027).
- `cargo test -p kernel ic_descriptor` — golden IC (z = 0): every descriptor field in f64 and f32 against hand-computed values; field names match the ledger; the derived E₀ equals K_0 + V_0 and the struct has no E₀ field (REQ-PAY-034).
- `cargo test -p ledger ic_descriptor_layout` — generated `ICDescriptor` field names, types and scales; the 16 B padding is a declared member; size is 64 B (REQ-PAY-035).
- `cargo test -p kernel golden_ic` — z = 0 → masses (⅓, ⅓, ⅓), α = π/4, β = π/2, q = 0, I = 1, every derived value asserted in f64 and f32 (REQ-VAL-018; encode's golden pair is asserted in TASK-M2-16).
- `cargo test -p kernel chart_landmarks_lagrange_euler` — z = 0 decodes to the Lagrange equilateral configuration; z_β → ±large decodes toward the Euler collinear configurations at β → 0 and β → π, approached as a limit (REQ-VAL-020).

## Notes
- Gap G7: the ICDescriptor formulas for `q_mass`, `rho_mag`, `lambda_mag`, `rho_ratio`, `rho_angle` and `r_min_pair_0` are not in the corpus (dd_decoder §3.6 names "ρ-magnitudes, ρ_ratio, ρ_angle, r_min_pair₀"; ledger §3.6 lists the fields; neither gives a formula, nor whether `rho_mag` is ‖ρ‖ or ‖ρ̃‖). REQ-PAY-034's hand-computed values can't be written for those fields until this is ruled.
- PIT-9: the Σpᵢ = 0 check cannot catch the crossed-factor swap at equal masses — the fuzz must draw unequal masses, and a swapped-factor control must fail.
