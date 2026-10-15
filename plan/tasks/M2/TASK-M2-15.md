# TASK-M2-15 — Encode: block inverses, E = inverses ∘ C, the discard notices and T1

- **Milestone:** M2
- **Closes:** REQ-ENC-007, REQ-ENC-003, REQ-ENC-014, REQ-SYS-015, REQ-DEC-014
- **Depends on:** TASK-M2-01, TASK-M2-07
- **Needs (earlier milestones):** REQ-SYS-004
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~460 lines

## Goal
Encode exists in the same shared source as decode, inverted: the closed-form block inverses of inverse-encode Part 3 (μₖ = log(mₖ/m₀), z = artanh(μₖ/μ_max) clamped; α = atan2(‖λ̃‖, ‖ρ̃‖), β = atan2(|λ̃_y|, λ̃_x) (R-438), z = logit(s) clamped to [ε_z, 1−ε_z]; p_λ = p₂, p_ρ = p₁ + (m₁/M₀₁)p₂, z = logit(clamp(s, ε_q, 1−ε_q)), |q| > q_max clamped and flagged) through the registry inverses, composed as E = (block inverses) ∘ C. Every discard is reported: `lookup_rescaled`, `lookup_mirrored`, `lookup_clamped` fire iff their operation did. E is constant on gauge orbits (T1). A configuration ingested exactly at an α-pole gives a deterministic value, never NaN, and sets `lookup_clamped` (R-133). The fragment WGSL decode and encode are never hand-written: they are generated from this source by the rust-gpu → SPIR-V → WGSL translation (R-116; the fragment snippets are built in TASK-M2-25).

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
- `decisions.md` § "R-116 — The fragment decode and encode are generated from the one source *(closes RQ-84)*"
- `decisions.md` § "R-133 — The seven checkpoint-B interpretations are accepted *(closes RQ-111)*"
- `decisions.md` § "R-438 — The encode reads `β = atan2(|λ̃_y|, λ̃_x)`, the mirror's quotient for `β` only, so `β ∈ [0, π]` holds because `β` reads `|λ̃_y|`; at `λ̃ = 0` it takes R-437's pole value by a select; R-82 is unchanged *(closes RQ-287)*"
- `docs/design/principia_dd_encode.md` § "3.2 Canonicalisation `C` — the exact ordered procedure"
- `decisions.md` § "R-437 — The value of every `atan2` at a possible pole of the decode, encode and canonicalisation is defined: 0, for every sign of the zeros, chosen branch-free by a select on the operands against 0, so CPU and GPU agree *(closes RQ-285)*"

## Deliverables
- `crates/kernel/src/encode/{mod.rs, blocks.rs, report.rs}` (the one encode entry, the discard report).
- The gauge-group sampler of dd_encode §3.3 (translation, boost, rotation, scale, mirror) as a test utility.
- Tests `crates/kernel/tests/encode_core.rs`; cross-backend pole checks in `crates/engine/tests/encode_gpu.rs`.

## Acceptance tests
- `cargo test -p kernel encode_block_inverses` — forward substituted into inverse for fuzzed z: each block recovers z to tolerance; a |q| > q_max input sets `lookup_clamped` (REQ-ENC-007).
- `cargo test -p kernel encode_collinear_beta` — dd_encode test 10: collinear shapes encoded with `λ̃_y ∈ {−1e-13, −0, +0, +1e-13}`, body 2 on either side and at each Euler landmark, give `β = π` (body 2 on the `−x` side) or 0 (on the `+x` side) as appropriate, never near `−π`, and a round trip within the deadband; `β` reads `|λ̃_y|` by `abs`, never `max` (REQ-ENC-007).
- `cargo test -p kernel encode_t1_gauge` — encode test 2: random g per dd_encode §3.3, including deadband-straddling rotations (λ̃_y within ±δ_λ); outputs equal after float tolerance; the mirror tie |λ̃_y| ≤ δ_λ gives no mirror, identically twice (REQ-ENC-003).
- `cargo test -p kernel encode_notices` — canonical inputs raise no notice; each operation forced individually raises exactly its notice (REQ-ENC-014).
- Review (code + physics): one decode module is referenced by the kernel stage, the CPU path and encode; a scan (including `.wgsl` files) finds no second implementation of any dd_decoder §3 formula outside the shared source, so any fragment decode/encode can only be the translation's output (REQ-SYS-015).
- `cargo test -p engine encode_exact_poles` — configurations with ‖λ̃‖ = 0 and ‖ρ̃‖ = 0 exactly: no NaN; the output is deterministic, identical on repeat and on CPU-f64 and GPU-f32; `lookup_clamped` is set — the encode clamp flag, not the payload's `saturated` bit (R-133) (REQ-DEC-014).

## Notes
- R-133: REQ-DEC-014's "saturation flag" (R-21's "SAT flags") is read as the encode clamp flag `lookup_clamped` (inverse_encode Part 3–4, dd_encode §3.5), which fires at either exact α-pole.
- RQ-84 ruled: R-116 — the fragment WGSL decode and encode are generated from the one Rust source (rust-gpu → SPIR-V → WGSL), never hand-written; REQ-SYS-015 stands and the agreement presets check the translation (TASK-M2-25).
- TASK-M2-01 (PR #187, qa finding (c)): the registry's simplex inverses are defined on the open simplex. At an exact
  corner, with two masses 0, `softmax_tanh`'s `log(mₖ/m₀)` and `stick_breaking`'s `u = m₂/(m₁ + m₂)` divide 0 by 0.
  On the host, Rust's `max` ignores the NaN and the clamp returns a finite value, but SPIR-V's `FMax` with a NaN
  operand is undefined. Encode passes the mass inverse no exact corner, or guards it before the inverse runs.
- **R-437 (RQ-285):** `β = atan2(|λ̃_y|, λ̃_x)` (R-438) takes the defined 0 at `λ̃ = 0` (the `α = 0` pole), whatever the
  signs of the zeros, by a select on `λ̃_x = 0` and `λ̃_y = 0`, never `atan2(0, 0)`; `α = atan2(‖λ̃‖, ‖ρ̃‖)` has no pole
  once `I = 1` (dd_encode §3.1; inverse-encode Part 3). REQ-DEC-014's ingest test covers `±0` components at both poles
  on both precisions. The hold RQ-285 is gone.
- **R-438 (RQ-287):** `β = atan2(|λ̃_y|, λ̃_x)` reads the mirror's quotient for `β` only; R-82's mirror test and
  deadband are unchanged, so `C` leaves `λ̃_y ≥ −δ_λ`, and `β ∈ [0, π]` holds because `β` reads `|λ̃_y|`. Read it by
  `abs`, never `max` (`max(−0, +0)` can return `−0`); at `λ̃ = 0`, R-437's select gives 0. `β ≈ 0` and `β ≈ π` are
  different collinear shapes, so the test checks which. The hold RQ-287 is gone.
