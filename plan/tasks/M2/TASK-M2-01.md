# TASK-M2-01 — Link registry: generated links with inverses, log-det, ε clamps and the chart constants

- **Milestone:** M2
- **Closes:** REQ-CHART-032, REQ-CHART-034, REQ-GEN-013, REQ-GEN-014, REQ-GEN-015, REQ-DEC-009, REQ-GEN-025, REQ-GEN-026
- **Depends on:** TASK-M1-08
- **Needs (earlier milestones):** REQ-SYS-004, REQ-GEN-010, REQ-SYS-005, REQ-SYS-001
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-3
- **Size:** ~450 lines

## Goal
The link registry of generation-root §3.9 exists as ledger data and is the root of every decode and encode transcendental (the dependency DAG puts it beside the layout table). Each entry — simplex (softmax ∘ μ_max·tanh), bounded σ, bounded tanh, positive (softplus / exp), symmetric c·tanh, unbounded identity — ships forward, inverse, log-det Jacobian, ε clamps and its over/under-sampling note, and the generator emits the link functions as Rust generic over `Real` for the one shared kernel source. Links are selected per block/control, restricted to the block's codomain, with the defaults mass = softmax ∘ μ_max·tanh, config = sigmoid, free momentum = sigmoid, and the selection is baked per block. The chart constants μ_max = 5, q_max = 2, α_min = 0, ε_μ = ε_z = ε_q = 10⁻⁶, δ_λ = 10⁻¹² and ε_w = 10⁻¹⁰ are registry data, and the formulae that follow reference them by name.

## References
- `docs/design/principia_dd_generation_root.md` § "3.9 The link registry (consolidated from chart contract Part 2.5)"
- `docs/design/principia_dd_generation_root.md` § "2. Consolidated contract"
- `docs/contracts/principia_chart_decoder_contract.md` § "Links carry a measure"
- `docs/contracts/principia_chart_decoder_contract.md` § "Integrity: the link is part of the experiment"
- `docs/design/principia_dd_generation_root.md` § "5. Tests (properties any generator must satisfy)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Three hard requirements on any registered link"
- `docs/contracts/principia_canonical_spec.md` § "3. The physics & manifold model *(authoritative: `chart_decoder_contract`, `dd_decoder`)*"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `docs/contracts/principia_chart_decoder_contract.md` § "Link functions — the mass-simplex / logit question"
- `docs/design/principia_dd_decoder.md` § "2. Consolidated contract (every clause that binds this component)"
- `docs/design/principia_dd_decoder.md` § "3. The maths"
- `docs/design/principia_chart_reference.md` § "0.1 Masses"
- `docs/design/principia_dd_encode.md` § "2. Consolidated contract"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 3 — Block inverses (closed forms, with their forward mates)"
- `docs/design/principia_chart_reference.md` § "2.2 Deterministic momentum construction"
- `decisions.md` § "R-10 — μ_max and q_max are settled *(closes RQ-8, amends R-5)*"
- `decisions.md` § "R-21 — `α_min = 0` *(CD-1)*"
- `decisions.md` § "R-82 — One mirror test, one seed rule *(closes RQ-33)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 2.5 — Link functions & compactification (customisable)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- The link registry entries and chart constants in the generation-root ledger source (`crates/ledger/`), each entry with forward, inverse, log-det, ε clamps and sampling note; a missing member fails generation.
- Generator output `crates/kernel/src/generated/links.rs` (Rust, generic over `Real`, `no_std`-clean for the SPIR-V build) and `crates/kernel/src/generated/constants.rs`.
- The per-block link selection type (codomain-checked at registration; defaults resolve by name), baked as a type parameter so a link change is a different monomorphised variant.
- Tests: `crates/ledger/tests/link_registry.rs`, `crates/kernel/tests/links.rs` (generation-root §5 test 8 (a)–(d) per link), and a source scan asserting decode/encode formulae name the constants.

## Acceptance tests
- `cargo test -p ledger link_registry_complete` — the registry enumerates the six constraint classes of §3.9, each entry with forward, inverse, log-det, ε clamps and sampling note; deleting any member fails generation (REQ-CHART-032).
- `cargo test -p kernel link_properties` — per link, fuzzed across the domain including saturation: (a) constraint preservation (simplex outputs positive and summing to 1, bounded outputs in range); (b) inverse round-trip within the ε-clamp tolerance, asserted in physical units; (c) analytic log-det against a numeric Jacobian; (d) C¹ by central differences, no kinks. Each property is shown able to fail on a deliberately broken link (REQ-CHART-034).
- `cargo test -p ledger link_codomain_compat` — registering a link whose codomain does not match the block's constraint type is rejected; each block default resolves to the named entry (REQ-GEN-013).
- Review (physics): for each block the registry holds at least two links whose recorded over/under-sampling regions differ (REQ-GEN-014).
- Review (code): generator outputs inspected per target — link functions and kernel pack/unpack are generated Rust; fragment accessors and the catalogue are generated WGSL through the fragment assembler; link selection is baked per block (REQ-GEN-015).
- `cargo test -p kernel chart_constants` — the registry values equal μ_max = 5, q_max = 2, α_min = 0, ε_μ = ε_z = ε_q = 10⁻⁶, δ_λ = 10⁻¹², ε_w = 10⁻¹⁰; a source scan of `crates/kernel/src/{decode,encode}` finds no literal of these values outside the generated constants (REQ-DEC-009).
- Proposal: the step and tolerances of registry test 8 (c) and (d), with the measured discrepancy per link as evidence; the human confirms them at the M2 gate (REQ-GEN-025).
- Definition: the alternative links (edge-reaching simplex, heavier-tailed bounded) written into dd_generation_root §3.9 and approved by the physics reviewer (REQ-GEN-026).

## Notes
- Gap G10: the second link per block (REQ-GEN-014) — the corpus names "a simplex map that reaches the edges", "a heavier-tailed bounded map" and "temperature-softmax" without a formula or parameter, and §3.9's table has one simplex link. The task registers only links the corpus defines until this is ruled.
- Gap G9: generation-root §5 test 8 (c) and (d) give no tolerance for the numeric-Jacobian and C¹ checks.
- The ε clamps are registry data (dd_encode §2); encode (TASK-M2-15) consumes the registry inverses, never an inline `logit`/`artanh`.
- Closes, for gaps the corpus leaves open: REQ-GEN-025 (R-71 calibration), REQ-GEN-026 (R-72 definition) (REVIEW_QUEUE RQ-110 lists them for the human).
