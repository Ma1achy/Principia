# TASK-M6-28 — The GUI track (R-390), 5: the windows: Run, Chart builder, Export & share, Display, Profiler, Console, the Inspector frame

- **Milestone:** M6
- **Closes:** REQ-GUI-174, REQ-GUI-126
- **Depends on:** TASK-M6-27
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~650 lines

## Goal
On the mock engine, the windows look, feel and behave as their artboards and the design notes give them, their frames
and controls real and their content plausible data from the mock, each opened from its top-bar or menu entry and joining
the keyboard scope tree: Run (horizon, integrator, escape settings, quality, budget, recompute and cancel, under the
contract's names, no "tolerance" field); Chart builder (the axis kinds, presets, the domain preview and the quick
render, both square for a square viewport, Revert / Apply); Export & share (image with pxpack, snapshot JSON, share
link, present mode); Display (the fixed order of the last stages, the display overlays); Profiler (the window and its
tabs, with the mock's plausible trace); Console (the footer opened: severity, time, source and message columns,
filters, copy and clear, opening by itself on an error); and the Inspector window's frame (Inspect and Create & locate,
the three panes and the one timeline).

## References
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "03 Chart builder"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "05 Inspector — one IC, its trajectory, one timeline"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "12 Console"
- `docs/gui/principia_render_gui_spec.md` § "G5. Windows (`04_windows.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G7. Chart builder (`03_chartbuilder.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G12. Console (`12_console.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G13. Where the artboards are overridden"
- `decisions.md` § "R-68 — Artboard values are illustrative; corpus values win *(closes RQ-24)*"
- `decisions.md` § "R-129 ✱ — Where the surfaces with no artboard live *(closes RQ-105)*"
- `decisions.md` § "R-152 — A minimal Profiler window holds the Arbiter tab at M6 *(closes RQ-122)*"

## Deliverables
- `crates/gui/src/windows/{run,chart_builder,export,display,profiler,console,inspector}.rs`, registered in the window
  registry the Windows menu lists.
- The mock's content for them: plausible run settings, a chart's domain and quick render, a profiler trace in profiler
  schema v1's shape, console entries of each severity, an IC and its trajectory for the Inspector frame; any contract
  field a control needs, added as the corpus names it (R-390's "Contract fields"), with the conformance suite re-run on
  both engines.
- Screenshot cases `04_windows/mock_run`, `04_windows/mock_export`, `04_windows/mock_display`,
  `04_windows/mock_profiler`, `03_chartbuilder/mock_chart_builder`, `12_console/mock_console`,
  `05_inspectors/mock_inspector`; test `mock_windows_open`.

## Acceptance tests
- `cargo xtask screenshot 04_windows` (mock_run, mock_export, mock_display, mock_profiler), `cargo xtask screenshot 03_chartbuilder` (mock_chart_builder), `cargo xtask screenshot 12_console` (mock_console), `cargo xtask screenshot 05_inspectors` (mock_inspector) and `cargo test -p gui mock_windows_open` — against their artboards; the Chart builder's two previews are square for a square viewport; each window opens from its top-bar or menu entry (REQ-GUI-174).
- `cargo xtask screenshot 12_console` (mock_console) — screenshot against 12_console.png; raising an error opens it (REQ-GUI-126).
- `cargo run -p gui --features mock`, `cargo test -p engine conformance` and `cargo test -p gui conformance` — the app launches on the mock and the conformance suite passes on both engines (REQ-GUI-165, REQ-GUI-166, closed by TASK-M6-24, re-run here).

## Notes
- **What to try.** The PR description has a "What to try" section: Run…, Profiler…, Export… from the top bar, the Chart
  builder from the Manifold view, an error raised to open the Console, IC Inspector… from the Trajectory panel, on
  `cargo run -p gui --features mock`.
- REQ-GUI-126 moved here from TASK-M8-27 (R-390); TASK-M8-27 depends on this task, re-runs its acceptance on the real
  engine's telemetry stream and keeps REQ-TOOL-108. Every other window requirement stays with its task, each of which
  depends on this one: the Run window's contract fields and behaviour (TASK-M8-24, TASK-M6-22), the Chart builder
  (TASK-M8-18), Export & share (TASK-M8-32), Display (TASK-M8-23, TASK-M7-25), the Profiler (TASK-M8-28, and its Arbiter
  tab, TASK-M6-22 under R-152), and the Inspector (TASK-M8-14 onward).
- Values follow R-68: the Run window's defaults and names are the contract's, not the artboard's (§G13).
- Sources and silences as TASK-M6-24's Notes give them (R-390).
