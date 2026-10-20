# TASK-M7-45 — The decoder's identity per link: each recorded link's own entry hash, refusing exactly the changed link (R-483 X4)

- **Milestone:** M7
- **Closes:** REQ-TOOL-172
- **Depends on:** TASK-M7-31
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~150 lines, the tests included

## Goal
A record whose link's registry entry differs from the writer's is refused, "state present, cannot recreate", naming exactly the changed link, by whichever means RQ-385 rules.

## References
- `decisions.md` § "R-483 — Export, sharing and the embedded record carry the decoder: `SimConfig`'s `Links` holds each block's link id and parameter values, the one place they live, written in full even at the defaults; the embedded record's list gains the links, the tilt fields, the energy-axis mappings and a saved chart by value; a record's decoder is the build's registry plus its links, an unknown or changed link refused and never substituted; an old record reads under its own build's defaults; capacity is re-measured and a record never truncated; a link change between keyframes is a cut; the sim key and the compatibility signature gain the parameter values; two questions queued (RQ-384, RQ-385)"
- `docs/design/principia_dd_image_embedding.md` § "6. What travels"

## Deliverables
- The per-link refusal as RQ-385 rules it.
- Test `decoder_identity_per_link`.

## Acceptance tests
- `cargo test -p engine decoder_identity_per_link` — REQ-TOOL-172's verify.

## Notes
- Waits on RQ-385 (a leaf): meanwhile REQ-TOOL-171's schema-version refusal stands. The recommendation is (b), a per-link entry hash in the sidecar.
- **⚑ Physics review is required** (the instruction's "How to work this", 4): this task does not merge on the code reviewer's word alone; its reviewer list includes physics.
