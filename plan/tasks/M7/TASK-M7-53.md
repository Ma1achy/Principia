# TASK-M7-53 — The schema version hashes only the layout and kernel columns; a ledger scale edit re-styles with zero recompute (R-545, RQ-297)

- **Milestone:** M7
- **Closes:** REQ-GEN-043
- **Depends on:** TASK-M7-05, TASK-M5-06
- **Needs (earlier milestones):** REQ-GEN-008, REQ-GEN-017
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~200 lines

## Goal
**R-545** rules RQ-297, option (a): "The schema version hashes only the ledger columns that decide the payload's
layout or the kernel's output. `scale`, and any render-only column a ruling names, leaves the hash." Today
`crates/ledger/src/version.rs` hashes each entry's `scale` into the schema version, so a ledger scale edit invalidates
every cached payload, against seam 5's "zero recompute" (dd_colouring §4, REQ-COL-032). This task takes `scale` out of
the hash and runs seam 5 end to end.

## References
- `decisions.md` § "R-545 — The human's batch of 21 Oct 2026, ported by area as R-545 to R-559; and the ledger, payload and schema: `running_mean_divergence` and `coherence` one-signed, `alpha_energy` centred on 1.0, a substeps-per-pixel histogram in the frame record, a schema version that hashes only layout and kernel columns, and descriptor bits 10–14 for `I_max` with bit 15 R-37's departed bit *(closes RQ-278, RQ-286, RQ-297 and RQ-310; amends R-36 and R-56)*"
- `decisions.md` § "R-36 — Schema version = content hash of the ledger *(PL-1)*"
- `docs/design/principia_dd_generation_root.md` § "2. Consolidated contract"
- `docs/design/principia_dd_generation_root.md` § "4. Seams (obligations → integration tests)"
- `docs/design/principia_dd_colouring.md` § "4. Seams (obligations → integration tests)"
- `docs/contracts/principia_caching_contract.md` § "Part 1 — Two-level keying: identity vs validity"

## Deliverables
- `crates/ledger/src/version.rs`: the schema version hashes only the columns that decide the payload's layout or the
  kernel's output; `scale` leaves the hash; the set of excluded columns is one named list, citing R-545.
- Seam 5's end-to-end test: a regenerated ledger with one field's `scale` changed.

## Acceptance tests
- `cargo test -p ledger schema_version_columns` — changing one entry's `scale` leaves the schema version unchanged;
  changing a bit offset, a width, a type or a continuation-table entry changes it; control: a variant hashing `scale`
  fails (REQ-GEN-043).
- `cargo test -p engine schema_signature_scale` — REQ-GEN-017's cache test extended: one field's `scale` changed alone
  leaves the hash and the compatibility signature unchanged and the cached payload is served; a bit offset flipped still
  invalidates it; control: a variant hashing `scale` fails (REQ-GEN-043).
- `cargo test -p render ledger_scale_restyle_zero_recompute` — seam 5 end to end: with the regenerated ledger the view
  re-styles, the cache serves the same payload and the dispatch counter is unchanged (REQ-GEN-043, REQ-COL-032).

## Notes
- Applied per R-369 (R-545 C1): REQ-COL-032 stays TASK-M7-05's (merged), its hold lifted; its zero-recompute arm runs
  here. REQ-GEN-017 (TASK-M5-06) is restated to "a layout or kernel change"; REQ-GEN-008 (TASK-M0-12, merged) stays as
  built. Every `scale` arm is REQ-GEN-043's, here, since neither earlier task can pass it (fix round 1 of PR #274, qa
  Q2, applied per R-369). *Was:* "REQ-GEN-008 (TASK-M0-12) and REQ-GEN-017 (TASK-M5-06) are restated to \"a layout or
  kernel change\"."
- qa exception forced by R-545 (RQ-297), named per R-290 and R-369: `crates/ledger/tests/qa_TASK-M0-12.rs:143-149`, the
  `scale` edit in `entry_edits` run by `qa_m012_every_field_of_every_entry_changes_the_version` (module doc :4), asserts
  that a `scale` change moves the version, the opposite of the ruling. It changes here as an `M` line on a qa file whose
  commits are all qa's (f4cb7e8e, 1c68f65c): the `scale` edit moves to a "leaves the version unchanged" arm. The code
  reviewer confirms no other assertion is weakened.
