# TASK-M0-13 — The WGSL generator: the fragment-side unpack layer

- **Milestone:** M0
- **Closes:** REQ-RENDER-001, REQ-PAY-016
- **Depends on:** TASK-M0-10, TASK-M0-11, TASK-M0-12
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
The same ledger now also emits WGSL (generation-root §1: one source, two language targets): the fragment-side unpack layer in `crates/render/frag/generated/payload_unpack.wgsl` — the stored `SimState` layouts, the `sd_*`, `pa_`/`pb_`/`tm_` and `fgw_*` accessors with the payload §6 names, the continuation tables as shader constants and the schema version. The generated WGSL uses only the u32 overload of `extractBits`, has no f64, reads f16 pairs through core `unpack2x16float` with no `enable f16`, groups `r`, `p` and the shadow as vec2 members, and binds the word buffer separately with per-copy indexing. A lint over naga's IR holds these, and the Rust and WGSL continuation tables are asserted equal to the frozen arrays.

## References
- `docs/contracts/principia_render_contract.md` § "Unpack layer (generated, one accessor per named field)"
- `docs/design/principia_dd_generation_root.md` § "1. What it is"
- `docs/design/principia_dd_generation_root.md` § "2. Consolidated contract"
- `docs/design/principia_dd_generation_root.md` § "3.3 `free_group_word` (uint4) — mixed-radix packing, **in a separate buffer**"
- `docs/design/principia_dd_simstate_payload.md` § "6. Accessors (illustrative of generated output; source is the Rust layout definition — emitted as Rust for the kernel/host and WGSL for the fragment side)"
- `docs/design/principia_dd_simstate_payload.md` § "3. The word buffer — `free_group_word`"
- `docs/contracts/principia_lowering_contract.md` § "Part 2 — Two assembly mechanisms (the substrate split; the old "one mechanism" claim retires)"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `decisions.md` § "R-63 — The continuation table is hashed with the ledger *(confirms R-36's application)*"

## Deliverables
- `crates/ledger/src/gen/wgsl.rs` — the WGSL emitter; `cargo xtask codegen` writes `crates/render/frag/generated/payload_unpack.wgsl`.
- `xtask/src/lint_wgsl.rs` — `cargo xtask lint wgsl`: parses the generated WGSL with naga and checks every `extractBits` argument's type is u32, no f64 type, no `enable f16`, the vec2 groupings and the separate word binding; registered in `cargo xtask ci`.
- Fixture WGSL files that break each rule, for the lint's own tests.

## Acceptance tests
- `cargo xtask lint wgsl` — on the generated WGSL: every `extractBits` argument is u32, no f64, no `enable f16`, `r`/`p`/`r_sh`/`p_sh` are `array<vec2<f32>, 3>`, the word buffer is its own binding indexed per copy; review checklist (code §4) grep of the generated WGSL agrees (REQ-RENDER-001).
- `cargo test -p xtask lint_wgsl` — a fixture with an i32 `extractBits`, one with an f64 and one with `enable f16` each fail, naming the rule (REQ-RENDER-001; the lint can fire).
- `cargo test -p ledger continuation_table` — the generated Rust and WGSL tables equal the frozen arrays (`inverse = [1,0,3,2]`, `cont_symbol[0] = [0,1,2,3]`, `[1] = [2,3,0,1]`, `[2] = [3,2,1,0]`); each digit map is an involution; no continuation equals `inverse(prev)` (REQ-PAY-016).

## Notes
- The WGSL debug catalogue and the Rust export decoder (the other two generated artefacts) are M1 (REQ-GEN-010, REQ-GEN-011).
- The fragment read side's tier-uniform interface (`has_<feature>` consts, NaN for absent features — lowering Part 3a) is M1's; this task emits the stored layouts only.
- The GPU self-test of this WGSL is TASK-M0-15.
