# TASK-M7-27 — Image embedding: the record format, header and tile geometry

- **Milestone:** M7
- **Closes:** REQ-TOOL-110, REQ-TOOL-109, REQ-TOOL-118, REQ-TOOL-060, REQ-TOOL-062
- **Depends on:** TASK-M0-01
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
The embedded record — header(16 B: magic 4, version 1, flags 1, payload_len 4, n_records 2, crc32(header) 4) ‖ payload ‖ crc32(payload), trusted or discarded whole — with its flag bits, byte order, bit order within a tile, payload serialisation and tEXt keyword defined in dd_image_embedding §2 and §9 (R-72), the magic bytes and version byte proposed (R-71), the version bumped above the prototype's for R-81's contract-name layout, and the tile side derived as ceil(sqrt(ceil(record_bits / 3))).

## References
- `docs/design/principia_dd_image_embedding.md` § "2. Layout"
- `docs/design/principia_dd_image_embedding.md` § "3. Tiles, not raster order"
- `docs/design/principia_dd_image_embedding.md` § "7. Measured capacity and behaviour"
- `docs/design/principia_dd_image_embedding.md` § "9. Obligations"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_image_embedding.md` § "6. What travels"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-81 — The embedded record uses the contract names *(closes RQ-32)*"

## Deliverables
- `docs/design/principia_dd_image_embedding.md` §2 and §9 — the definitions (flag bits, byte order, bit order within a tile, payload serialisation, tEXt keyword), with the "Removed lines" note.
- `crates/render/src/embed/record.rs` — record encode/decode and tile geometry, reader and writer generated from one format description.
- The R-71 proposal: the four magic bytes (checked against PNG signatures and the prototype's) and the version byte.

## Acceptance tests
- Review (physics): dd_image_embedding states each item; the serialisation reproduces §7's measured payload size class; reader and writer are generated from one description (REQ-TOOL-110).
- Review (code, qa): the proposal gives the four magic bytes (not colliding with PNG signatures or the prototype's) and a version byte above the prototype's; confirmed by the human at the M7 gate (REQ-TOOL-109).
- `cargo test -p render embed_record_version` — the header version of a newly embedded record differs from the prototype layout's version (REQ-TOOL-118).
- `cargo test -p render embed_record_crc` — corrupt one byte in a record: that record is discarded, the others decode (REQ-TOOL-060).
- `cargo test -p render embed_tile_side` — the tile side for the measured 382 B record is ceil(sqrt(ceil(record_bits / 3))) (REQ-TOOL-062).

## Notes
- Definition (R-72) REQ-TOOL-110: the physics reviewer approves the doc change before merge.
- Calibration (R-71) REQ-TOOL-109.
- Crate placement: `crates/render/src/embed/` (the plan layout names no export crate).
