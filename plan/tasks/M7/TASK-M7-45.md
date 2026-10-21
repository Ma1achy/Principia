# TASK-M7-45 — The decoder's identity per link: each recorded link's own entry hash, refusing exactly the changed link (R-483 X4)

- **Milestone:** M7
- **Closes:** REQ-TOOL-172, REQ-TOOL-186
- **Depends on:** TASK-M7-31, TASK-M7-32
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

## Deliverables
- The generated entry hashes (dd_generation_root §3.9, "Each entry's own hash"), written per block beside the build hash and the schema version in the M7 destinations (tier 1, the `tEXt` chunk, the profiler header; *was:* "in every destination"), and the per-link refusal (R-521).
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
