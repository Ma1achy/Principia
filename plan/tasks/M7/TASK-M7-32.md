# TASK-M7-32 — Image embedding in PNG export: defaults, the tEXt chunk, capacity and the resolution round trip

- **Milestone:** M7
- **Closes:** REQ-TOOL-070, REQ-TOOL-072, REQ-TOOL-069, REQ-TOOL-113, REQ-VAL-098
- **Depends on:** TASK-M7-29, TASK-M7-31
- **Needs (earlier milestones):** REQ-PERF-032
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~400 lines

## Goal
PNG export with provenance: pixel embedding off by default for figure export (pixels bit-for-bit the renderer's output) and on by default for share export, a tEXt chunk carrying the same payload alongside; in-pixel embedding carries the full slice, sim, colour mode and ramp window (plus a custom shader's source from 128² up) at max pixel delta 1/255; from 128² an image carries config plus full shader; the supported resolutions are proposed (R-71) and the embed → PNG → extract round trip is bitwise-equal at each.

## References
- `docs/design/principia_dd_image_embedding.md` § "9. Obligations"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_dd_image_embedding.md` § "7. Measured capacity and behaviour"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/contracts/principia_export_animation_contract.md` § "Part 6 — Sharing: the spec is the object, the video is its shadow"

## Deliverables
- `crates/render/src/embed/png.rs` — figure vs share export defaults, the tEXt chunk.
- `fixtures/golden/embed-roundtrip/` and the `embed-roundtrip` golden suite; `fixtures/gates/embed-capacity/` and the `embed-capacity` gate.
- The R-71 proposal: the resolution list with §7's capacity measurements.

## Acceptance tests
- `cargo test -p render export_png_defaults` — figure export pixels equal renderer output bit-for-bit; share export has the LSB payload and a tEXt chunk (REQ-TOOL-070).
- `cargo test -p render embed_transforms` — embed, apply each transform (crop, rotation, flip, PNG re-encode, alpha strip), recover the config; max |Δ| per channel ≤ 1; the figure-export default has embedding off (REQ-TOOL-072).
- `cargo xtask gate embed-capacity` — capacity at 128² ≥ 1 tile of config + full shader (REQ-TOOL-069).
- Review (code, qa): the proposal cites §7's capacity measurements (64² to 1024²) and shows every listed resolution holds at least one intact record of the current payload; confirmed by the human at the M7 gate (REQ-TOOL-113).
- `cargo xtask golden embed-roundtrip` — embed → PNG round trip → extract yields a bitwise-equal config at every resolution of REQ-TOOL-113 (calibrated), the verify naming 64², 128², 256², 512², 1024² (REQ-VAL-098).

## Notes
- Calibration (R-71): REQ-TOOL-113.
- The Export & share window itself is M8; this task provides the export functions and their defaults.
