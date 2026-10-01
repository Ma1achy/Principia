# TASK-M0-14 — The substrate toolchain: one kernel source compiled twice, and the Real-generic payload

- **Milestone:** M0
- **Closes:** REQ-PAY-017, REQ-PAY-087, REQ-SYS-075, REQ-SYS-076
- **Depends on:** TASK-M0-04, TASK-M0-10
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-10
- **Size:** ~450 lines

## Goal
`crates/kernel` builds two ways from one source (canonical_spec §1 item 2): to f32 SPIR-V through rust-gpu, then to WGSL through naga (`cargo xtask build-kernel`), and natively through rustc. A trivial kernel — the generated pack/unpack of the packed words run in-kernel over a buffer — is dispatched on the GPU through the harness and run natively, in one native `cargo test`, with the results compared exactly (integer fields, parity Tier B). And the payload is generated per precision: its widths derive from `size_of::<Real>()` for f32, f64 and a DoubleF64 stub row, never hardcoded to f32 — philosophy §7.1's one "decide now, costs nothing" item (§7.7).

## References
- `docs/contracts/principia_canonical_spec.md` § "1. The substrate (the defining decision)"
- `docs/design/principia_core_design.md` § "1. Physics defined once, compiled twice — now *structurally*, not by discipline"
- `docs/contracts/principia_lowering_contract.md` § "Part 2 — Two assembly mechanisms (the substrate split; the old "one mechanism" claim retires)"
- `docs/contracts/principia_parity_contract.md` § "6. The harness"
- `docs/contracts/principia_parity_contract.md` § "Tier B — integer-exact *given the same branch decisions* (integer & packed fields)"
- `docs/read_first/principia_00_philosophy.md` § "7.1 Extended and arbitrary precision"
- `docs/read_first/principia_00_philosophy.md` § "7.7 The rule for this section"
- `docs/read_first/principia_01_pitfalls.md` § "10. GENERALISING A STATELESS RESULT TO A TRAJECTORY"
- `docs/design/principia_dd_simstate_payload.md` § "1. `SimState` — the hot struct"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-169 — The GPU CI jobs, and an install step for every toolchain *(closes G1, H5)*"
- `docs/design/principia_dd_generation_root.md` § "3.6 `ICDescriptor` (12 × f32)"
- `decisions.md` § "R-313 — `ICDescriptor` follows `Real`; R-86's 64 B is its f32 instantiation *(closes RQ-185; amends R-86)*"
- `decisions.md` § "R-314 — The `rustc-check-cfg` declaration for `spirv` in `crates/kernel/build.rs` is accepted *(closes RQ-184)*"
- `decisions.md` § "R-316 — #96's `closure_min` widening with `Real` stands"
- `decisions.md` § "R-320 — CI caches the rust-gpu build, keyed on the pinned toolchain version *(amends R-285)*"
- `decisions.md` § "R-325 — CI: the GPU kernel build and its tests run in their own parallel job; ≤ ~10.5 min per job *(amends R-301)*"
- `decisions.md` § "R-326 — Actions caches are saved only on pushes to `main`; pull-request jobs restore only *(amends R-285, R-320)*"
- `decisions.md` § "R-331 — #96's veto item 11 stands: `ICDescriptor`'s `_pad` keeps 16 B (64 / 112 / 208 B)"

## Deliverables
- `rust-toolchain.toml` — the toolchain pin rust-gpu needs; CI installs it from this file in every job (R-169).
- `crates/kernel` — `#![no_std]`; a `Real` trait with f32 and f64 impls; the trivial kernel entry point (`#[spirv(compute)]`) over the generated pack/unpack.
- `xtask/src/build_kernel.rs` — `cargo xtask build-kernel`: spirv-builder → `target/spirv/kernel.spv`, naga → `target/spirv/kernel.wgsl`; registered in `cargo xtask ci`.
- `crates/ledger/src/gen/rust.rs` — per-precision emission: the payload structs instantiated per `Real`, widths from `size_of::<Real>()`; the DoubleF64 stub as a ledger precision row.
- `crates/validation/tests/toolchain.rs` — the trivial kernel on the GPU vs natively.
- `ICDescriptor` generic over `Real` too, with a layout row per precision; 64 B with declared padding is its f32 instantiation (R-313).
- `crates/kernel/build.rs` — `cargo::rustc-check-cfg=cfg(target_arch, values("spirv"))`, the lint configuration R-314 accepts; no `allow(unexpected_cfgs)` at any scope.
- Every workflow job that runs `build-kernel` caches `~/.cache/rust-gpu` under a key naming its job and the pinned toolchain version (R-320, REQ-SYS-075; per-job keys, R-285, REQ-SYS-073).
- `.github/workflows/ci.yml` — `build-kernel` and the tests that need the built kernel run in a job of their own, in parallel with `ci` (R-325, REQ-SYS-076).
- Every cache step in every workflow restores in every run and saves only in a run on a push to `main`; a pull-request run restores only. The rust-gpu cache keeps its key (R-326, REQ-SYS-073, REQ-SYS-075).

