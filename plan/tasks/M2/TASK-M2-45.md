# TASK-M2-45 — The native build without MIR GVN on the pinned nightly, a release-profile CI job, and the upstream report (R-547)

- **Milestone:** M2
- **Closes:** REQ-TOOL-188, REQ-TOOL-189
- **Depends on:** TASK-M0-14, TASK-M2-07
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~120 lines

## Goal
The pinned nightly (`rust-toolchain.toml`, `nightly-2026-04-11`) miscompiles plain Rust above opt-level 0 in rustc's
MIR GVN pass (RQ-299; `stage_canon_native` fails at the release profile). R-547 rules (b) + (c), then (a): the native
build sets `-Zmir-enable-passes=-GVN`, scoped to the host target and not the SPIR-V build; a CI job runs the native
tests at the release profile; the fault is reported upstream; the pin moves to a fixed nightly once
`rustc_codegen_spirv` builds against one.

## References
- `decisions.md` § "R-547 — Decode and encode numerics: the swap test is the conjugacy; step 0 is skipped within a calibrated `τ_I` and rescales in two steps; a branch input that must agree across backends is built only from operations every IEEE backend rounds alike, with no subnormal intermediate; the native build runs without MIR GVN and CI runs it at the release profile; T1 holds off the mirror band; REQ-SYS-015 is split *(closes RQ-283, RQ-295, RQ-296, RQ-298, RQ-299, RQ-360 and RQ-365; corrects R-34)*"
- `docs/contracts/principia_canonical_spec.md` § "1. The substrate (the defining decision)"
- `decisions.md` § "R-169 — The GPU CI jobs, and an install step for every toolchain *(closes G1, H5)*"

## Deliverables
- The host-target rustflags (`.cargo/config.toml` or the build's equivalent) carrying `-Zmir-enable-passes=-GVN`,
  never reaching the rust-gpu SPIR-V build.
- RQ-299's standalone reproducer as a test, and a check that the SPIR-V build's flags carry no `-Zmir-enable-passes`.
- A CI job that runs the native tests at the release profile.
- The upstream report drafted in the PR (the reproducer, the toolchain table, the pass named).

## Acceptance tests
- `cargo test -p kernel --release --test canon` — `stage_canon_native` passes; the reproducer prints 0.5 at opt-level
  1, 2 and 3 with the workspace's host flags; control: on the pin without the flag it prints −0.5; the SPIR-V build's
  flags carry no `-Zmir-enable-passes` (REQ-TOOL-188).
- CI log on the PR — the release-profile job green; a `measure/` run (R-272) shows it failing `stage_canon_native`
  with the flag removed (REQ-TOOL-189).

## Notes
- Applied per R-369: filing the upstream report on rust-lang/rust is outside the repo, so the PR drafts it and the
  human files it.
- Applied per R-369: reviewers code and qa; the change is build and CI configuration, ruled by R-547, and changes no
  physics.
- The pin's move, R-547's "then (a)", is a toolchain change through `rust-toolchain.toml` in its own PR when a fixed
  nightly that `rustc_codegen_spirv` builds against exists (R-414); it drops the flag.
