# TASK-M2-41 — Tunable link parameters: the declarations and their check, `LinkParams` in the generated links, the active value everywhere, the log-det at it, and the hash (R-481) ⚑

- **Milestone:** M2
- **Closes:** REQ-GEN-037, REQ-GEN-038, REQ-GEN-039, REQ-GEN-040, REQ-GEN-041, REQ-GEN-042, REQ-DEC-048
- **Depends on:** TASK-M2-01, TASK-M2-30, TASK-M2-15, TASK-M0-46
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~700 lines, the tests included

## Goal
Each link registry entry declares its tunable parameters (`mu_max` on `softmax_tanh`, `q_max` on the momentum links) with a default, a calibrated range and a citation; `check` proves Part 2.5's three requirements across each range; the generator emits the links with a `LinkParams` argument in place of the constants read by name, bitwise today's at the defaults; decode, encode and the codomain bounds read the active value; the log-det reweights at it; the declarations are hashed into the schema version (R-481, with R-482's C1 control, C2 on the link functions, C4 and C5).

## References
- `decisions.md` § "R-481 — Link parameters are adjustable ⚑: each registry entry declares its tunable parameters with a name, a default, an allowed range and a citation, today `μ_max` on `softmax_tanh` and `q_max` on the momentum links, `α_min` staying fixed; every value of a range keeps Part 2.5's three hard requirements, `check` testing its ends and a sampled interior; the values are recorded beside the link id wherever it is recorded; the log-det reweights at the active value; the generated code reads the parameters as a uniform, bitwise today's at the defaults *(amends R-10)*"
- `decisions.md` § "R-482 — The tests of R-478 to R-481, each with an arm that can fail: the dropdown compatible-only, the defaults bitwise today's, a parameter change moving the result, the range's ends admissible, the log-det at a non-default value, provenance reproducing bitwise, the preview the kernel's own function, and Home leaving the links alone"
- `docs/contracts/principia_chart_decoder_contract.md` § "Three hard requirements on any registered link"
- `docs/contracts/principia_chart_decoder_contract.md` § "Links carry a measure"
- `docs/design/principia_dd_generation_root.md` § "3.9 The link registry (consolidated from chart contract Part 2.5)"
- `docs/contracts/principia_inverse_encode_contract.md` § "Chart-aware validation"

## Deliverables
- The tunable-parameter table in `crates/ledger/src/links.rs`, its `check` (ends and a seeded interior of at least 16 values) and its hash block in `crates/ledger/src/version.rs`.
- `LinkParams<R>` and `LinkParams::DEFAULT`; the emitter's parametrised forward, inverse and log-det; the existing at-default functions kept, delegating.
- The active value read by decode, encode (`crates/kernel/src/encode/blocks.rs`), `Codomain::range` and encode's layer 1.
- The range proposal for REQ-GEN-038 with its evidence, in the PR.
- Tests `link_params_declared`, `link_range_admissible`, `link_params_hashed`, `link_params_default_bitwise`, `link_logdet_params`, `link_params_active`.

## Acceptance tests
- `cargo test -p ledger link_params_declared` — REQ-GEN-037's verify (C1's control: an injected entry refused by `check`).
- Review (physics): the proposed ranges with their evidence, both ends and the interior, the inverse's conditioning at f32 and f64 (REQ-GEN-038, calibration).
- `cargo test -p kernel link_logdet_params` — REQ-GEN-039's verify (C5, with its mutation arm).
- `cargo test -p kernel link_params_default_bitwise` — REQ-GEN-040's verify (C2 on the link functions; control one ulp off).
- `cargo test -p ledger link_range_admissible` — REQ-GEN-041's verify (C4).
- `cargo test -p ledger link_params_hashed` — REQ-GEN-042's verify.
- `cargo test -p kernel link_params_active` — REQ-DEC-048's verify.

## Notes
- **⚑ Physics review is required** (the instruction's "How to work this", 4): this task does not merge on the code reviewer's word alone; its reviewer list includes physics. It covers each range (G2), the inverse's conditioning at its ends at f32 and f64 (G3, C4), and the log-det against the forward at non-default values by finite differences (G6, C5) (R-481 G8).
- A5 is resolved as a uniform (R-481 G7, applied per R-369): the parameters are an argument, never a compiled variant; the GPU's uniform is TASK-M4-21's. The at-default functions stay, so no existing caller or qa file changes; a qa file that would have to change is a C2 failure.
- The ranges are provisional until the M2 gate (REQ-GEN-038, R-71); nothing waits on them. R-597 and R-625 (25 Oct 2026) cap `μ_max`'s range at 10; the proposal merged here, `mu_max ∈ [1, 10]` (`crates/ledger/src/links.rs`, `tunables()`), is within the cap, so nothing changes.
- R-10 is amended (R-481): `μ_max = 5` and `q_max = 2` become the defaults; `constants.rs` keeps the values and still cites R-10.
