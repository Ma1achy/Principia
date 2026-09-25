# TASK-M2-15 — Encode: block inverses, E = inverses ∘ C, the discard notices and T1

- **Milestone:** M2
- **Closes:** REQ-ENC-007, REQ-ENC-003, REQ-ENC-014, REQ-SYS-015, REQ-DEC-014
- **Depends on:** TASK-M2-01, TASK-M2-07
- **Needs (earlier milestones):** REQ-SYS-004
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~460 lines

## Goal
Encode exists in the same shared source as decode, inverted: the closed-form block inverses of inverse-encode Part 3 (μₖ = log(mₖ/m₀), z = artanh(μₖ/μ_max) clamped; α = atan2(‖λ̃‖, ‖ρ̃‖), β = atan2(λ̃_y, λ̃_x), z = logit(s) clamped to [ε_z, 1−ε_z]; p_λ = p₂, p_ρ = p₁ + (m₁/M₀₁)p₂, z = logit(clamp(s, ε_q, 1−ε_q)), |q| > q_max clamped and flagged) through the registry inverses, composed as E = (block inverses) ∘ C. Every discard is reported: `lookup_rescaled`, `lookup_mirrored`, `lookup_clamped` fire iff their operation did. E is constant on gauge orbits (T1). A configuration ingested exactly at an α-pole gives a deterministic value, never NaN.

## References
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 3 — Block inverses (closed forms, with their forward mates)"
- `docs/design/principia_dd_encode.md` § "3.1 Forward ↔ inverse pairing table — closures verified"
- `decisions.md` § "R-10 — μ_max and q_max are settled *(closes RQ-8, amends R-5)*"
- `decisions.md` § "R-21 — `α_min = 0` *(CD-1)*"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 1 — What encode is: a quotient map onto a section"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 8 — Round-trip contracts and the new debug view"
- `docs/design/principia_dd_encode.md` § "3.3 The gauge group action, explicit (this is what T1 sweeps)"
- `docs/design/principia_dd_encode.md` § "5. Unit tests"
- `decisions.md` § "R-82 — One mirror test, one seed rule *(closes RQ-33)*"
- `docs/design/principia_dd_encode.md` § "2. Consolidated contract"
- `docs/design/principia_dd_encode.md` § "4. Seams (obligations → integration tests)"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 6 — The canonical inverse policy, completed"
- `docs/design/principia_dd_decoder.md` § "Drill-down — the Decoder (Matter rung)"
- `docs/design/principia_dd_decoder.md` § "1. What it is"
- `docs/design/principia_dd_encode.md` § "Drill-down — Encode (the Question rung's upward door)"
- `docs/design/principia_dd_decoder.md` § "3.2 Configuration — hyperspherical mass-weighted Jacobi"
- `docs/design/principia_chart_reference.md` § "0.2 Configuration — hyperspherical mass-weighted Jacobi"

## Deliverables
- `crates/kernel/src/encode/{mod.rs, blocks.rs, report.rs}` (the one encode entry, the discard report).
- The gauge-group sampler of dd_encode §3.3 (translation, boost, rotation, scale, mirror) as a test utility.
- Tests `crates/kernel/tests/encode_core.rs`; cross-backend pole checks in `crates/engine/tests/encode_gpu.rs`.

## Acceptance tests
- `cargo test -p kernel encode_block_inverses` — forward substituted into inverse for fuzzed z: each block recovers z to tolerance; a |q| > q_max input sets `lookup_clamped` (REQ-ENC-007).
- `cargo test -p kernel encode_t1_gauge` — encode test 2: random g per dd_encode §3.3, including deadband-straddling rotations (λ̃_y within ±δ_λ); outputs equal after float tolerance; the mirror tie |λ̃_y| ≤ δ_λ gives no mirror, identically twice (REQ-ENC-003).
- `cargo test -p kernel encode_notices` — canonical inputs raise no notice; each operation forced individually raises exactly its notice (REQ-ENC-014).
- Review (code + physics): one decode module is referenced by the kernel stage, the CPU path and encode; a scan finds no second implementation of any dd_decoder §3 formula outside the shared source (REQ-SYS-015).
- `cargo test -p engine encode_exact_poles` — configurations with ‖λ̃‖ = 0 and ‖ρ̃‖ = 0 exactly: no NaN; the output is deterministic, identical on repeat and on CPU-f64 and GPU-f32; the flag is set (REQ-DEC-014).

## Notes
- Gap G14: REQ-DEC-014's "saturation flag" (dd_decoder §3.2 "the saturation flags", R-21 "the SAT flags") isn't defined for encode: payload §2's `saturated` bit means the substep cap. At ‖ρ̃‖ = 0 the flag is `lookup_clamped` (R-13); at ‖λ̃‖ = 0 (s_α at the clamp) the corpus doesn't name it.
- REQ-SYS-015's WGSL instantiation is the question in Gap G1 (TASK-M2-25).
