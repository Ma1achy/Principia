# TASK-M8-05 — Explore shell: top bar, footer, theme, and the figure left uncovered (01_main.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-070, REQ-GUI-074, REQ-GUI-078, REQ-GUI-093, REQ-GUI-180, REQ-CHART-054
- **Depends on:** TASK-M8-03, TASK-M8-04, TASK-M6-20, TASK-M7-21, TASK-M6-24, TASK-M6-30
- **Needs (earlier milestones):** REQ-TOOL-050, REQ-TOOL-057, REQ-GUI-010, REQ-RENDER-069
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~600 lines

## Goal
The Explore page's frame exists over the wgpu render: egui-wgpu is built from the engine's device and queue and paints on the same surface; dark egui theme with Ubuntu / Ubuntu Mono; the top bar with the File / View / Windows / Help menus, the Explore / Stain switch, the Overlays ▾ / Run… / Profiler… / Export… entry points, the keyboard breadcrumb slot, and the status line (t, fps, frame ms, quad count, `budget-bound`, undo / redo depth, "F3 hide"); the footer with warning and error counts, the latest message, memory and "? keys". Nothing is drawn over the figure: warnings go to the footer. With F3 hiding the layer the figure fills the window, showing more of the field at the same scale (R-406); past the chart's `[0,1]²` each axis extends by the extension type it declares (affine, periodic, pole-crossing or bounded, the default), and a pixel whose `Φ` fails or whose state fails validation is hatched as forbidden, labelled `decode_failed` and never integrated (R-407).

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "Rules that hold everywhere"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `decisions.md` § "R-406 — With the egui layer hidden by F3, the figure fills the window, showing more of the field at the same scale rather than stretching; showing the layer returns the layout"
- `decisions.md` § "R-407 — Past `[0,1]²` each chart axis extends by the type it declares: affine, periodic, pole-crossing or bounded, the default; a pixel that fails is hatched as forbidden *(closes RQ-262)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Past the unit square — each axis's extension type (R-407)"
- `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)"
- `docs/design/principia_coordinate_conventions_note.md` § "The three coordinate spaces (they nest; each is right for its job)"

## Deliverables
- `crates/gui/src/render_host.rs` — egui-wgpu `Renderer` constructed from the engine's `wgpu::Device` / `Queue`, painting after the engine's pass on the same surface texture.
- Uses the track's `crates/gui/src/theme.rs` — egui dark default + Ubuntu / Ubuntu Mono (fonts under `crates/gui/assets/fonts/`) (TASK-M6-24, R-390) — on the real engine.
- `crates/gui/src/explore/{top_bar,footer,figure}.rs`, extending the track's top bar and footer (TASK-M6-24, R-390) — the status line reads the real engine's Snapshot (frame record, binding axis, undo depth); the figure region admits only the hover label and the lock reticle as marks.
- `crates/gui/src/windows/mod.rs` — the window registry the Windows menu lists (entries filled by later tasks).
- `xtask` screenshot cases `01_main/shell`, `01_main/f3_off`, `01_main/warning`, `01_main/budget_bound`.
- The figure filling the window with F3 off (R-406): the canvas given the shown layout's placement of the field, UV
  continuing past the shown figure's rect at the same scale (the coordinate note, A4 of R-407), and the area past the
  chart's `[0,1]²` drawn by each axis's extension type with the hatch fallback (R-407). No quad request set is fixed
  beyond what R-407 implies: a hatched pixel needs no integration.
- The axis extension types (REQ-CHART-054; chart_decoder_contract Part 3, chart_reference §5.4): each axis's declared
  type on the `Chart` trait (`crates/kernel/src/chart/`), bounded the default; the types acting before `Φ` and
  `validate`; the fallback hatching, labelled `decode_failed`; the existing charts' types (latent and flat affine, the
  sphere's spherical map θ periodic and φ pole-crossing, its exponential map and the invariant warp bounded, the mass
  simplex affine). Tests `chart_extension`, `f3_fill_labels`; screenshot cases `01_main/f3_off_sphere`,
  `01_main/f3_off_invariant`.

## Acceptance tests
- `cargo xtask screenshot 01_main` (F3 on / off cases) and Review checklist (gui reviewer) on the egui-wgpu construction — screenshots with F3 on and off against 01_main.png: the figure is identical underneath; review that egui-wgpu is constructed from the engine's device/queue, not a second context (REQ-GUI-070).
- `cargo xtask screenshot 01_main` — screenshot against 01_main.png: dark theme, Ubuntu for text, Ubuntu Mono for numbers/code (REQ-GUI-075). Closed by TASK-M6-24 on the mock engine since R-390; this task re-runs it on the real engine.
- `cargo xtask screenshot 01_main` (f3_off, f3_off_sphere, f3_off_invariant) on the real engine and `cargo test -p gui f3_fill_labels` — with a stain that reads no screen-lane field, on the latent chart, the shape sphere's spherical map and an invariant chart: the F3-off capture is pixel-identical to the F3-on one over the shown figure's rect; past `[0,1]²` the latent chart continues its formula, the sphere wraps in θ and crosses its poles with θ shifted by π, and the invariant chart is hatched on every side; every window pixel carries a labelled output, each hatched one `decode_failed`, none dropped; F3 again restores the shown capture; the undo depth is unchanged and no SetField is sent; controls: the shown view stretched to the window fails the check over the rect, and a window pixel left unlabelled fails the label check (REQ-GUI-180, R-407).
- `cargo test -p kernel chart_extension` and Review checklist (physics reviewer) — per chart, samples past each edge of `[0,1]²`: the latent chart's z continues `z₀ + (2s−1)q₁ + (2t−1)q₂`; the sphere's `s` and `s + 1` decode equal, and `t = 1 + δ` decodes as `t = 1 − δ` with θ + π (likewise below `t = 0`); the invariant warp is hatched past every edge; the mass simplex continues and its negative-mass pixels are hatched; a custom chart's undeclared axis is hatched past its edge; every hatched pixel is `decode_failed` and none is integrated; inside `[0,1]²` every chart's output is bit-identical to before; controls: a periodic axis built as affine, a pole-crossing axis without the half-period shift, and an undeclared axis that continues each fail; the physics reviewer checks the types and the fallback against R-407 (REQ-CHART-054).
- `cargo xtask screenshot 01_main` (warning case) — trigger a warning and an error with the figure visible: screenshot against 01_main.png shows nothing new over the plot; the footer count increments (REQ-GUI-074).
- `cargo xtask screenshot 01_main` (top bar, budget-bound case) and `cargo test -p gui status_undo_depth` — screenshot against 01_main.png's top bar; force the budget to bind and check 'budget-bound' appears; make two edits and check the undo depth reads 2 (REQ-GUI-078).
- `cargo xtask screenshot 01_main` (footer) and `cargo test -p gui footer_opens_console` — screenshot against 01_main.png's footer; a click opens the 12_console.png layout (REQ-GUI-093).

## Notes
- The footer click opens the console window; its layout (12_console.png) is TASK-M8-27's. Until then the test asserts the console window id is requested.
- R-390: TASK-M6-24 builds the shell on the mock engine and closes REQ-GUI-075; this task depends on it, wires the shell to the real engine (egui-wgpu on the engine's device and queue, the real status line, the real footer) and re-runs REQ-GUI-075's acceptance there. The console's layout is TASK-M6-28's on the mock (REQ-GUI-126) and TASK-M8-27's on the real engine.
- RQ-243, RQ-247 and RQ-255, decided per R-369 (7 Oct 2026): TASK-M6-24's contract gives the snapshot an optional
  frame summary (`frame_ms`, `fps`, `quad_count`, `live_memory { heap_bytes, gpu_bytes }`), `None` from the real
  engine until this task wires its frame loop to fill it; this task keeps REQ-GUI-078 (`budget-bound`, which the mock
  never binds) and may revise REQ-GUI-176's definition through the porting rule. The canvas is a separate engine-side
  trait in `engine::contract`, outside the data contract; the real engine's implementation of it (its device and
  queue, and drawing the figure into the app's pass) is this task's, for REQ-GUI-070. On the real engine the window
  title and the footer carry no "mock engine" tag.
- R-406 (10 Oct 2026): with F3 off the figure fills the window, each point of the field where the shown layout puts it,
  so REQ-GUI-070's "the figure is identical underneath" is checked over the shown figure's rect. TASK-M6-30 builds it
  on the mock.
- R-407 (10 Oct 2026) rules RQ-262, so nothing here waits: REQ-GUI-180 and the axis extension types (REQ-CHART-054)
  land in this task, as the ruling says. The physics reviewer, added under R-406, reviews the extension types and the
  fallback (applied per R-369, R-407, A1). `ctx.screen.uv` and `ctx.screen.pixel` are taken over the window's figure
  area as shown (the full window with F3 off), so a stain that reads them draws differently over the shown rect in the
  two states; REQ-GUI-180's pixel-identity check uses a stain that reads no screen-lane field (A4). The Burrau family's
  types are TASK-M2-11's definition (REQ-CHART-055); a Burrau axis without one is bounded. This task computes no area
  statistic over the extended window, so RQ-263 holds nothing here. Size: ~600 lines with the extension types and their
  tests (applied per R-369).
