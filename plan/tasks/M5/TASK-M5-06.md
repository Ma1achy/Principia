# TASK-M5-06 — Sim key, render key, cache identity and the payload compatibility signature

- **Milestone:** M5
- **Closes:** REQ-SCHED-048, REQ-SCHED-014, REQ-SCHED-015, REQ-SCHED-027, REQ-GEN-017, REQ-SCHED-018
- **Depends on:** TASK-M5-03
- **Needs (earlier milestones):** REQ-SCHED-005, REQ-SCHED-007, REQ-GEN-008, REQ-RENDER-075, REQ-SYS-021, REQ-INT-026
- **Reviewers:** code, qa
- **Pitfalls:** PIT-9
- **Size:** ~400 lines

## Goal
The two keys and the cache key exist as typed values. The sim key holds chart id + params, the slice plane
(z₀'s out-of-plane part, span{q₁, q₂}, in-plane orientation), warps, link ids, occupant, T / dt / thresholds, the tier's
sim-key components (N, FTLE, word) and the schema; the render key holds the stain graph, node params, overlays and
palette/compaction; the playhead, in-plane pan and zoom, the lock and E are in neither. Cache identity is
`(chart id + params, slice plane) + QuadID`; the compatibility signature carries chart/decode version, link ids,
occupant + config (incl. `owns_time_mapping`), T, tier flags, thresholds, `copy_index` for a copy (the nominal's excluding
E) and the ledger content hash; lookup requires identity and signature match. v1 has no IC-keyed lookup.

## References
- `docs/design/principia_systems_architecture.md` § "The two keys as ladder geometry"
- `docs/contracts/principia_canonical_spec.md` § "8. Locked vocabulary"
- `docs/contracts/principia_caching_contract.md` § "Part 1 — Two-level keying: identity vs validity"
- `docs/contracts/principia_integrator_contract.md` § "What this preserves, and what it costs"
- `docs/design/principia_dd_generation_root.md` § "2. Consolidated contract"
- `docs/design/principia_dd_generation_root.md` § "4. Seams (obligations → integration tests)"
- `docs/design/principia_dd_generation_root.md` § "5. Tests (properties any generator must satisfy)"
- `docs/design/principia_dd_generation_root.md` § "6. Deferred / flagged"
- `docs/design/principia_dd_simstate_payload.md` § "`sample_descriptor` (low 16 bits of `packed_a`) — 10 used, rest reserved"
- `docs/contracts/principia_caching_contract.md` § "Part 3 — Cross-chart sharing: permitted, deferred"
- `decisions.md` § "R-89 — Depth and E are not on the sim key *(closes RQ-40)*"
- `decisions.md` § "R-92 — What the sim key holds of navigation *(closes RQ-43)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- `crates/engine/src/keys.rs`: `SimKey`, `RenderKey`, `CacheIdentity`, `CompatSignature` (hashing; the schema hash
  from the ledger generator).
- Tests `crates/engine/tests/keys.rs`: one test per key component flipping exactly the stated key; signature misses per
  component; E leaves the nominal signature unchanged; a ledger bit-offset flip changes hash and signature.

## Acceptance tests
- `cargo test -p engine keys_flip_exactly` — changing each listed component flips exactly the stated key; advancing the playhead, panning or zooming in-plane, toggling the lock or changing E flips neither (REQ-SCHED-048).
- `cargo test -p engine cache_identity` — same (z,tx,ty) under two charts or two slice planes (tilt, rotation, out-of-plane slice) → two distinct entries; an in-plane pan or zoom on the same plane keeps the identity (REQ-SCHED-014).
- `cargo test -p engine compat_signature` — changing any listed component misses the cache; changing E leaves the nominal's signature unchanged (REQ-SCHED-015).
- `cargo test -p engine signature_occupant` — change only the occupant (or the regularisation) -> compatibility hash changes -> payload re-integrated (REQ-SCHED-027).
- `cargo test -p engine stale_schema` — flip one bit offset → hash and signature change; the cache test serves zero stale-schema payloads (REQ-GEN-017).
- Review checklist (code) — no IC-keyed lookup in v1 (REQ-SCHED-018).

## Notes
- Each per-component test must be shown to fail when that component is dropped from the hash (pitfalls §9).
- REQ-GEN-017's "the cache test serves zero stale-schema payloads" is asserted again end to end once the cache exists
  (TASK-M5-08 reuses this test's fixture).
- RQ-93 ruled: R-113 — REQ-GEN-008 (M0) verifies only the schema version; the compatibility signature carrying it is REQ-GEN-017, closed here.
