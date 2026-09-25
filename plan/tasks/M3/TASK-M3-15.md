# TASK-M3-15 — The free-group word buffer: append, pop and the length cap

- **Milestone:** M3
- **Closes:** REQ-PAY-036, REQ-PAY-038, REQ-PAY-042, REQ-PAY-048, REQ-PAY-052, REQ-PAY-053, REQ-TOOL-035
- **Depends on:** TASK-M3-01
- **Needs (earlier milestones):** REQ-PAY-016, REQ-PAY-023, REQ-PAY-025, REQ-PAY-028, REQ-PAY-029, REQ-PAY-033
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~400 lines

## Goal
The kernel side of the word exists: the cold `vec4<u32>` word buffer indexed identically to SimState, never inline and never touched per step; the Horner mixed-radix append (`W₁ = d₀`, `W_{k+1} = 3W_k + e_k` over 121 bits, length in `.w` 25–31) following payload §3's algorithm — ignore once truncated, first symbol, cancellation-pop via div-mod and the reverse table, `length_raw = 127` on a non-cancelling push at length 76 (the length cap, not overflow); and `last_symbol` (descriptor bits 8–9) mirrored via `set_last_symbol` whenever `prev` changes.

## References
- `docs/contracts/principia_canonical_spec.md` § "6. Memory & deployment model *(authoritative: `memory_tiers`, `caching_contract`, `deep_zoom`, `systems_architecture`)*"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `docs/contracts/principia_render_contract.md` § "Part 1 — The payload (render input)"
- `docs/contracts/principia_render_contract.md` § "Unpack layer (generated, one accessor per named field)"
- `docs/design/principia_dd_simstate_payload.md` § "0. The two buffers (split by access pattern)"
- `docs/design/principia_dd_simstate_payload.md` § "3. The word buffer — `free_group_word`"
- `docs/design/principia_dd_generation_root.md` § "3.3a The word buffer — a parallel cold buffer"
- `docs/design/principia_dd_simstate_payload.md` § "`sample_descriptor` (low 16 bits of `packed_a`) — 10 used, rest reserved"
- `docs/design/principia_dd_simstate_payload.md` § "6. Accessors (illustrative of generated output; source is the Rust layout definition — emitted as Rust for the kernel/host and WGSL for the fragment side)"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `docs/design/principia_dd_generation_root.md` § "3.3 `free_group_word` (uint4) — mixed-radix packing, **in a separate buffer**"
- `decisions.md` § "R-70 — Where two docs conflict, the later consolidated doc wins"
- `docs/design/principia_debug_tooling_plan.md` § "B. Payload field views — `sample_descriptor` (bit-packed u32)"
- `docs/design/principia_debug_tooling_plan.md` § "C. Payload field views — `times` (u32), f16-packed scalars & `free_group_word` (separate buffer)"

## Deliverables
- `crates/kernel/src/word/append.rs` — `append(word, prev, symbol)`, the pop path, the truncation sentinel; 121-bit arithmetic over u32 limbs, no allocation.
- `crates/kernel/src/word/binding.rs` — the separate binding indexed by sample (per copy); the march kernel's per-step path has no word access, only the crossing hook (TASK-M3-16) and resolve/inspect.
- `crates/validation/src/reference/free_group.rs` — a reference free-group reducer for property tests.

## Acceptance tests
- `cargo test -p kernel word_buffer_binding` — the word buffer is a distinct binding with the same index; no per-step write outside a crossing (REQ-PAY-036).
- Review (code): SimState has no word member; the word buffer is a separate binding indexed by sample index; the march kernel never binds or reads it except at a crossing (REQ-PAY-038).
- `cargo test -p kernel word_write_count` — property test: instrumented CPU march over fixture trajectories, word-buffer write count == detected crossing count; no word reads in the per-step path (REQ-PAY-042).
- `cargo test -p kernel last_symbol_mirror` — property test: random crossing sequences, whenever 1 ≤ length ≤ 76 last_symbol equals the final symbol of decode(W); other packed_a bits unchanged by every set (REQ-PAY-048).
- `cargo test -p kernel word_horner_packing` — property test: append random reduced 76-symbol words; packing matches the closed form and fits 121 bits; length field values (REQ-PAY-052).
- `cargo test -p kernel word_vs_reference_reducer` — property test against the reference reducer on random symbol streams, incl. 77-symbol words with small numeric W (REQ-PAY-053).
- `cargo test -p kernel word_truncation_sentinel` — pushing a 77th symbol reads length 127, including for a word whose numeric W is small; reduction pops decrement the length (REQ-TOOL-035).

## Notes
- REQ-PAY-042's crossing count needs TASK-M3-16's detector; until then the test drives the hook from a synthetic crossing stream, and TASK-M3-16 re-runs it on real trajectories.
