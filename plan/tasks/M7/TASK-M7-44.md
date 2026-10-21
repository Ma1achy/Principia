# TASK-M7-44 — The embedding's tiers: the tier table, the drop from the bottom and never below physics, the colour flag and layout version 4, the report, old records, and capacity re-measured (R-483, R-484) ⚑

- **Milestone:** M7
- **Closes:** REQ-TOOL-173, REQ-TOOL-174, REQ-TOOL-179, REQ-TOOL-180, REQ-TOOL-181
- **Depends on:** TASK-M7-31, TASK-M7-32, TASK-M7-45
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~600 lines, the tests included

## Goal
Every field of `SimConfig` and `RenderState` is given a tier in one compile-time-exhaustive table (`crates/engine/src/contract/tiers.rs`); the image's low-bit writer embeds the largest whole-tier set that fits, never cutting a tier and never below tier 1, writing nothing ("not embedded") if tier 1 does not fit; the record's flags say which tiers it holds (bit 3 `colour`, layout version 4); writer and reader report them; a pre-X1 record (`"links":{}`) recreates under its own build's defaults or on acceptance; §7's capacity is re-measured.

## References
- `decisions.md` § "R-483 — Export, sharing and the embedded record carry the decoder: `SimConfig`'s `Links` holds each block's link id and parameter values, the one place they live, written in full even at the defaults; the embedded record's list gains the links, the tilt fields, the energy-axis mappings and a saved chart by value; a record's decoder is the build's registry plus its links, an unknown or changed link refused and never substituted; an old record reads under its own build's defaults; capacity is re-measured and a record never truncated; a link change between keyframes is a cut; the sim key and the compatibility signature gain the parameter values; two questions queued (RQ-384, RQ-385)"
- `decisions.md` § "R-484 — The round-trip rule: importing any export is the identity on `SimConfig` and `RenderState`, bitwise in the canonical serialisation, on every path but the image's low-bit embedding, which drops whole tiers from the bottom (custom source, then colour) and never tier 1, physics, writing nothing rather than a partial record; the degrading is reported by the writer, the record and the reader, and badged; every field is assigned a tier in one table, exhaustive at compile time; four tests keep it from drifting; splitting tier 2 is queued (RQ-386)"
- `decisions.md` § "R-521 — Each recorded link carries its own registry-entry hash beside the build hash and the schema version: a mismatch refuses, naming exactly that link, and a match recreates across an unrelated schema change *(closes RQ-385; amends R-483)*"
- `decisions.md` § "R-522 — Whether tier 2 of the embedding splits is ruled after TASK-M7-44's capacity re-measure: RQ-386 is parked until then, and tier 2 stays whole meanwhile"
- `docs/design/principia_dd_image_embedding.md` § "2. Layout"
- `docs/design/principia_dd_image_embedding.md` § "6. What travels"
- `docs/design/principia_dd_image_embedding.md` § "7. Measured capacity and behaviour"
- `docs/design/principia_dd_image_embedding.md` § "9. Obligations"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"

## Deliverables
- `crates/engine/src/contract/tiers.rs` and `TIERS`.
- The tiered writer and the flags in `crates/render/src/embed/`; the version-4 layout.
- The old-record reading; the re-measured §7 table, in the PR and in a docs edit through the porting rule.
- Tests `tier_table` (with its compile-fail arm), `embed_tiers`, `embed_tier_flags`, `embed_old_records`; the capacity measurement.

## Acceptance tests
- `cargo test -p render embed_old_records` — REQ-TOOL-173's verify (C12).
- Capacity measurement on the Mac (R-186) — REQ-TOOL-174's verify; §7's table rewritten.
- `cargo test -p render embed_tiers` — REQ-TOOL-179's verify (T3, with its mutation arm).
- `cargo test -p render embed_tier_flags` — REQ-TOOL-180's verify.
- `cargo test -p engine tier_table` — REQ-TOOL-181's verify (T4).

## Notes
- **⚑ Physics review is required** (the instruction's "How to work this", 4): this task does not merge on the code reviewer's word alone; its reviewer list includes physics. It covers that tier 1 holds everything that decides which systems are integrated and how (RT2, J4).
- qa files that must change, in this task's qa commit (R-290, forced by R-483 H5 and R-484 J5): `crates/render/tests/qa_TASK-M7-27.rs` (`DOC_VERSION` at :19 and :20; the reserved-flags loop at :341–348, bit 3 now defined). The PR lists each `M` line with its reason.
- The version byte is REQ-TOOL-109's calibration, confirmed at the M7 gate.
- **R-522 (20 Oct 2026):** RQ-386 (whether tier 2 splits) is parked until this task's capacity re-measure. The PR reports tier 2's own size and, at 64² and 128², whether tiers 1 and 2 fit for the default stain and a typical edited one (REQ-TOOL-174); the orchestrator then returns RQ-386 to the human with those numbers. Meanwhile tier 2 is whole. *Was:* "Splitting tier 2 is RQ-386; meanwhile tier 2 is whole."
- **R-521:** tier 1 carries the four entry hashes beside the build hash and the schema version; the measurement includes them (TASK-M7-45, a dependency; applied per R-369).
