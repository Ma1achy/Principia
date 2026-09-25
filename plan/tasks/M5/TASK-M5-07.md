# TASK-M5-07 — Payload store: the per-copy index map, chunk sharding, SimState and word in lockstep

- **Milestone:** M5
- **Closes:** REQ-PAY-067, REQ-PERF-020, REQ-SCHED-042
- **Depends on:** TASK-M5-06
- **Needs (earlier milestones):** REQ-PAY-025, REQ-PAY-036, REQ-PAY-038, REQ-PERF-008, REQ-SCHED-002
- **Reviewers:** code, qa, perf
- **Pitfalls:** PIT-10
- **Size:** ~420 lines

## Goal
The logical `SimState` and word buffers exist as per-quad chunk buffers that respect the 128 MiB binding and
256 MiB buffer limits, with the engine tracking which quad lives in which chunk. One `(quad, sample, copy)` → linear
index map addresses both buffers, so there are exactly E+1 words per footprint and none for shadows. A `SimState` and its
word are allocated, evicted, recomputed, resumed and restored after device loss together, under one signature.

## References
- `docs/design/principia_dd_simstate_payload.md` § "0. The two buffers (split by access pattern)"
- `docs/design/principia_dd_generation_root.md` § "3.3a The word buffer — a parallel cold buffer"
- `docs/design/principia_dd_generation_root.md` § "3.3 `free_group_word` (uint4) — mixed-radix packing, **in a separate buffer**"
- `docs/design/principia_dd_simstate_payload.md` § "7. Memory"
- `docs/design/principia_memory_tiers.md` § "8. Caveats"
- `docs/contracts/principia_caching_contract.md` § "Part 1 — Two-level keying: identity vs validity"

## Deliverables
- `crates/engine/src/store/{index.rs, chunks.rs, payload.rs}`: index map, chunk allocator, the paired
  `PayloadSlot` (both buffers, one signature, one `t`).
- Property test `crates/engine/tests/store.rs`: index fuzz; random allocate/evict/resume/recompute/device-loss sequences
  against a never-evicted reference run (CPU reference driver).
- Unit test: multi-GB logical allocation never creates a `GPUBuffer` > 256 MiB or a binding > 128 MiB.

## Acceptance tests
- `cargo test -p engine index_map_fuzz` — fuzz (quad, sample, copy, E) and assert both buffers resolve to the same index; word count per footprint == E+1 (REQ-PAY-067).
- `cargo test -p engine chunk_sharding` — allocate a multi-GB payload: no GPUBuffer > 256 MiB, no binding > 128 MiB; quad→chunk map is consistent (REQ-PERF-020).
- `cargo test -p engine payload_lockstep` — random sequences of allocate/evict/resume/recompute/device-loss on a quad: after each, SimState[i].t_end_step and word[i] equal those of a never-evicted reference run (REQ-SCHED-042).

## Notes
- The lockstep property test compares after each operation on long marches, not on a held-fixed state
  (pitfalls §10: a result with inputs held fixed says nothing where they move).
