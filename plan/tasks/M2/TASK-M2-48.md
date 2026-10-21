# TASK-M2-48 — The decoder's fma-built branch inputs rewritten to R-547's form and re-proved: M₀₁ for M01_TINY, and decode test 12 on Metal and lavapipe

- **Milestone:** M2
- **Closes:** REQ-DEC-011, REQ-VAL-017
- **Depends on:** TASK-M2-02, TASK-M2-24
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~250 lines

## Goal
The mass block forms `M₀₁ = fma(m₁, 1, m₀)` (dd_decoder §3.1), whose premise fails: CI's lavapipe evaluates an
explicit `fma` as a rounded product and an add. R-547 (RQ-298 (a)) corrects the rule: a branch input that must agree
across backends is built only from operations every IEEE backend rounds alike, with no intermediate that can be
subnormal. `M₀₁` is rewritten to that rule and re-proved, and decode test 12 is re-run with it on Metal and lavapipe,
every branch input of dd_decoder §2's table in its corrected form.

## References
- `decisions.md` § "R-547 — Decode and encode numerics: the swap test is the conjugacy; step 0 is skipped within a calibrated `τ_I` and rescales in two steps; a branch input that must agree across backends is built only from operations every IEEE backend rounds alike, with no subnormal intermediate; the native build runs without MIR GVN and CI runs it at the release profile; T1 holds off the mirror band; REQ-SYS-015 is split *(closes RQ-283, RQ-295, RQ-296, RQ-298, RQ-299, RQ-360 and RQ-365; corrects R-34)*"
- `docs/design/principia_dd_decoder.md` § "3.1 Mass"
- `docs/design/principia_dd_decoder.md` § "2. Consolidated contract (every clause that binds this component)"
- `docs/notes/principia_gpu_determinism_note.md` § "The discipline (each rule = one measured failure)"
- `decisions.md` § "R-84 — Branch decisions across precisions *(closes RQ-35)*"

## Deliverables
- `crates/kernel/src/decode/mass.rs`: `M₀₁` in R-547's form, its proof in the module's note; dd_decoder §2's table
  row and §3.1 updated to it.
- Tests in `crates/kernel/tests/` and the GPU dispatch tests of TASK-M2-24.

## Acceptance tests
- `cargo test -p kernel decode_mass` — decoder test 4; the tag fires exactly on `M₀₁ < ε` and agrees bit for bit
  between CPU-f32 and the GPU on Metal and lavapipe at TASK-M2-02's onset crossings; control: the unfused
  `fma(m₁, 1, m₀)` evaluation flips the tag (REQ-DEC-011).
- `cargo test -p engine decode_parity` — decoder test 12 on Metal and lavapipe: continuous outputs within the
  f32-eps-scaled tolerance, branch tags bitwise equal (REQ-VAL-017).
- Review (physics) — each branch input's form and proof (REQ-DEC-011, REQ-VAL-017).

## Notes
- Applied per R-369: TASK-M2-02 and TASK-M2-24 merged with REQ-DEC-011 and REQ-VAL-017 held (rq: RQ-298); R-547
  rewrites and re-proves each in its own task, so they move here. REQ-DEC-017 and REQ-DEC-022, already built in the
  corrected form, stay closed by their tasks.
- The test names above are the existing tests' where they exist; the implementer keeps them.
