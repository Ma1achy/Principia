# TASK-M8-33 — Import picture, saved views, and record a time sweep (09_importrecord.png)

- **Milestone:** M8
- **Closes:** REQ-TOOL-104, REQ-TOOL-105, REQ-TOOL-106, REQ-TOOL-107, REQ-TOOL-170, REQ-TOOL-207
- **Depends on:** TASK-M8-32, TASK-M8-08, TASK-M6-48, TASK-M6-49, TASK-M8-81
- **Needs (earlier milestones):** REQ-TOOL-061, REQ-TOOL-063, REQ-TOOL-066, REQ-TOOL-071, REQ-SYS-011
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-1
- **Size:** ~450 lines

## Goal
Import picture (drop a Principia PNG or File › Import picture…) reads the pxpack, shows what is stored and what differs from now, and offers restore this view, only its stain, and open side by side. Saved views are pxpack snapshots with a thumbnail and "go", plus "Save this view", kept in the store §G9 defines (TASK-M8-08). Record a time sweep takes a time range, a frame count, the per-frame quality, a size at the view's aspect, a format (natively PNG frames, GIF, and MP4 through a system ffmpeg when present; in the browser zipped PNG frames, GIF via a wasm encoder, and MP4/WebM through WebCodecs where supported, R-131), overlays on or off, with the record in every file (once in a GIF's, an MP4's or a WebM's container, through TASK-M8-81's carrier, always; in each PNG frame its own, at its frame time) and a `.principia-sweep.json` beside it (R-584), and integrates each frame to its own t through the blocking exporter — never by scrubbing.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "09 Import picture · saved views · record a sweep"
- `docs/gui/principia_render_gui_spec.md` § "G9. Import picture · saved views · record a sweep (`09_importrecord.png`)"
- `docs/contracts/principia_export_animation_contract.md` § "Part 4 — Export: the frame loop in blocking mode"
- `decisions.md` § "R-131 — The video encoders *(closes RQ-108)*"

## Deliverables
- `crates/gui/src/windows/import_picture.rs`, `saved_views.rs`, `record_sweep.rs`.
- Golden suite `record-sweep` (a recorded frame at t equals a fresh blocking render at t).
- Tests: `import_diff_stain_only`, `saved_view_go`, `record_frames_pxpack`, `sweep_settings_file`; screenshot case `09_importrecord/layout`.

## Acceptance tests
- `cargo test -p gui import_diff_stain_only` and `cargo xtask screenshot 09_importrecord` — import a PNG exported with a different stain: the diff lists the stain; 'only its stain' changes RenderState's stain and nothing else; screenshot against 09_importrecord.png (REQ-TOOL-104).
- `cargo test -p gui saved_view_go` — save a view, change it, press go: the state equals the saved one (REQ-TOOL-105).
- `cargo test -p engine record_frames_pxpack` — record 5 frames as PNG frames: each carries a pxpack whose time is its own frame time; record the same 5 frames as a GIF and as an MP4: each file carries the record once in its container, read back bitwise equal to the .principia-sweep.json beside it, its frames' low bits not read; the dialog offers no switch that skips the record (R-584); size aspect equals the view's; natively MP4 is offered only when a system ffmpeg is found; in the browser MP4/WebM is offered only where WebCodecs supports it (R-131) (controls: a Record dialog with a switch that skips the record, a GIF or MP4 record read from the frames' low bits, and a PNG-frames recording that follows the single-figure default, low bits off, each fail) (REQ-TOOL-106).
- `cargo xtask golden record-sweep` — a recorded frame at t equals a fresh blocking render at t pixel for pixel (REQ-TOOL-107).
- `cargo test -p gui saved_view_tilt_map` — REQ-TOOL-170's verify.
- `cargo test -p gui sweep_settings_file` — a sweep recorded as PNG frames, as a GIF and as an MP4 leaves a .principia-sweep.json beside each, bitwise the JSON its container's record holds, and each PNG frame's own record carries the same sim and render; with n = 11 from t = 0 to 1 frame i's recorded playhead equals (1 − 0) × i / 10 bitwise (0.3 at i = 3); with n = 11 from t = 0.1 to 0.7 frame 9's recorded playhead is 0.1 + (0.7 − 0.1) × 9 / 10 in that order, 0.6399999999999999; and a one-frame sweep is at t_start (controls: a PNG-frames export with no file beside it, a playhead computed as i × ((t_end − t_start) / (n − 1)), which gives 0.30000000000000004 at i = 3, one computed as t_start + (t_end − t_start) × (i / (n − 1)), and one computed as t_start × (1 − i / (n − 1)) + t_end × (i / (n − 1)), the last two giving 0.64 at i = 9, each fail) (REQ-TOOL-207).

## Notes
- "Open side by side" opens the v2 side-by-side view of TASK-M8-36 once it lands; until then the action is asserted to request it.
- RQ-108 ruled: R-131 — the formats offered per build (TASK-M8-30 builds the sinks).
- **R-474, R-475 (20 Oct 2026):** a saved view reopens under its tilt map, a pre-R-474 one under Lean; a saved view on a saved chart carries its definition.
- R-483 X4, X5, R-484 RT3 (20 Oct 2026): Import picture's new outcomes (cannot recreate, links unrecorded, tiers recovered) are reported by TASK-M8-53 (REQ-GUI-263).
- **R-584 (21 Oct 2026, gap sweep PR 2c):** a recording carries the record once per file in its container, always (TASK-M8-81, on which this task now depends), and the settings file beside it; REQ-TOOL-207 (the settings file and the playhead rule, applied per R-369) is closed here. *Was (goal):* "overlays on or off, with pxpack in every frame, and". *Was:* "`cargo test -p engine record_frames_pxpack` — record 5 frames as PNG frames: each carries a pxpack whose time is its own frame time; size aspect equals the view's; natively MP4 is offered only when a system ffmpeg is found; in the browser MP4/WebM is offered only where WebCodecs supports it (R-131) (REQ-TOOL-106)."; "- **Closes:** REQ-TOOL-104, REQ-TOOL-105, REQ-TOOL-106, REQ-TOOL-107, REQ-TOOL-170"; "- Tests: `import_diff_stain_only`, `saved_view_go`, `record_frames_pxpack`; screenshot case `09_importrecord/layout`."
- **Fix round 2 of PR #287 (qa review 5638084688, Q5, Q7, applied per R-369):** the acceptance lines follow the new verifies. *Was:* "`cargo test -p engine record_frames_pxpack` — record 5 frames as PNG frames: each carries a pxpack whose time is its own frame time; record the same 5 frames as a GIF and as an MP4: each file carries the record once in its container, read back bitwise equal to the .principia-sweep.json beside it, its frames' low bits not read; the dialog offers no switch that skips the record (R-584); size aspect equals the view's; natively MP4 is offered only when a system ffmpeg is found; in the browser MP4/WebM is offered only where WebCodecs supports it (R-131) (REQ-TOOL-106)."; "`cargo test -p gui sweep_settings_file` — a sweep recorded as PNG frames, as a GIF and as an MP4 leaves a .principia-sweep.json beside each, bitwise the JSON its container's record holds, and each PNG frame's own record carries the same sim and render; with n = 11 from t = 0 to 1 frame i's recorded playhead equals (1 − 0) × i / 10 bitwise (0.3 at i = 3), and a one-frame sweep is at t_start (controls: a PNG-frames export with no file beside it, and a playhead computed as i × ((t_end − t_start) / (n − 1)), which gives 0.30000000000000004 at i = 3, fail) (REQ-TOOL-207)."
