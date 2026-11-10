# TASK-M7-45 — The decoder's identity per link: each recorded link's own entry hash, refusing exactly the changed link (R-483 X4)

- **Milestone:** M7
- **Closes:** REQ-TOOL-172, REQ-TOOL-186
- **Depends on:** TASK-M7-31, TASK-M7-32, TASK-M2-57
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~150 lines, the tests included

## Goal
Each recorded link carries its own registry-entry hash beside the build hash and the schema version, in the destinations built by M7, the embedded record's tier 1, the `tEXt` chunk and the profiler header (R-521; *was:* "in every destination"); a record whose link's entry hash differs from the reading build's is refused, "state present, cannot recreate", naming exactly the changed link, and one whose hashes all match recreates across an unrelated schema change, re-integrating rather than reusing a stored payload. *Was:* "by whichever means RQ-385 rules."

## References
- `decisions.md` § "R-483 — Export, sharing and the embedded record carry the decoder: `SimConfig`'s `Links` holds each block's link id and parameter values, the one place they live, written in full even at the defaults; the embedded record's list gains the links, the tilt fields, the energy-axis mappings and a saved chart by value; a record's decoder is the build's registry plus its links, an unknown or changed link refused and never substituted; an old record reads under its own build's defaults; capacity is re-measured and a record never truncated; a link change between keyframes is a cut; the sim key and the compatibility signature gain the parameter values; two questions queued (RQ-384, RQ-385)"
- `docs/design/principia_dd_image_embedding.md` § "6. What travels"
- `decisions.md` § "R-521 — Each recorded link carries its own registry-entry hash beside the build hash and the schema version: a mismatch refuses, naming exactly that link, and a match recreates across an unrelated schema change *(closes RQ-385; amends R-483)*"
- `docs/design/principia_dd_generation_root.md` § "3.9 The link registry (consolidated from chart contract Part 2.5)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `decisions.md` § "R-921 — RQ-755, a record made under a retired `β` link: option (c), the owner's own; records store latent coordinates only, so recreation decodes the record's stored latent coordinates through that exact retired link, kept for conversion, to the original physical starting state, bitwise, and runs from there, labelled "made under a retired link", its hashes checked as usual; nothing is converted on recreation; conversion into the periodic `β` only when the record is opened as an editable view, once, with a notice, as R-919 Q3 (a); a record made before TASK-M2-57 recreates bitwise after it lands, an acceptance line on REQ-TOOL-172 *(closes RQ-755; refines R-919)*"

