# TASK-M0-46 — The link registry in the schema version: each link hashed by its semantic content

- **Milestone:** M0
- **Closes:** REQ-GEN-031, REQ-GEN-032
- **Depends on:** TASK-M0-12
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~200 lines

## Goal
TASK-M0-12 computes `PAYLOAD_SCHEMA_VERSION` over the canonicalised ledger, but not over the link registry
(generation-root §3.9). The registry's definitions decide how a chart coordinate decodes, so changing them changes what
cached and saved results mean (R-340). The registry joins the hashed ledger: each entry by its semantic content, its
name, constraint, forward, inverse, log-det, ε clamps and the parameters its functions read, and not its sampling note,
which is prose, as R-251 leaves out a constant's citation. Every chart constant the registry holds is hashed too, by
value, including those no link reads, `δ_λ` and `ε_w`, since they change how a coordinate decodes (R-344). The registry's own entries are TASK-M2-01's; this task
builds the hashed table and its canonical serialisation and tests them over test entries. The version's value changes
when this lands, since the canonical serialisation gains the registry's table.

## References
- `decisions.md` § "R-340 — The schema version hashes each link registry entry's semantic content, not its prose *(applies R-251)*"
- `decisions.md` § "R-344 — `δ_λ` and `ε_w` are hashed; #108's four "veto?" items are accepted *(closes RQ-189; amends R-340)*"
- `decisions.md` § "R-251 — TASK-M0-08 accepted at ~918; its register items, with the hash covering value, type and class"
- `decisions.md` § "R-36 — Schema version = content hash of the ledger *(PL-1)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_generation_root.md` § "3.9 The link registry (consolidated from chart contract Part 2.5)"
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"
- `docs/design/principia_dd_generation_root.md` § "2. Consolidated contract"
- `docs/design/principia_dd_generation_root.md` § "5. Tests (properties any generator must satisfy)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Integrity: the link is part of the experiment"
- `docs/contracts/principia_chart_decoder_contract.md` § "Three hard requirements on any registered link"

## Deliverables
- `crates/ledger`: the link registry's entry type, its hashed members as §3.9 "The hash" lists them (name, constraint,
  forward, inverse, log-det, ε clamps, the parameters its functions read), and its sampling note, which is not hashed.
- `crates/ledger/src/version.rs`: `Hashed` gains the link registry and its chart constants; `canonical` writes each
  entry's hashed members by its rules (fixed order, length-prefixed strings, numbers by their bits), entries sorted by
  name, and each chart constant by name and value, sorted by name (R-344); `emit` hashes the registry the ledger holds,
  empty until TASK-M2-01.
- The definition of the canonical form of a link's forward, inverse and log-det, written into generation-root §3.9
  (REQ-GEN-032, R-72), such that TASK-M2-01's entries can be written in it.
- `crates/ledger/tests/schema_version.rs` (or a test file beside it): the tests below, each with a registered negative
  control (R-176).

## Acceptance tests
- `cargo test -p ledger schema_version_links` — over test registry entries, editing one entry's name, constraint,
  forward, inverse, log-det, an ε clamp or a parameter changes the schema version, each separately; editing a chart
  constant no link reads (a test `δ_λ`, a test `ε_w`) changes it, each separately (R-344); adding and removing an entry
  each change it; a sampling-note-only edit does not; the unchanged registry gives a stable version
  across two runs; the emitted `PAYLOAD_SCHEMA_VERSION` equals the computed hash (REQ-GEN-031, R-340).
- `cargo test -p ledger schema_version` — TASK-M0-12's tests still pass (REQ-GEN-008, REQ-SYS-063).
- Definition: the canonical form of a link's forward, inverse and log-det in the hashed bytes, written into
  generation-root §3.9, with no formatting-dependent bytes, approved by the physics reviewer (REQ-GEN-032).

## Notes
- RQ-189 ruled: R-344 — the registry's chart constants that no link reads (`δ_λ`, `ε_w`) are hashed too, by value. The
  task no longer waits.
- R-340 (applied per R-204, accepted by R-344): the entry's name is hashed, as TASK-M0-12 hashes each layout entry's
  and register constant's name, and every entry is hashed, whether or not a block uses it by default.
- The physics reviewer reviews it, as for TASK-M0-12 (R-251): what the schema version covers is a physics definition.
- `PAYLOAD_SCHEMA_VERSION` changes value when this lands, and again when TASK-M2-01 adds its entries (R-340, R-36).
