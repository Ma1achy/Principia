# TASK-M2-43 — Canonicalisation's step 0 per R-547: skipped within τ_I with lookup_rescaled raised exactly when it ran, the two-step rescale, I_in = 0 kept, and C's branch inputs in the exact form

- **Milestone:** M2
- **Closes:** REQ-ENC-006, REQ-ENC-014, REQ-DEC-002, REQ-ENC-034
- **Depends on:** TASK-M2-07, TASK-M2-15
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~350 lines

## Goal
`kernel::canon::canonicalise` (TASK-M2-07) runs step 0 on every input, forms no raised flag for `lookup_rescaled`, and
leaves `I_in` rounded to 0 or `+∞` undefined; its CoM and `I_in` are built with explicit `fma`. R-547 rules all
three (RQ-295 (c), RQ-296 (d) + (a), RQ-298 (a)). Step 0 is skipped (`λ = 1`, `r` and `p` unchanged) where
`|I_in − 1| ≤ τ_I`, and `lookup_rescaled` fires exactly when step 0 ran; where it runs it rescales in two steps, first
by a power of two (exact), then by the remainder, so `I_in` neither underflows nor overflows for any finite state with
the bodies apart; `I_in = 0` (the bodies coincide) keeps the applied select: divisor 1, `λ = 1`, `p = 0`,
`lookup_clamped`, `φ = +0`. Every branch input of `C` that must agree across backends is built only from operations
every IEEE backend rounds alike, with no intermediate that can be subnormal, `IeeeBits::product` the pattern. `τ_I` is
proposed for the M2 gate.

## References
- `decisions.md` § "R-547 — Decode and encode numerics: the swap test is the conjugacy; step 0 is skipped within a calibrated `τ_I` and rescales in two steps; a branch input that must agree across backends is built only from operations every IEEE backend rounds alike, with no subnormal intermediate; the native build runs without MIR GVN and CI runs it at the release profile; T1 holds off the mirror band; REQ-SYS-015 is split *(closes RQ-283, RQ-295, RQ-296, RQ-298, RQ-299, RQ-360 and RQ-365; corrects R-34)*"
- `docs/design/principia_dd_encode.md` § "3.2 Canonicalisation `C` — the exact ordered procedure"
- `docs/design/principia_dd_encode.md` § "4. Seams (obligations → integration tests)"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 2 — The scale gauge gap (missing from the original inverse policy)"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 6 — The canonical inverse policy, completed"
- `docs/notes/principia_gpu_determinism_note.md` § "The discipline (each rule = one measured failure)"
- `decisions.md` § "R-13 — Lookup has no coincident-bodies rejection *(closes RQ-11)*"
- `decisions.md` § "R-23 — Encode order: CoM subtraction precedes `I` *(CD-3)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/kernel/src/canon.rs`: step 0's skip under `τ_I`, the two-step rescale, the `lookup_rescaled` flag raised
  exactly when step 0 ran, and the CoM, `I_in` and skip comparison rebuilt to R-547's rule, each form's proof in the
  module's note.
- `τ_I` at f64 and f32 as a constants-register entry, with its proposal record.
- Tests in `crates/kernel/tests/canon.rs` and `crates/engine/tests/canon_gpu.rs`.

## Acceptance tests
- `cargo test -p kernel scale_step` — encode test 4's whole row; canonical-frame decodes of fuzzed z leave step 0
  skipped with `λ = 1` and `r`, `p` bit-identical, no `lookup_rescaled`; `|I_in − 1|` just above `τ_I` runs it and
  raises it; the underflow and overflow states with the bodies apart leave `C` at `I = 1` to rounding with every output
  finite; the coincident state keeps the select; controls: a one-step rescale, and a step 0 that always runs, each fail
  (REQ-ENC-006).
- `cargo test -p kernel encode_notices` — canonical inputs raise no notice; each operation forced individually raises
  exactly its notice (REQ-ENC-014).
- `cargo test -p kernel canonicalise_section` and `cargo test -p engine canonicaliser_seam_sweep` — the four section
  conditions and idempotence; the guard, the mirror decision, the `I_in = 0` select and the `τ_I` skip agree bit for
  bit between CPU-f32 and the GPU on Metal and lavapipe (REQ-DEC-002).
- Calibration: `τ_I` proposed from the measured `|I − 1|` (TASK-M2-07 measured up to 3 eps), reviewer-checked,
  confirmed by the human at the M2 gate, recorded in `decisions.md` (REQ-ENC-034).
- Review (physics) — each branch input's form, and its proof that it rounds alike fused or not, flushing subnormals or
  not (REQ-DEC-002).

## Notes
- Applied per R-369: TASK-M2-07 and TASK-M2-15 merged with REQ-ENC-006, REQ-DEC-002 and REQ-ENC-014 held, not closed
  (rq: RQ-295, RQ-296, RQ-298); R-547 rules them, and this task closes them.
- Applied per R-369: `τ_I` is proposed here, TASK-M2-07 having merged; R-547's words name TASK-M2-07's measurement as
  its evidence.
- `crates/kernel/tests/canon.rs` is the implementer's file (TASK-M2-07); qa's files are qa's (R-290).
