# TASK-M8-30 — Export: the frame loop in blocking mode, as a background job

- **Milestone:** M8
- **Closes:** REQ-SCHED-073, REQ-TOOL-077, REQ-TOOL-078, REQ-TOOL-079, REQ-TOOL-080, REQ-TOOL-073, REQ-TOOL-081, REQ-TOOL-082, REQ-TOOL-086
- **Depends on:** TASK-M8-02, TASK-M5-24
- **Needs (earlier milestones):** REQ-SCHED-004, REQ-SCHED-008, REQ-SCHED-043, REQ-SCHED-029, REQ-RENDER-029, REQ-RENDER-036, REQ-GUI-009, REQ-PERF-032, REQ-TOOL-046, REQ-EVT-012
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-1, PIT-1.7
- **Size:** ~500 lines

## Goal
Export runs the same frame loop as interactive playback with a hard barrier over all visible quads per captured frame: a frame is captured only when every visible quad is at the frame's t and refined to the export quality, with no moving fallback, blur or progressive fill, and never from a preview quad. Checkerboard and the other motion-time levers are forced off. The export quality may exceed the interactive tier with no motion gating, priced into the estimate. The exporter renders offscreen at the target resolution, ignores wall-clock pacing and speed, and runs as a background job (progress, ETA, live thumbnail, cancel) that streams frames to the encoder and never blocks the UI.

## References
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_temporal_architecture_note.md` § "The frame loop — the one genuinely new object (and what makes lockstep clean)"
- `docs/design/principia_temporal_architecture_note.md` § "Footguns (all re-applications of disciplines already established)"
- `docs/contracts/principia_export_animation_contract.md` § "Part 4 — Export: the frame loop in blocking mode"
- `docs/gui/principia_render_gui_spec.md` § "G9. Import picture · saved views · record a sweep (`09_importrecord.png`)"
- `docs/contracts/principia_scheduler_contract.md` § "Part 7 — The frame loop (lockstep presentation)"
- `docs/contracts/principia_checkerboard_contract.md` § "3. The three-state control"
- `docs/contracts/principia_checkerboard_contract.md` § "5. Self-erasing: catch-up at rest, endpoints, and pause"
- `docs/contracts/principia_export_animation_contract.md` § "Part 5 — Determinism & reproducibility"
- `docs/contracts/principia_scheduler_contract.md` § "Part 5 — Preview vs refine is a second sim key"
- `decisions.md` § "R-155 — One GIF encoder for both builds *(closes RQ-125)*"

- `decisions.md` § "R-131 — The video encoders *(closes RQ-108)*"
## Deliverables
- `crates/engine/src/export/{job,barrier,offscreen}.rs` — blocking mode, the job, streamed encode.
- `crates/engine/src/export/encoder.rs` — the encoder sink trait and the sinks R-131 names: natively PNG frames, GIF, and MP4 through a system ffmpeg when present; in the browser PNG frames (zipped), GIF via a wasm encoder, and MP4/WebM through WebCodecs where supported.
- Golden suites `export-barrier`, `export-checkerboard`; bench `export-memory`.
- Tests: `export_blocking`, `export_tier_estimate`, `export_forces_levers_off`, `export_resolution`, `export_ignores_speed`, `export_no_preview`.

## Acceptance tests
- `cargo test -p engine export_blocking` — every exported frame has all visible quads at the frame's t and refined; the UI thread stays responsive during export (REQ-SCHED-073).
- `cargo xtask golden export-barrier` — an exported frame equals a fully refined still render at the same t (REQ-TOOL-077).
- `cargo xtask bench export-memory` — exporting 600 frames keeps peak memory flat (independent of frame count); the UI stays responsive; cancel stops the job (REQ-TOOL-078).
- `cargo test -p engine export_tier_estimate` — a higher export tier raises the estimate; E is full during export (REQ-TOOL-079).
- `cargo test -p engine export_forces_levers_off` — checkerboard on in the UI: every exported frame computes all visible pixels at one t (REQ-TOOL-080).
- `cargo xtask golden export-checkerboard` — export with checkerboard permanent matches the export with it off, bit-for-bit (REQ-TOOL-073).
- `cargo test -p engine export_resolution` — export 3840×2160 from a 1280×720 window: output frames are 3840×2160 (REQ-TOOL-081).
- `cargo test -p engine export_ignores_speed` — exports with speed 0.5× and 4× produce identical files (REQ-TOOL-082).
- `cargo test -p engine export_no_preview` — run an export with only preview payloads cached; assert it dispatches full-T computations and reads no PREVIEW_MODE entry (REQ-TOOL-086).

## Notes
- RQ-108 ruled: R-131 — the encoders are named (native: PNG frames, GIF, MP4 through a system ffmpeg when present; browser: zipped PNG frames, GIF via a wasm encoder, MP4/WebM through WebCodecs where supported); the sink trait keeps them out of the frame loop.
- RQ-125 ruled: R-155 — one GIF encoder for both builds: the Rust `gif` crate (MIT or Apache-2.0), with `color_quant` for palettes, compiled to wasm for the browser.
