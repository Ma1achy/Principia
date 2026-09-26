# TASK-M0-15 — The codegen self-test on the GPU

- **Milestone:** M0
- **Closes:** REQ-GEN-004, REQ-GEN-006, REQ-GEN-007, REQ-TOOL-003, REQ-PAY-011
- **Depends on:** TASK-M0-04, TASK-M0-13, TASK-M0-14
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~400 lines

## Goal
The codegen self-test of debug_tooling_plan §H and generation-root §5 runs in CI on every commit (parity_contract §6, the codegen row), on the two CI adapters R-110 names, as amended by R-186: GitHub-hosted `macos-15` (Metal) and lavapipe on `ubuntu-latest`. pack∘unpack is the identity per field, property-fuzzed over the full value range with the top bits set, in three places: the host Rust, the kernel's own pack/unpack on the GPU (the rust-gpu build) and a GPU self-test dispatch of the generated WGSL fragment unpack using the u32 `extractBits` overload — on a device requested without `shader-f16`. f16 pairs round-trip; `t_end_step` and `t_dmin_step` round-trip exactly as u16, bit-identical CPU/GPU, with display-fraction endpoints exactly 0 and 1; `detail` decodes per `state`; and the static layout checks pass.

## References
- `docs/design/principia_debug_tooling_plan.md` § "H. Codegen self-test (the tooling that tests the tooling)"
- `docs/design/principia_debug_tooling_plan.md` § "B. Payload field views — `sample_descriptor` (bit-packed u32)"
- `docs/design/principia_debug_tooling_plan.md` § "C. Payload field views — `times` (u32), f16-packed scalars & `free_group_word` (separate buffer)"
- `docs/design/principia_dd_generation_root.md` § "5. Tests (properties any generator must satisfy)"
- `docs/design/principia_dd_simstate_payload.md` § "6. Accessors (illustrative of generated output; source is the Rust layout definition — emitted as Rust for the kernel/host and WGSL for the fragment side)"
- `docs/design/principia_dd_simstate_payload.md` § "Why closure is here, at this width, unconditionally"
- `docs/contracts/principia_parity_contract.md` § "6. The harness"
- `docs/read_first/principia_01_pitfalls.md` § "9. A PARITY CHECK THAT MASKS THE BITS THE FORK LANDS IN"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"
- `decisions.md` § "R-186 — GitHub-hosted runners first; no self-hosted runner *(amends R-110, R-169, R-174)*"

## Deliverables
- `crates/validation/tests/codegen_selftest.rs` — the §H suite: the three pack∘unpack paths per field, times, f16 pairs, detail-per-state, static checks; proptest strategies that include every field's top bit.
- A WGSL self-test entry point wrapping the generated accessors (reads packed words, writes unpacked fields) and the kernel entry from TASK-M0-14.
- The device for these tests is requested with no optional features (no `SHADER_F16`).
- `.github/workflows/ci.yml` — the `gpu-metal` job (`macos-15`) and the `gpu-lavapipe` job (`ubuntu-latest`), both on every commit, running this suite (R-110, R-186).

## Acceptance tests
- `cargo test -p validation codegen_selftest_pack_unpack` (property) — per field, all fields including top bits set: identity in host Rust, in the kernel on the GPU and in the WGSL unpack on the GPU (REQ-GEN-004).
- `cargo test -p validation codegen_selftest_times` (property) — fuzz 0..65535: exact round trip on CPU and GPU, bit-identical; the display fraction is exactly 0 and 1 at the endpoints (REQ-GEN-006).
- `cargo test -p validation codegen_selftest` — the §H suite: fuzz per field on host and GPU; static layout checks pass; f16 pairs round-trip; `detail` decodes per `state` (REQ-GEN-007).
- `cargo test -p validation extractbits_top_bit` — fields with the top bit set round-trip without sign extension on the GPU; the same dispatch built with the i32 overload fails (REQ-TOOL-003).
- `cargo test -p validation unpack_without_shader_f16` — the fragment unpack self-test runs on a device without `shader-f16`, and its results match the Rust unpack (REQ-PAY-011).

## Notes
- Negative controls (TASK-M0-04): a WGSL accessor with a shifted offset; the i32 overload; a mask over bits a round trip compares (pitfalls §9).
- R-110 (RQ-79), amended by R-186: GPU CI is GitHub-hosted `macos-15` (Metal) plus lavapipe on `ubuntu-latest`, on every commit; the acceptance commands above run on both.
