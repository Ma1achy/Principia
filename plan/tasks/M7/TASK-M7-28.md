# TASK-M7-28 — Image embedding: the tiled writer, majority-vote reader and the three outcomes

- **Milestone:** M7
- **Closes:** REQ-TOOL-059, REQ-TOOL-061, REQ-TOOL-071
- **Depends on:** TASK-M7-27
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
The tiled variant: the payload written in the low bit of every RGB channel as whole records in a 2-D square tile grid, the alpha low bit carrying a second systematic copy; the reader collects every intact record and majority-votes per byte, and reports exactly one of three outcomes — no embedded state, state present but corrupt, or recovered with how.

## References
- `docs/design/principia_dd_image_embedding.md` § "2. Layout"
- `docs/design/principia_dd_image_embedding.md` § "3. Tiles, not raster order"
- `docs/design/principia_dd_image_embedding.md` § "9. Obligations"
- `docs/design/principia_dd_image_embedding.md` § "4. The three searches on read"

## Deliverables
- `crates/render/src/embed/{writer,reader}.rs`.
- `crates/render/tests/embed_tiled.rs`.

## Acceptance tests
- `cargo test -p render embed_rgb_alpha` — embed then read with alpha stripped: RGB alone recovers the payload (REQ-TOOL-059).
- `cargo test -p render embed_majority_vote` — with a minority of records carrying consistent corrupted-but-CRC-passing bytes, the majority wins (REQ-TOOL-061).
- `cargo test -p render embed_outcomes` — plain PNG → none; all records corrupted → corrupt; valid → recovered + transform report (REQ-TOOL-071).

## Notes
- none
