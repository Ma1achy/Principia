# TASK-M0-12 — The schema version: the content hash of the ledger

- **Milestone:** M0
- **Closes:** REQ-GEN-008
- **Depends on:** TASK-M0-11
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~200 lines

## Goal
The payload schema version is computed at generation as a content hash of the canonicalised ledger — every layout entry, the frozen continuation table (R-36, R-63) and each constants-register entry that decides what stored bits mean, by its value, type and class (R-251) — and emitted as a constant into the generated Rust (and, with TASK-M0-13, the WGSL). No hand-bumped version number exists anywhere: changing one ledger row or one continuation-table entry changes the version, and an unchanged ledger gives the same version on every run and machine.

## References
- `decisions.md` § "R-36 — Schema version = content hash of the ledger *(PL-1)*"
- `decisions.md` § "R-63 — The continuation table is hashed with the ledger *(confirms R-36's application)*"
- `decisions.md` § "R-22 — Body indices are 0-based; pair ids name the opposite side *(CD-2, amended)*"
- `docs/design/principia_dd_simstate_payload.md` § "3. The word buffer — `free_group_word`"
- `docs/design/principia_dd_generation_root.md` § "5. Tests (properties any generator must satisfy)"
- `docs/design/principia_dd_generation_root.md` § "6. Deferred / flagged"
- `docs/design/principia_dd_generation_root.md` § "4. Seams (obligations → integration tests)"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- `crates/ledger/src/version.rs` — the canonical serialisation of the ledger (fixed field order, no formatting-dependent bytes) and its hash; `PAYLOAD_SCHEMA_VERSION` emitted into `crates/kernel/src/payload/generated.rs`.
- `crates/ledger/tests/schema_version.rs`.

## Acceptance tests
- `cargo test -p ledger schema_version` — changing one ledger row, then one continuation-table entry, changes the schema version each time; the unchanged ledger gives a stable version across two runs; the emitted constant equals the computed hash (REQ-GEN-008).
- `cargo test -p ledger schema_version` — editing a hashed register entry's value, type or class changes the schema version; a citation-only edit to it does not; an entry that doesn't decide stored bits leaves the version unchanged (REQ-SYS-063, R-251). §3.8 gives the criterion but no field marking an entry as hashed, so this task says how it selects them.

## Notes
- Constants-register entries: TASK-M0-08's R-72 definition (§3.8 "The hash") put the entries that decide stored bits inside the schema hash, and R-251 made it cover their value, type and class, not their citation text. This replaces the earlier "outside the hashed layout table unless ruled otherwise".
- RQ-93 ruled: R-113 — "caching signature carries it" is dropped from REQ-GEN-008's verify; the compatibility signature carrying the version is REQ-GEN-017 (M5, TASK-M5-06).