## Acceptance tests
- `cargo xtask build-kernel` — rust-gpu compiles `crates/kernel` to SPIR-V and naga translates it to WGSL, in CI.
- `cargo test -p validation toolchain_trivial_kernel` — the same kernel source, run natively and dispatched on the GPU, gives bit-identical packed words over 2¹⁶ inputs (integer-exact); a deliberately mis-built GPU variant (one field's offset shifted) differs (the test can fire).
- `cargo test -p kernel payload_real_generic` — the payload, `SimState` and `ICDescriptor`, instantiated for f32 and f64 (and the DoubleF64 stub row); every width derives from `size_of::<Real>()`; the layout is generated per precision; `ICDescriptor` at f32 is 64 B (REQ-PAY-017, R-313).
- Definition: the per-field Real dependence, the f64 layout and the DoubleF64 stub written into dd_simstate_payload §1, and `ICDescriptor`'s f64 and DoubleF64 sizes and padding into generation-root §3.6, approved by the physics reviewer (REQ-PAY-087, R-313).
- Review checklist (code) — each workflow job that runs `build-kernel` caches `~/.cache/rust-gpu` under a key containing its job name and the pinned toolchain channel; the PR shows a warm run's `build-kernel` step time beside a cold one (REQ-SYS-075, REQ-SYS-073, R-320).
- Review checklist (code, qa) — every cache step in every workflow saves only in a run on a push to `main`, and a pull-request run restores only; the PR shows the Actions cache listing and its total against 10 GB (REQ-SYS-073, REQ-SYS-075, R-326).
- Review checklist (code) — `ci.yml` runs `build-kernel` and the tests that need the built kernel in their own job, parallel to `ci`; the PR shows each CI job's warm wall time against ~10.5 min, names any job over it, and says the new job needs adding to the required checks (REQ-SYS-076, R-325, R-266).

## Notes
- PIT-10: the trivial kernel is stateless. It certifies the toolchain and the compiled pack/unpack, not trajectory parity, which is M4 (REQ-VAL-056, REQ-VAL-057).
- The trivial kernel is not the bring-up mode of colour_composition Appendix A (M1, R-75).
- No extended-precision arithmetic is built (REQ-SYS-007): DoubleF64 is a stub row for the width function only.
- See Gaps: the f64 payload layout, and the DoubleF64 stub's shape.
- Closes, for gaps the corpus leaves open: REQ-PAY-087 (R-72 definition) (classification accepted by R-132).
- Rulings on PR #96 (1 Oct): R-313 (RQ-185) — `ICDescriptor` follows `Real`, its 64 B the f32 instantiation; #96's §1
  sentence calling it "not generic" is superseded. R-314 (RQ-184) — the check-cfg declaration in `build.rs` stands, no
  blanket allow. R-316 — `closure_min` widening with `Real` stands. R-320 — the rust-gpu build cache joins this task,
  which brings kernel builds into CI.
- Rulings of 1 Oct (later): R-325 — the kernel build and its tests move to their own parallel CI job, and ~10.5 min is
  the target per job (REQ-SYS-076, new; closed here). R-326 — caches are saved only on pushes to `main`; this task
  makes the change in every workflow, since it already edits their cache steps (REQ-SYS-073 stays closed by
  TASK-M0-42; its R-326 change is made here). R-331 — #96's veto item 11, `_pad` at 16 B, stands. Applied per
  R-204 — veto?: R-326 also applies to the workflows TASK-M0-42 and others wrote, all in this PR. Size: the two CI
  rulings add workflow edits beyond the ~450 lines.
