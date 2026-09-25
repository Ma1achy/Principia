# TASK-M7-29 — Image embedding: the three read searches and the survives-list suite

- **Milestone:** M7
- **Closes:** REQ-TOOL-063, REQ-TOOL-068
- **Depends on:** TASK-M7-28
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
The reader searches tile offset (dx, dy) over one tile, the 8 dihedral transforms and nearest-neighbour decimation factors, with the CRC as the oracle, and reports what it found {transform, decimate, tiles}. The embedding survives dd_image_embedding §7's list at max pixel delta 1: PNG save/load (optimize, compress_level 9), alpha stripping, crops (50 %, 37 px offset), rotations 90/180/270, flips, transpose, nearest upscale ×2/×3 (also with rot90) and overwrite of the top 50 %.

## References
- `docs/design/principia_dd_image_embedding.md` § "4. The three searches on read"
- `docs/design/principia_dd_image_embedding.md` § "7. Measured capacity and behaviour"

## Deliverables
- `crates/render/src/embed/search.rs`.
- `crates/render/tests/embed_survives.rs` — the §7 "Survives" list at 512².

## Acceptance tests
- `cargo test -p render embed_searches` — recover after a 37 px crop, rot270 and ×2 upscale; the report gives {transform, decimate, tiles} (REQ-TOOL-063).
- `cargo test -p render embed_survives` (proptest) — the §7 "Survives" list as a test suite at 512², max pixel delta 1 (REQ-TOOL-068).

## Notes
- none
