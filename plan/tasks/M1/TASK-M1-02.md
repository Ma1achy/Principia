# TASK-M1-02 — The word buffer: mixed-radix packing, sequential decode and the word accessors

- **Milestone:** M1
- **Closes:** REQ-PAY-023, REQ-PAY-025, REQ-PAY-028, REQ-PAY-029, REQ-PAY-033
- **Depends on:** TASK-M1-01
- **Needs (earlier milestones):** REQ-PAY-016, REQ-GEN-003, REQ-GEN-004, REQ-GEN-008, REQ-RENDER-001, REQ-RENDER-002
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-3
- **Size:** ~400 lines

## Goal
The cold word buffer exists as its own physical buffer beside `SimState`: one `vec4<u32>` (16 B) per sample, indexed identically. The free-group word is packed mixed-radix (76 symbols in 121 bits over x, y, z and .w[0:24]) with a 7-bit length at .w[25:31] and truncation signalled only by the length sentinel 127. The shared-source append (free reduction via the frozen continuation table, push at length 76 → 127) and the sequential O(length) decode exist in Rust, and the word accessors (`fgw_length_raw`, `fgw_truncated`, `fgw_reduced_length`/`_valid`, `fgw_retained_prefix_length`, `fgw_symbol`, `sd_last_symbol_valid`) are generated for Rust and WGSL, one name each.

## References
- `docs/contracts/principia_render_contract.md` § "Unpack layer (generated, one accessor per named field)"
- `docs/design/principia_dd_simstate_payload.md` § "0. The two buffers (split by access pattern)"
- `docs/design/principia_dd_generation_root.md` § "3.3 `free_group_word` (uint4) — mixed-radix packing, **in a separate buffer**"
- `docs/design/principia_dd_generation_root.md` § "3.3a The word buffer — a parallel cold buffer"
- `docs/design/principia_dd_simstate_payload.md` § "3. The word buffer — `free_group_word`"
- `docs/design/principia_dd_simstate_payload.md` § "5. Derived — computed at read, NOT stored"
- `docs/design/principia_dd_simstate_payload.md` § "6. Accessors (illustrative of generated output; source is the Rust layout definition — emitted as Rust for the kernel/host and WGSL for the fragment side)"
- `docs/contracts/principia_symbolic_dynamics_contract.md` § "4. Validity under truncation"
- `docs/contracts/principia_symbolic_dynamics_contract.md` § "What is already settled (in the payload spec, not here)"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `decisions.md` § "R-22 — Body indices are 0-based; pair ids name the opposite side *(CD-2, amended)*"

## Deliverables
- `crates/kernel/src/word.rs` (shared source, f32/f64-agnostic): `fgw_append` (free reduction, cap at 76 → length 127), `fgw_decode` (pop the base-3 tail, residue as d₀, replay forward through the continuation table), `set_last_symbol`.
- `crates/ledger`: word-buffer element type (exactly 16 B) and the generated word accessors in Rust and WGSL; no word member in any generated `SimState` type.
- Tests: `crates/kernel/tests/word.rs` (proptest) and `crates/ledger/tests/word_accessors.rs`; a truncated fixture word.

## Acceptance tests
- `cargo test -p kernel word_roundtrip` (proptest) — random words up to 76 symbols encode/decode round-trip; length 127 ⇒ `fgw_truncated`; the retained-prefix helper clamps 127 → 76 (REQ-PAY-023).
- `cargo test -p ledger word_buffer_split` — the generated `SimState` types contain no `vec4<u32>`/word member and the word buffer element is exactly 16 B (REQ-PAY-025).
- `cargo test -p ledger truncated_word_validity` — truncated fixture word: `fgw_reduced_length_valid` false, `sd_last_symbol_valid` false, retained prefix length 76 (REQ-PAY-028).
- `cargo test -p kernel word_decode_reduced` (proptest) — `decode(append(s))` equals the freely reduced s for random streams up to 76 symbols (REQ-PAY-029).
- `cargo test -p ledger word_accessor_names` — accessor outputs for length 0, 1, 76 and 127; the generated unpack layer defines no second name for any accessor (no `fgw_prefix_length`) (REQ-PAY-033).

## Notes
- Interpreting the word into pair tallies (`enc_XY`, `dominant_pair`) stays out (REQ-RENDER-002, M0).
- The decode round-trip must be shown able to fail: a corrupted continuation-table entry or a dropped base-3 digit must break `word_decode_reduced` (PIT-9).