## Deliverables
- The generated entry hashes (dd_generation_root §3.9, "Each entry's own hash"), written per block beside the build hash and the schema version in the M7 destinations (tier 1, the `tEXt` chunk, the profiler header; *was:* "in every destination"), and the per-link refusal (R-521).
- The recreate path for a record made under a retired `β` link (R-921): its stored latent coordinates decoded through that exact link, bitwise, labelled "made under a retired link", never converted on recreation; REQ-TOOL-172's retired-link arm.
- That arm's expected bits read from `fixtures/retired_beta_links/pre_m2_57.json`, every bit of which TASK-M2-57 writes
  through its baseline commit's decoder, before any link changes: the records' latent coordinates and links the
  fixture's, each pixel's recreated starting state equal bit for bit to the fixture's, and the converting reader's
  control run on the fixture's pixels where conversion changes a bit, for each link TASK-M2-57 does not list as not
  exercisable with its REVIEW_QUEUE entry (R-921; physics review 5836519618 of PR #469; physics review 5836599062 of
  PR #469, P3).
- Tests `decoder_identity_per_link` and `entry_hashes_written`.

## Acceptance tests
- `cargo test -p engine decoder_identity_per_link` — REQ-TOOL-172's verify.
- `cargo test -p engine entry_hashes_written` — REQ-TOOL-186's verify.

## Notes
- **R-521 (20 Oct 2026) rules RQ-385** ("per-link hash", option (b)): this task no longer waits. REQ-TOOL-171's schema-version refusal stays for a record with no entry hashes. TASK-M7-44 measures tier 1 with the hashes, so it depends on this task (applied per R-369). *Was:* "Waits on RQ-385 (a leaf): meanwhile REQ-TOOL-171's schema-version refusal stands. The recommendation is (b), a per-link entry hash in the sidecar."
- **⚑ Physics review is required** (the instruction's "How to work this", 4): this task does not merge on the code reviewer's word alone; its reviewer list includes physics.
- **Code review F1 of PR #265 (applied per R-369):** REQ-TOOL-186 is kept to the destinations M7 builds. The animation spec (TASK-M8-31), snapshot JSON and a share link (TASK-M8-32) carry the hashes where they are built, and REQ-TOOL-176 (TASK-M8-53) checks all six. Each entry's hash also covers the registry constants its decode reaches (`tanh_sat`, `ε_M01`, `δ_λ`, `ε_w`; physics review P2), and REQ-TOOL-172's verify has the constants' control.
- **Code review F8 of PR #265 (applied per R-369):** this task depends on TASK-M7-32, which builds the `tEXt` chunk that `entry_hashes_written` reads back (R-521 H5). *Was:* "- **Depends on:** TASK-M7-31".
- **Physics review P5–P7 of PR #265 (applied per R-369):** the refusal names the constants a mismatched link's hash covers as well as the link; REQ-TOOL-172's verify checks each constant's scope (`ε_M01` the mass entries only, `ε_w` every entry, `tanh_sat` never a sigmoid_* or softsign_* entry); the decoder's construction, the operators' evaluation and the emitter's rules stay with the build-hash component (REQ-TOOL-112).
- *9 Nov 2026 (R-919, RQ-754 Q1 (c); physics review 5834538186 of PR #465; applied per R-369):* latent `β` takes one periodic link, and `tanh_beta` stays only as a conversion source, not selectable (TASK-M2-57). REQ-TOOL-172's all-tanh arm keeps `tanh_beta`, read as the `β` link of a record written before TASK-M2-57 merged and checked against the conversion source's entry hash, which the registry keeps with its forward map: the arm still tests the refusal's lead computed from the record's links, `tanh_sat` in it when all four are tanh-bearing. The periodic link in its place would test that only if it is tanh-bearing, which TASK-M2-57 has not defined, so that reading is not taken. REQ-TOOL-172's note records it.
- *9 Nov 2026 (RQ-755):* whether a pre-R-919 record naming one of the three `β` conversion sources, every entry hash matching, recreates through the conversion source or is refused is RQ-755, open. The recreate arm for such a record waits on it; the refusal side above does not.
- *10 Nov 2026 (R-921, closes RQ-755):* the line above is settled. The owner ruled option (c): records store latent coordinates only (`principia_dd_image_embedding.md` § 6, tier 1; `principia_gui_state_contract.md` §2), so a pre-R-919 record naming one of the three `β` conversion sources, every entry hash matching, recreates by decoding its stored latent coordinates through that exact link, bitwise, labelled "made under a retired link", with no conversion on recreation; conversion into the periodic `β` is only on opening it as an editable view (R-919 Q3 (a)). REQ-TOOL-172's verify gains the arm "a record made before TASK-M2-57 recreates bitwise after it lands", run by `decoder_identity_per_link`; the recreate arm no longer waits. The Deliverables gain its line, nothing removed.
- *10 Nov 2026 (physics review 5836519618 of PR #469, P1 and P2; applied per R-369: the pre-M2-57 fixture):* the
  record writer is built in M7, after TASK-M2-57, so no real record can be made before TASK-M2-57; "a record made
  before TASK-M2-57" is read as one whose latent coordinates and links are what a pre-TASK-M2-57 build writes, with that
  build's decode as the expected states. They come from `fixtures/retired_beta_links/pre_m2_57.json`, written on
  TASK-M2-57's baseline commit, so the retained forward maps are checked against a pre-TASK-M2-57 decode, not against
  themselves. This task depends on TASK-M2-57 for it. The Deliverables gain the fixture's line; nothing removed.
  *Was (Depends on):* "TASK-M7-31, TASK-M7-32".
- *10 Nov 2026 (physics review 5836599062 of PR #469, P3; applied per R-369):* TASK-M2-57's search for pixels where
  conversion changes a bit is bounded; a link with none within its budget is listed there as not exercisable, with a
  REVIEW_QUEUE entry for the owner, and this task's converting-reader control is not run for it. The fixture's bits all
  come from TASK-M2-57's baseline commit's decoder, a rewrite included. *Was (Deliverables):* "which TASK-M2-57's
  baseline commit" "writes through the decoder before any link changes: the records' latent coordinates and links the
  fixture's, each" "pixel's recreated starting state equal bit for bit to the fixture's, and the converting reader's
  control run on the" "fixture's pixels where conversion changes a bit (R-921; physics review 5836519618 of PR #469)."
