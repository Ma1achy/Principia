# TASK-M8-05 — Explore shell: top bar, footer, theme, and the figure left uncovered (01_main.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-070, REQ-GUI-074, REQ-GUI-078, REQ-GUI-093, REQ-GUI-180, REQ-CHART-054, REQ-CHART-056
- **Depends on:** TASK-M8-03, TASK-M8-04, TASK-M6-20, TASK-M7-21, TASK-M6-24, TASK-M6-30
- **Needs (earlier milestones):** REQ-TOOL-050, REQ-TOOL-057, REQ-GUI-010, REQ-RENDER-069
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~700 lines

## Goal
The Explore page's frame exists over the wgpu render: egui-wgpu is built from the engine's device and queue and paints on the same surface; dark egui theme with Ubuntu / Ubuntu Mono; the top bar with the File / View / Windows / Help menus, the Explore / Stain switch, the Overlays ▾ / Run… / Profiler… / Export… entry points, the keyboard breadcrumb slot, and the status line (t, fps, frame ms, quad count, `budget-bound`, undo / redo depth, "F3 hide"); the footer with warning and error counts, the latest message, memory and "? keys". Nothing is drawn over the figure: warnings go to the footer. With F3 hiding the layer the figure fills the window, showing more of the field at the same scale (R-406); past the chart's `[0,1]²` each axis extends by the extension type it declares (affine, periodic, pole-crossing or bounded, the default), and a pixel whose `Φ` fails or whose state fails validation is hatched as forbidden, labelled `decode_failed` and never integrated (R-407). Area statistics over the window count each system once, through the axis types' primary ranges; a pixel hatched where the domain ends leaves both the count and the total, and only validity failures are forbidden (R-408).

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "Rules that hold everywhere"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `decisions.md` § "R-406 — With the egui layer hidden by F3, the figure fills the window, showing more of the field at the same scale rather than stretching; showing the layer returns the layout"
- `decisions.md` § "R-407 — Past `[0,1]²` each chart axis extends by the type it declares: affine, periodic, pole-crossing or bounded, the default; a pixel that fails is hatched as forbidden *(closes RQ-262)*"
- `decisions.md` § "R-408 — Area statistics count each system once, through the axis types: a visible pixel counts only if every axis is inside its primary range; domain-hatched pixels leave the count and the total; only validity failures are forbidden *(closes RQ-263)*"
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
  simplex affine in R-407's sense, its map bilinear; a Chart-builder physical-quantity axis mapped linearly onto its
  range, one of §G7's non-periodic quantities (energy, `L_z`, virial ratio, mass ratio), affine, and one behind a
  nonlinear warp declaring nothing, so bounded, R-408's port, B1). The pixel's validity check is inverse_encode layers 2
  and 3 only (layer 1 is the encode path's). Tests `chart_extension`, `f3_fill_labels`; screenshot cases
  `01_main/f3_off_sphere`, `01_main/f3_off_invariant`.
- The area statistics' counting (REQ-CHART-056; chart_decoder_contract Part 3, R-408): each type's primary range on the
  `Chart` trait beside its type, and a classifier in `crates/kernel/src/chart/` that, for a visible pixel, gives the
  domain's end, not counted, forbidden or a counted system, in chart_decoder_contract Part 3's order, reading the chart
  (types, primary ranges, `validate`, `Φ`), not the payload label. TASK-M8-18's `forbidden_fraction` uses it. Test
  `area_stats_primary_range`.

## Acceptance tests
- `cargo xtask screenshot 01_main` (F3 on / off cases) and Review checklist (gui reviewer) on the egui-wgpu construction — screenshots with F3 on and off against 01_main.png: the figure is identical underneath, over the shown figure's rect, with a stain that reads no screen-lane field (R-406, R-407 A4); review that egui-wgpu is constructed from the engine's device/queue, not a second context (REQ-GUI-070).
- `cargo xtask screenshot 01_main` — screenshot against 01_main.png: dark theme, Ubuntu for text, Ubuntu Mono for numbers/code (REQ-GUI-075). Closed by TASK-M6-24 on the mock engine since R-390; this task re-runs it on the real engine.
- `cargo xtask screenshot 01_main` (f3_off, f3_off_sphere, f3_off_invariant) on the real engine and `cargo test -p gui f3_fill_labels` — with a stain that reads no screen-lane field, on the latent chart, the shape sphere's spherical map and an invariant chart: the F3-off capture is pixel-identical to the F3-on one over the shown figure's rect; past `[0,1]²` the latent chart continues its formula, the sphere wraps in θ and crosses its poles with θ shifted by π, and the invariant chart is hatched on every side; every window pixel carries a labelled output, each hatched one `decode_failed`, none dropped; F3 again restores the shown capture; the undo depth is unchanged and no SetField is sent; controls: the shown view stretched to the window fails the check over the rect, and a window pixel left unlabelled fails the label check (REQ-GUI-180, R-407).
- `cargo test -p kernel chart_extension` and Review checklist (physics reviewer) — per chart, samples past each edge of `[0,1]²`: the latent chart's z continues `z₀ + (2s−1)q₁ + (2t−1)q₂`; the sphere's `s` and `s + 1` decode equal, and `t = 1 + δ` decodes as `t = 1 − δ` with θ + π (likewise below `t = 0`); the invariant warp is hatched past every edge; the mass simplex continues and its negative-mass pixels are hatched by layer 2's narrowed simplex (buffered `mᵢ ≥ ε_m`, exactly raw `mᵢ ≥ 0`), at every sample past the edge, including one within `ε_m` of it, so the hatch starts at the edge with no band; control: a check of buffered `mᵢ > 0` alone (which passes the band) fails; a custom chart's undeclared axis is hatched past its edge; every hatched pixel is `decode_failed` and none is integrated; inside `[0,1]²` every chart's output is bit-identical to before; controls: a periodic axis built as affine, a pole-crossing axis without the half-period shift, and an undeclared axis that continues each fail; the physics reviewer checks the types and the fallback against R-407 (REQ-CHART-054). Run with both of the
  shape sphere's hemispheres drawn, R-14's full `φ = π·(1 − t)` span; a custom chart's physical-quantity axis continues
  linearly and its unreachable pixels are hatched by `validate`, with a physical-quantity axis built as bounded as a
  further control (R-408's port, B1).
- `cargo test -p kernel area_stats_primary_range` and Review checklist (physics reviewer) — on the shape sphere's
  spherical map (both hemispheres), a window covering `[0,1]²` and reaching past it in `s` (a periodic duplicate) and
  in `t` (past a pole, θ shifted by π) gives the same forbidden count and the same total as `[0,1]²` alone, so each
  periodic duplicate is counted once; on the invariant warp, a window reaching past an edge gives `[0,1]²`'s count and
  total, the domain-hatched band leaving both; on the mass simplex, the window's negative-mass pixels count as forbidden
  and the forbidden share rises (R-408, B2); a custom chart's unreachable physical-quantity pixels count as forbidden;
  B5's order, on a test chart where no shipped chart has the case: a pixel where layer 2 rejects and `Φ` fails
  (non-finite) counts as forbidden, and a pixel where `Φ` fails and layer 2 accepts is the domain's end, out of the
  count and the total; inside `[0,1]²` every chart's statistic is bit-identical to before; controls: counting the
  periodic duplicate, counting a pixel past a pole, counting the domain-hatched band as forbidden, dropping it from the
  count but not the total, and a classifier running `Φ` before `validate` each fail; the physics reviewer checks the primary ranges and the classification order against R-408
  (REQ-CHART-056).
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
  types are TASK-M2-11's definition (REQ-CHART-055); a Burrau axis without one is bounded. Size: ~600 lines with the
  extension types and their tests (applied per R-369).
- R-408 (10 Oct 2026) rules RQ-263: area statistics count each system once, through the axis types' primary ranges.
  This task closes REQ-CHART-056 (new): the primary ranges and the classifier, with the `area_stats_primary_range`
  line; the physics reviewer reviews it with the extension types. A Chart-builder physical-quantity axis is affine
  (applied per R-369, R-408's port, B1, a correction to R-407's applied text, flagged to the human), checked in
  `chart_extension`. Every line runs with both of the shape sphere's hemispheres; the one-hemisphere case (RQ-264) is
  TASK-M8-44's, a leaf that depends on this task, so this task has no open RQ (applied per R-369, code review
  5478800754, F4). Size: ~700 lines with the counting and its tests (applied per R-369).
- Round-2 review fixes, applied per R-369: the REQ-GUI-070 line reads "over the shown figure's rect, with a stain that
  reads no screen-lane field" (gui review 5478798782, G5); `area_stats_primary_range` checks B5's order (physics review
  5478805963, F1); the mass simplex is sampled past the `ε_m` buffer (F5); the physical-quantity axis carries B1's
  guard (F3).
