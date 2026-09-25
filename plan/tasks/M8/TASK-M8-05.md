# TASK-M8-05 — Explore shell: top bar, footer, theme, and the figure left uncovered (01_main.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-070, REQ-GUI-075, REQ-GUI-074, REQ-GUI-078, REQ-GUI-093
- **Depends on:** TASK-M8-03, TASK-M8-04, TASK-M6-20, TASK-M7-21
- **Needs (earlier milestones):** REQ-TOOL-050, REQ-TOOL-057, REQ-GUI-010, REQ-RENDER-069
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The Explore page's frame exists over the wgpu render: egui-wgpu is built from the engine's device and queue and paints on the same surface; dark egui theme with Ubuntu / Ubuntu Mono; the top bar with the File / View / Windows / Help menus, the Explore / Stain switch, the Overlays ▾ / Run… / Profiler… / Export… entry points, the keyboard breadcrumb slot, and the status line (t, fps, frame ms, quad count, `budget-bound`, undo / redo depth, "F3 hide"); the footer with warning and error counts, the latest message, memory and "? keys". Nothing is drawn over the figure: warnings go to the footer.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "Rules that hold everywhere"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"

## Deliverables
- `crates/gui/src/render_host.rs` — egui-wgpu `Renderer` constructed from the engine's `wgpu::Device` / `Queue`, painting after the engine's pass on the same surface texture.
- `crates/gui/src/theme.rs` — egui dark default + Ubuntu / Ubuntu Mono (fonts under `crates/gui/assets/fonts/`).
- `crates/gui/src/explore/{top_bar,footer,figure}.rs` — the status line reads the Snapshot (frame record, binding axis, undo depth); the figure region admits only the hover label and the lock reticle as marks.
- `crates/gui/src/windows/mod.rs` — the window registry the Windows menu lists (entries filled by later tasks).
- `xtask` screenshot cases `01_main/shell`, `01_main/f3_off`, `01_main/warning`, `01_main/budget_bound`.

## Acceptance tests
- `cargo xtask screenshot 01_main` (F3 on / off cases) and Review checklist (gui reviewer) on the egui-wgpu construction — screenshots with F3 on and off against 01_main.png: the figure is identical underneath; review that egui-wgpu is constructed from the engine's device/queue, not a second context (REQ-GUI-070).
- `cargo xtask screenshot 01_main` — screenshot against 01_main.png: dark theme, Ubuntu for text, Ubuntu Mono for numbers/code (REQ-GUI-075).
- `cargo xtask screenshot 01_main` (warning case) — trigger a warning and an error with the figure visible: screenshot against 01_main.png shows nothing new over the plot; the footer count increments (REQ-GUI-074).
- `cargo xtask screenshot 01_main` (top bar, budget-bound case) and `cargo test -p gui status_undo_depth` — screenshot against 01_main.png's top bar; force the budget to bind and check 'budget-bound' appears; make two edits and check the undo depth reads 2 (REQ-GUI-078).
- `cargo xtask screenshot 01_main` (footer) and `cargo test -p gui footer_opens_console` — screenshot against 01_main.png's footer; a click opens the 12_console.png layout (REQ-GUI-093).

## Notes
- The footer click opens the console window; its layout (12_console.png) is TASK-M8-27's. Until then the test asserts the console window id is requested.
